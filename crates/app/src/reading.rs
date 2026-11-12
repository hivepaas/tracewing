//! What a selected value turns out to be, in the menu a right click opens and
//! beside the value itself.
//!
//! The converters are in `tracewing-convert`; this is the clock they are
//! given, and the menu items they become.

use crate::select::{Act, Item};
use chrono::Offset as _;
use gpui::{App, ClipboardItem, Window};
use std::rc::Rc;
use tracewing_convert::{Input, Reading, Shown};
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

/// The one reading a remembered rule asks for, if the value still reads that
/// way.
pub fn read_as(id: &str, text: &str, field: Option<&str>) -> Option<Reading> {
    if text.len() > tracewing_convert::MAX_INPUT {
        return None;
    }
    let (now, offset) = here();
    tracewing_convert::read_as(id, &Input::new(text, now, offset).under(field)).map(small)
}

/// The field and the value a laid-out line holds: the `created_at` and the
/// `1763000000` of `  "created_at": 1763000000,`.
pub fn field_and_value(line: &str) -> Option<(&str, &str)> {
    let open = line.find('"')?;
    let rest = &line[open + 1..];
    let close = rest.find('"')?;
    let field = &rest[..close];
    let after = rest[close + 1..].strip_prefix(':')?.trim_start();
    let value = after
        .trim_end()
        .trim_end_matches(',')
        .trim_end()
        .trim_matches('"');
    (!field.is_empty() && !value.is_empty()).then_some((field, value))
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

/// A reading small enough to keep beside a field. What was unpacked is not
/// kept: one line says what it holds until there is a pane to open it in, and
/// a reader who picks six of them should not be holding six bodies in memory.
pub fn small(reading: Reading) -> Reading {
    match reading.shown {
        Shown::Bytes { .. } => Reading {
            shown: Shown::Line(reading.beside()),
            ..reading
        },
        _ => reading,
    }
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
    fn what_was_unpacked_is_kept_as_the_line_that_says_so() {
        let packed = of("eyJhIjoxfQ", None);
        let json = packed.iter().find(|r| r.id == "base64").unwrap().clone();
        assert!(matches!(json.shown, Shown::Bytes { .. }));
        let kept = small(json.clone());
        assert!(matches!(kept.shown, Shown::Line(_)));
        assert_eq!(kept.beside(), json.beside());
        assert_eq!(kept.id, "base64");
    }

    #[test]
    fn a_line_gives_up_its_field_and_its_value() {
        assert_eq!(
            field_and_value(r#"  "created_at": 1763000000,"#),
            Some(("created_at", "1763000000"))
        );
        assert_eq!(
            field_and_value(r#"  "name": "Chi Pham""#),
            Some(("name", "Chi Pham"))
        );
        assert_eq!(field_and_value("  {"), None);
        assert_eq!(field_and_value(r#"  "orders": ["#), Some(("orders", "[")));
        assert_eq!(
            field_and_value(r#"  "note": null,"#),
            Some(("note", "null"))
        );
    }

    #[test]
    fn a_remembered_reading_is_found_for_a_new_value() {
        let found = read_as("unix-s", "1794492800", Some("created_at")).unwrap();
        assert_eq!(found.id, "unix-s");
        assert!(read_as("unix-s", "not a time", Some("created_at")).is_none());
    }

    #[test]
    fn nothing_readable_is_no_items() {
        assert!(of("just some words", None).is_empty());
        assert!(items(&[], |_, _, _| {}).is_empty());
        assert!(copy_item(&[], |_, _| {}).is_none());
    }
}
