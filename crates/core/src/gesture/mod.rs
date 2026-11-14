//! Turns pointer presses, movement and time into key triggers: taps, long-press
//! holds, key repeat and swipes.
//!
//! [`Gestures`] never reads a clock. Hosts pass the current time with each event
//! and call [`Gestures::tick`] when [`Gestures::next_deadline`] is reached. That
//! keeps the timing rules deterministic, testable, and the same on every
//! platform: the Dioxus app and a future native Android host share them.
//!
//! The rules, per key:
//!
//! - A plain key fires its tap as soon as it is pressed.
//! - A repeating key (Backspace, arrows) fires on press, then again every
//!   [`GestureConfig::repeat_interval`] once held for
//!   [`GestureConfig::repeat_delay`].
//! - A key with a hold action or a swipe waits: releasing it before
//!   [`GestureConfig::hold_delay`] is a tap, holding it past the delay fires the
//!   hold, and dragging it sideways past [`GestureConfig::swipe_slop`] starts a
//!   swipe that fires one step per [`GestureConfig::swipe_step`].
//! - Pressing another key settles any waiting tap first and stops repeats, so
//!   fast typing with overlapping fingers keeps its order.

mod press;

use std::time::Duration;

use crate::{
    action::NamedKey,
    layout::{Key, KeyId},
};
use press::Press;

/// Identifies one finger, stylus or mouse, as the platform numbers them.
pub type PointerId = i64;

/// Timing and distance thresholds for gestures. Distances are in the same units
/// as the positions the host passes (CSS pixels in the app).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GestureConfig {
    /// How long a key with a hold action must be held for the hold to fire.
    pub hold_delay: Duration,
    /// How long a repeating key must be held before it starts repeating.
    pub repeat_delay: Duration,
    /// Time between repeats.
    pub repeat_interval: Duration,
    /// How far a finger must move sideways before a press becomes a swipe.
    pub swipe_slop: f32,
    /// Distance per swipe step.
    pub swipe_step: f32,
}

impl Default for GestureConfig {
    fn default() -> Self {
        Self {
            hold_delay: Duration::from_millis(300),
            repeat_delay: Duration::from_millis(400),
            repeat_interval: Duration::from_millis(33),
            swipe_slop: 10.0,
            swipe_step: 12.0,
        }
    }
}

/// Which gestures a key takes part in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PressKind {
    pub repeat: bool,
    pub hold: bool,
    pub swipe: bool,
}

impl PressKind {
    pub fn of(key: &Key) -> Self {
        Self {
            repeat: key.repeat,
            hold: key.hold.is_some(),
            swipe: key.swipe.is_some(),
        }
    }

    /// Whether the tap must wait for release to rule out a hold or a swipe.
    fn defers_tap(self) -> bool {
        self.hold || self.swipe
    }
}

/// A swipe direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    /// The cursor key that moves this way.
    pub fn arrow(self) -> NamedKey {
        match self {
            Self::Left => NamedKey::Left,
            Self::Right => NamedKey::Right,
        }
    }
}

/// Something a key should do now. [`crate::Keyboard::trigger`] turns it into
/// output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trigger {
    Tap(KeyId),
    Hold(KeyId),
    Repeat(KeyId),
    /// One step of a swipe.
    Swipe {
        key: KeyId,
        direction: Direction,
    },
    /// The finger lifted after swiping.
    SwipeEnd(KeyId),
}

impl Trigger {
    pub fn key(self) -> KeyId {
        match self {
            Self::Tap(key)
            | Self::Hold(key)
            | Self::Repeat(key)
            | Self::Swipe { key, .. }
            | Self::SwipeEnd(key) => key,
        }
    }
}

/// Tracks every key that is currently pressed.
#[derive(Debug, Clone, Default)]
pub struct Gestures {
    config: GestureConfig,
    /// In the order the keys went down.
    presses: Vec<Press>,
}

impl Gestures {
    pub fn new(config: GestureConfig) -> Self {
        Self {
            config,
            presses: Vec::new(),
        }
    }

    pub fn config(&self) -> &GestureConfig {
        &self.config
    }

    /// Changes the thresholds. Presses in progress keep their start time, so a
    /// shorter hold delay can make a waiting hold due at the next tick.
    pub fn set_config(&mut self, config: GestureConfig) {
        self.config = config;
    }

    /// A pointer went down on `key` at horizontal position `x`.
    pub fn press(
        &mut self,
        pointer: PointerId,
        key: KeyId,
        kind: PressKind,
        x: f32,
        now: Duration,
    ) -> Vec<Trigger> {
        let mut out = self.tick(now);
        // A second "down" without an "up" means the platform lost the release.
        if let Some(stale) = self.take(pointer) {
            stale.release(&mut out);
        }
        for press in &mut self.presses {
            press.interrupt(&mut out);
        }
        self.presses.push(Press::start(
            pointer,
            key,
            kind,
            x,
            now,
            &self.config,
            &mut out,
        ));
        out
    }

