//! What a selected value turns out to be, in the menu a right click opens and
//! beside the value itself.
//!
//! The converters are in `tracewing-convert`; this is the clock they are
//! given, and the menu items they become.

use crate::select::{Act, Item};
use chrono::Offset as _;
use gpui::{App, ClipboardItem, Window};
use std::rc::Rc;
use tracewing_convert::{Input, Reading};
use tracewing_i18n::tr;

/// How many readings a menu offers at once. Past a few it is a list to read,
/// not a menu to choose from.
pub const MOST: usize = 4;

/// How many more it offers under `Read as`, for when the first few were not
/// what the value is.
pub const MOST_MORE: usize = 6;

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

/// The readings as menu items: what each says, and `pick` with that reading
/// when it is chosen, which is what draws it beside the value. The ones the
/// registry ranked lower go under `Read as`.
pub fn items(
    readings: &[Reading],
    pick: impl Fn(Reading, &mut Window, &mut App) + Clone + 'static,
) -> Vec<Item> {
    let item = |reading: &Reading| {
        let chosen = reading.clone();
        let pick = pick.clone();
        Item::new(
            reading.id,
            reading.summary(),
            Some(
                Rc::new(move |window: &mut Window, cx: &mut App| pick(chosen.clone(), window, cx))
                    as Act,
            ),
        )
    };
    let mut items: Vec<Item> = readings.iter().take(MOST).map(item).collect();
    let mut more = readings.iter().skip(MOST).take(MOST_MORE).peekable();
    if more.peek().is_some() {
        items.push(Item::new("read-as", tr!("Read as"), None));
        items.extend(more.map(item));
    }
    items
}

/// `Copy reading`, which copies the best one: the same text a chosen reading
/// draws, in full rather than cut to a line.
pub fn copy_item(
    readings: &[Reading],
    close: impl Fn(&mut Window, &mut App) + 'static,
) -> Option<Item> {
    let text = readings.first()?.text();
    Some(Item::new(
        "copy-reading",
        tr!("Copy reading"),
        Some(Rc::new(move |window: &mut Window, cx: &mut App| {
            cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
            close(window, cx);
        }) as Act),
    ))
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
    fn the_menu_takes_the_best_few_and_offers_the_rest() {
        let found = of("137", None);
        assert!(found.len() > 1);
        let items = items(&found, |_, _, _| {});
        assert!(items.iter().all(|item| item.label.contains('(')));
        assert!(items.len() <= MOST + 1 + MOST_MORE);
    }

    #[test]
    fn what_was_ranked_lower_goes_under_read_as() {
        // More readings than the menu shows at once: a snowflake is also a
        // number, a size, a mask, and base64 of something.
        let many: Vec<Reading> = of("1794492800000", None);
        let items = items(&many, |_, _, _| {});
        if many.len() > MOST {
            let heading = items.iter().find(|item| item.id == "read-as").unwrap();
            assert!(heading.act.is_none(), "a heading is not chosen");
            assert_eq!(items.len(), many.len().min(MOST + 1 + MOST_MORE) + 1);
        }
    }

    #[test]
    fn nothing_readable_is_no_items() {
        assert!(of("just some words", None).is_empty());
        assert!(items(&[], |_, _, _| {}).is_empty());
        assert!(copy_item(&[], |_, _| {}).is_none());
    }
}
