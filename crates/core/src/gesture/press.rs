//! The life of one press, from pointer down to pointer up.

use std::time::Duration;

use super::{Direction, GestureConfig, PointerId, PressKind, Trigger};
use crate::layout::KeyId;

#[derive(Debug, Clone)]
pub(super) struct Press {
    pub pointer: PointerId,
    pub key: KeyId,
    kind: PressKind,
    origin_x: f32,
    started: Duration,
    state: State,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    /// The tap has fired. Repeats at `next_repeat`, if set.
    Fired { next_repeat: Option<Duration> },
    /// The tap waits for release, unless a hold or a swipe takes over.
    Waiting,
    /// Dragging; `steps` have been sent so far, positive to the right.
    Swiping { steps: i32 },
    /// Nothing more happens until the pointer lifts.
    Settled,
}

impl Press {
    pub fn start(
        pointer: PointerId,
        key: KeyId,
        kind: PressKind,
        x: f32,
        now: Duration,
        config: &GestureConfig,
        out: &mut Vec<Trigger>,
    ) -> Self {
        let state = if kind.defers_tap() {
            State::Waiting
        } else {
            out.push(Trigger::Tap(key));
            State::Fired {
                next_repeat: kind.repeat.then(|| now + config.repeat_delay),
            }
        };
        Self {
            pointer,
            key,
            kind,
            origin_x: x,
            started: now,
            state,
        }
    }

    /// Another key went down: fire a waiting tap now so output keeps the order
    /// keys were pressed in, and stop repeating.
    pub fn interrupt(&mut self, out: &mut Vec<Trigger>) {
        match self.state {
            State::Waiting => {
                out.push(Trigger::Tap(self.key));
                self.state = State::Settled;
            }
            State::Fired {
                next_repeat: Some(_),
            } => {
                self.state = State::Fired { next_repeat: None };
            }
            State::Fired { next_repeat: None } | State::Swiping { .. } | State::Settled => {}
        }
    }

    pub fn moved(&mut self, x: f32, config: &GestureConfig, out: &mut Vec<Trigger>) {
        let dx = x - self.origin_x;
        if self.state == State::Waiting && self.kind.swipe && dx.abs() >= config.swipe_slop {
            self.state = State::Swiping { steps: 0 };
        }
        let State::Swiping { steps } = &mut self.state else {
            return;
        };
        // Truncation toward zero keeps the start position stable: small wobbles
        // around it never send a step.
        let target = (dx / config.swipe_step).trunc() as i32;
        let direction = if target > *steps {
            Direction::Right
        } else {
            Direction::Left
        };
        let count = target.abs_diff(*steps);
        *steps = target;
        let key = self.key;
        out.extend((0..count).map(|_| Trigger::Swipe { key, direction }));
    }

    pub fn tick(&mut self, now: Duration, config: &GestureConfig, out: &mut Vec<Trigger>) {
        match self.state {
            State::Waiting if self.kind.hold && now >= self.started + config.hold_delay => {
                out.push(Trigger::Hold(self.key));
                self.state = State::Settled;
            }
            State::Fired {
                next_repeat: Some(at),
            } if now >= at => {
                out.push(Trigger::Repeat(self.key));
                // Schedule from now rather than from `at`, so a late timer does
                // not cause a burst of catch-up repeats.
                self.state = State::Fired {
                    next_repeat: Some(now + config.repeat_interval),
                };
            }
            _ => {}
        }
    }

    pub fn release(self, out: &mut Vec<Trigger>) {
        match self.state {
            State::Waiting => out.push(Trigger::Tap(self.key)),
            State::Swiping { .. } => out.push(Trigger::SwipeEnd(self.key)),
            State::Fired { .. } | State::Settled => {}
        }
    }

    pub fn cancel(self, out: &mut Vec<Trigger>) {
        if let State::Swiping { .. } = self.state {
            out.push(Trigger::SwipeEnd(self.key));
        }
    }

    /// When this press next needs a tick.
    pub fn deadline(&self, config: &GestureConfig) -> Option<Duration> {
        match self.state {
            State::Waiting if self.kind.hold => Some(self.started + config.hold_delay),
            State::Fired { next_repeat } => next_repeat,
            _ => None,
        }
    }
}