    /// A pointer moved to horizontal position `x`.
    pub fn moved(&mut self, pointer: PointerId, x: f32) -> Vec<Trigger> {
        let mut out = Vec::new();
        if let Some(press) = self.presses.iter_mut().find(|p| p.pointer == pointer) {
            press.moved(x, &self.config, &mut out);
        }
        out
    }

    /// A pointer lifted.
    pub fn release(&mut self, pointer: PointerId, now: Duration) -> Vec<Trigger> {
        // A hold that is due wins over a tap, even if the host's timer is late.
        let mut out = self.tick(now);
        if let Some(press) = self.take(pointer) {
            press.release(&mut out);
        }
        out
    }

    /// The platform cancelled a pointer (for example, the system took over the
    /// gesture). A waiting tap is dropped rather than fired.
    pub fn cancel(&mut self, pointer: PointerId) -> Vec<Trigger> {
        let mut out = Vec::new();
        if let Some(press) = self.take(pointer) {
            press.cancel(&mut out);
        }
        out
    }

    /// Cancels every press, for example when the layout changes under them.
    pub fn cancel_all(&mut self) -> Vec<Trigger> {
        let mut out = Vec::new();
        for press in self.presses.drain(..) {
            press.cancel(&mut out);
        }
        out
    }

    /// Fires every hold and repeat that is due at `now`.
    pub fn tick(&mut self, now: Duration) -> Vec<Trigger> {
        let mut out = Vec::new();
        for press in &mut self.presses {
            press.tick(now, &self.config, &mut out);
        }
        out
    }

    /// When [`Gestures::tick`] next has something to do, if ever.
    pub fn next_deadline(&self) -> Option<Duration> {
        self.presses
            .iter()
            .filter_map(|p| p.deadline(&self.config))
            .min()
    }

    /// Whether this pointer is pressing a key.
    pub fn is_tracking(&self, pointer: PointerId) -> bool {
        self.presses.iter().any(|p| p.pointer == pointer)
    }

