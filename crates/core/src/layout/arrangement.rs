//! Ordering and switching named items on and off: the sections of a layout, or
//! the entries of the profile menu.

use std::collections::BTreeMap;

/// An order and on/off choices for a list of named items.
///
/// Both are overrides of the items' defaults. Items listed in `order` take the
/// places the listed items have by default, in the listed order; items not listed
/// keep their default place. An item missing from `enabled` keeps its default
/// state. So a layout can gain sections, or the menu new profiles, without
/// breaking arrangements saved earlier, and `order = ["letters", "brackets"]`
/// swaps just those two.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Arrangement {
    /// Names in the order to show them.
    pub order: Vec<String>,
    /// Items switched on (`true`) or off (`false`) against their default.
    pub enabled: BTreeMap<String, bool>,
}

impl Arrangement {
    /// Arranges `defaults` (names in their default order) and returns their
    /// positions in `defaults`, in display order. Unknown and repeated names in
    /// `order` are ignored.
    pub fn arrange(&self, defaults: &[&str]) -> Vec<usize> {
        let mut listed: Vec<usize> = Vec::with_capacity(self.order.len());
        for name in &self.order {
            if let Some(index) = defaults.iter().position(|d| d == name)
                && !listed.contains(&index)
            {
                listed.push(index);
            }
        }
        let mut places = listed.clone();
        places.sort_unstable();
        let mut arranged: Vec<usize> = (0..defaults.len()).collect();
        for (place, index) in places.into_iter().zip(listed) {
            arranged[place] = index;
        }
        arranged
    }

    /// Whether `name` is on, given its default.
    pub fn is_enabled(&self, name: &str, default: bool) -> bool {
        self.enabled.get(name).copied().unwrap_or(default)
    }

    /// Switches an item on or off. Only differences from the default are kept.
    pub fn set_enabled(&mut self, name: &str, on: bool, default: bool) {
        if on == default {
            self.enabled.remove(name);
        } else {
            self.enabled.insert(name.to_owned(), on);
        }
    }

    /// Moves `name` by `by` places (negative moves it up), within `defaults`.
    /// The whole order is stored, or none of it if the result is the default.
    pub fn move_item(&mut self, defaults: &[&str], name: &str, by: isize) {
        let mut order: Vec<&str> = self
            .arrange(defaults)
            .into_iter()
            .map(|i| defaults[i])
            .collect();
        let Some(from) = order.iter().position(|n| *n == name) else {
            return;
        };
        let to = from.saturating_add_signed(by).min(order.len() - 1);
        let item = order.remove(from);
        order.insert(to, item);
        self.order = if order == defaults {
            Vec::new()
        } else {
            order.into_iter().map(str::to_owned).collect()
        };
    }

    /// Whether this arrangement changes nothing.
    pub fn is_default(&self) -> bool {
        self.order.is_empty() && self.enabled.is_empty()
    }

    /// Names this arrangement mentions that are not in `known`.
    pub fn unknown_names<'a>(&'a self, known: &[&str]) -> Vec<&'a str> {
        let mut unknown: Vec<&str> = self
            .order
            .iter()
            .chain(self.enabled.keys())
            .map(String::as_str)
            .filter(|name| !known.contains(name))
            .collect();
        unknown.sort_unstable();
        unknown.dedup();
        unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEMS: [&str; 4] = ["a", "b", "c", "d"];

    fn arranged(arrangement: &Arrangement) -> Vec<&'static str> {
        arrangement
            .arrange(&ITEMS)
            .into_iter()
            .map(|i| ITEMS[i])
            .collect()
    }

    fn order(names: &[&str]) -> Arrangement {
        Arrangement {
            order: names.iter().map(|n| (*n).to_owned()).collect(),
            ..Arrangement::default()
        }
    }

    #[test]
    fn the_default_is_the_declared_order() {
        assert_eq!(arranged(&Arrangement::default()), ITEMS);
    }

    #[test]
    fn listed_items_trade_places_and_others_stay_put() {
        assert_eq!(
            arranged(&order(&["d", "c", "b", "a"])),
            ["d", "c", "b", "a"]
        );
        assert_eq!(arranged(&order(&["c", "a"])), ["c", "b", "a", "d"]);
        assert_eq!(arranged(&order(&["d", "b"])), ["a", "d", "c", "b"]);
        // Unknown and repeated names are ignored; one name alone stays put.
        assert_eq!(
            arranged(&order(&["x", "b", "b", "a"])),
            ["b", "a", "c", "d"]
        );
        assert_eq!(arranged(&order(&["c"])), ITEMS);
    }

    #[test]
    fn enabled_overrides_only_store_differences() {
        let mut arrangement = Arrangement::default();
        arrangement.set_enabled("a", false, true);
        arrangement.set_enabled("b", true, false);
        assert!(!arrangement.is_enabled("a", true));
        assert!(arrangement.is_enabled("b", false));
        assert!(arrangement.is_enabled("c", true));
        arrangement.set_enabled("a", true, true);
        arrangement.set_enabled("b", false, false);
        assert!(arrangement.is_default());
    }

    #[test]
    fn moving_items() {
        let mut arrangement = Arrangement::default();
        arrangement.move_item(&ITEMS, "c", -1);
        assert_eq!(arranged(&arrangement), ["a", "c", "b", "d"]);
        arrangement.move_item(&ITEMS, "c", -10);
        assert_eq!(arranged(&arrangement), ["c", "a", "b", "d"]);
        arrangement.move_item(&ITEMS, "c", 10);
        assert_eq!(arranged(&arrangement), ["a", "b", "d", "c"]);
        arrangement.move_item(&ITEMS, "c", -1);
        assert!(arrangement.is_default(), "back to the default order");
        arrangement.move_item(&ITEMS, "missing", 1);
        assert!(arrangement.is_default());
    }

    #[test]
    fn reports_unknown_names_once() {
        let mut arrangement = order(&["a", "x", "y"]);
        arrangement.set_enabled("x", true, false);
        assert_eq!(arrangement.unknown_names(&ITEMS), ["x", "y"]);
    }
}
