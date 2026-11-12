//! What a selected value turns out to be, in the menu a right click opens.
//!
//! The converters are in `tracewing-convert`; this is the clock they are
//! given, and the menu items they become.

use crate::select::{Act, Item};
use chrono::Offset as _;
use gpui::{App, ClipboardItem, Window};
use std::rc::Rc;
use tracewing_convert::{Input, Reading};
use tracewing_i18n::tr;

/// How many readings a menu offers. Past a few it is a list to read, not a
/// menu to choose from, and what is further down is what was least likely.
pub const MOST: usize = 4;

/// Now, and the reader's offset from UTC, as the converters want them.
fn here() -> (i64, i32) {
    let now = chrono::Local::now();
    (now.timestamp(), now.offset().fix().local_minus_utc())
}

/// Every reading of `text`, best first, as the field named `field` holds it.
pub fn of(text: &str, field: Option<&str>) -> Vec<Reading> {
    let (now, offset) = here();
    tracewing_convert::readings(&Input::new(text, now, offset).under(field))
}

/// The readings as menu items: what each says, and a copy of it on a click.
/// `close` shuts the menu the item was chosen from.
pub fn items(
    readings: &[Reading],
    close: impl Fn(&mut Window, &mut App) + Clone + 'static,
) -> Vec<Item> {
    readings
        .iter()
        .take(MOST)
        .map(|reading| {
            let text = reading.text();
            let close = close.clone();
            Item::new(
                reading.id,
                reading.summary(),
                Some(Rc::new(move |window: &mut Window, cx: &mut App| {
                    cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                    close(window, cx);
                }) as Act),
            )
        })
        .collect()
}

/// What the menu says where a reading was asked for and none was found.
pub fn nothing() -> Item {
    Item::new("no-reading", tr!("Nothing to read in this"), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_is_read_against_this_machines_clock() {
        let found = of("1794492800", Some("created_at"));
        assert_eq!(found[0].id, "unix-s");
        // The offset is this machine's, so only the shape is sure here.
        assert!(found[0].text().starts_with("20"));
    }

    #[test]
    fn the_menu_takes_the_best_few() {
        let found = of("137", None);
        assert!(found.len() > 1);
        let items = items(&found, |_, _| {});
        assert!(items.len() <= MOST);
        assert!(items.iter().all(|item| item.act.is_some()));
    }

    #[test]
    fn nothing_readable_is_no_items() {
        assert!(of("just some words", None).is_empty());
    }
}