    /// The keys being pressed, in the order they went down.
    pub fn pressed_keys(&self) -> impl Iterator<Item = KeyId> + '_ {
        self.presses.iter().map(|p| p.key)
    }

    fn take(&mut self, pointer: PointerId) -> Option<Press> {
        let index = self.presses.iter().position(|p| p.pointer == pointer)?;
        Some(self.presses.remove(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: PressKind = PressKind {
        repeat: false,
        hold: false,
        swipe: false,
    };
    const REPEAT: PressKind = PressKind {
        repeat: true,
        ..PLAIN
    };
    const HOLD: PressKind = PressKind {
        hold: true,
        ..PLAIN
    };
    const SWIPE: PressKind = PressKind {
        swipe: true,
        ..PLAIN
    };

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn key(col: usize) -> KeyId {
        KeyId {
            layer: 0,
            row: 0,
            col,
        }
    }

    fn swipe(col: usize, direction: Direction) -> Trigger {
        Trigger::Swipe {
            key: key(col),
            direction,
        }
    }

    #[test]
    fn plain_keys_fire_on_press_only() {
        let mut g = Gestures::default();
        assert_eq!(
            g.press(1, key(0), PLAIN, 0.0, ms(0)),
            [Trigger::Tap(key(0))]
        );
        assert_eq!(g.next_deadline(), None);
        assert_eq!(g.tick(ms(5_000)), []);
        assert_eq!(g.release(1, ms(5_000)), []);
        assert!(!g.is_tracking(1));
    }

    #[test]
    fn repeating_keys_repeat_after_a_delay() {
        let mut g = Gestures::default();
        assert_eq!(
            g.press(1, key(0), REPEAT, 0.0, ms(0)),
            [Trigger::Tap(key(0))]
        );
        assert_eq!(g.next_deadline(), Some(ms(400)));
        assert_eq!(g.tick(ms(399)), []);
        assert_eq!(g.tick(ms(400)), [Trigger::Repeat(key(0))]);
        assert_eq!(g.next_deadline(), Some(ms(433)));
        // A late timer does not cause a burst of catch-up repeats.
        assert_eq!(g.tick(ms(1_000)), [Trigger::Repeat(key(0))]);
        assert_eq!(g.next_deadline(), Some(ms(1_033)));
        assert_eq!(g.release(1, ms(1_010)), []);
        assert_eq!(g.next_deadline(), None);
    }

    #[test]
    fn hold_keys_tap_on_quick_release_and_hold_when_held() {
        let mut g = Gestures::default();
        assert_eq!(g.press(1, key(0), HOLD, 0.0, ms(0)), []);
        assert_eq!(g.release(1, ms(100)), [Trigger::Tap(key(0))]);

        assert_eq!(g.press(1, key(0), HOLD, 0.0, ms(1_000)), []);
        assert_eq!(g.next_deadline(), Some(ms(1_300)));
        assert_eq!(g.tick(ms(1_300)), [Trigger::Hold(key(0))]);
        assert_eq!(g.next_deadline(), None);
        assert_eq!(g.release(1, ms(2_000)), []);
    }

    #[test]
    fn a_due_hold_wins_over_a_late_release() {
        let mut g = Gestures::default();
        g.press(1, key(0), HOLD, 0.0, ms(0));
        assert_eq!(g.release(1, ms(350)), [Trigger::Hold(key(0))]);
    }

    #[test]
    fn swipes_step_both_ways_and_end_on_release() {
        let mut g = Gestures::default();
        assert_eq!(g.press(1, key(3), SWIPE, 100.0, ms(0)), []);
        assert_eq!(g.moved(1, 105.0), [], "within the slop");
        assert_eq!(
            g.moved(1, 125.0),
            [swipe(3, Direction::Right), swipe(3, Direction::Right)]
        );
        assert_eq!(
            g.moved(1, 99.0),
            [swipe(3, Direction::Left), swipe(3, Direction::Left)]
        );
        assert_eq!(g.moved(1, 87.0), [swipe(3, Direction::Left)]);
        assert_eq!(g.release(1, ms(500)), [Trigger::SwipeEnd(key(3))]);
    }

    #[test]
    fn swipe_keys_without_movement_are_taps() {
        let mut g = Gestures::default();
        g.press(1, key(3), SWIPE, 100.0, ms(0));
        assert_eq!(g.moved(1, 104.0), []);
        assert_eq!(g.next_deadline(), None, "no hold to wait for");
        assert_eq!(g.release(1, ms(2_000)), [Trigger::Tap(key(3))]);
    }

    #[test]
    fn swiping_rules_out_the_hold() {
        let mut g = Gestures::default();
        let both = PressKind {
            hold: true,
            swipe: true,
            ..PLAIN
        };
        g.press(1, key(0), both, 0.0, ms(0));
        assert_eq!(
            g.moved(1, 30.0),
            [swipe(0, Direction::Right), swipe(0, Direction::Right)]
        );
        assert_eq!(g.next_deadline(), None);
        assert_eq!(g.tick(ms(1_000)), []);
        assert_eq!(g.release(1, ms(1_000)), [Trigger::SwipeEnd(key(0))]);
    }

    #[test]
    fn another_press_settles_waiting_taps_and_stops_repeats() {
        let mut g = Gestures::default();
        g.press(1, key(0), HOLD, 0.0, ms(0));
        assert_eq!(
            g.press(2, key(1), PLAIN, 0.0, ms(50)),
            [Trigger::Tap(key(0)), Trigger::Tap(key(1))]
        );
        assert_eq!(g.release(1, ms(400)), [], "the tap already fired");

        g.press(3, key(2), REPEAT, 0.0, ms(1_000));
        g.press(4, key(1), PLAIN, 0.0, ms(1_100));
        assert_eq!(g.next_deadline(), None);
        assert_eq!(
            g.pressed_keys().collect::<Vec<_>>(),
            [key(1), key(2), key(1)]
        );
    }

    #[test]
    fn cancel_drops_a_waiting_tap_but_ends_a_swipe() {
        let mut g = Gestures::default();
        g.press(1, key(0), HOLD, 0.0, ms(0));
        assert_eq!(g.cancel(1), []);
        g.press(1, key(0), SWIPE, 0.0, ms(0));
        g.moved(1, 20.0);
        assert_eq!(g.cancel_all(), [Trigger::SwipeEnd(key(0))]);
        assert!(!g.is_tracking(1));
    }

    #[test]
    fn a_repeated_down_releases_the_stale_press() {
        let mut g = Gestures::default();
        g.press(1, key(0), HOLD, 0.0, ms(0));
        assert_eq!(
            g.press(1, key(1), PLAIN, 0.0, ms(10)),
            [Trigger::Tap(key(0)), Trigger::Tap(key(1))]
        );
        assert_eq!(g.pressed_keys().collect::<Vec<_>>(), [key(1)]);
    }

    #[test]
    fn moves_of_unknown_pointers_are_ignored() {
        let mut g = Gestures::default();
        assert_eq!(g.moved(7, 500.0), []);
        assert_eq!(g.release(7, ms(0)), []);
        assert_eq!(g.cancel(7), []);
    }
}
