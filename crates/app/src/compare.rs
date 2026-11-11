//! Two flows side by side: what differs between them. Headers and the query
//! are matched by name; bodies line by line, JSON written the same way on
//! both sides first, so that only what it says can differ.

use similar::{Algorithm, DiffOp};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use tracewing_store::{binjson, prost_reflect::MessageDescriptor};

/// The most of a body compared.
pub const BODY_MAX: usize = 4 << 20;
/// Lines shown either side of a change; the same lines past them are folded.
pub const CONTEXT: usize = 3;
/// How long the line diff may take before it settles for a rougher answer.
const DIFF_TIME: Duration = Duration::from_secs(2);
const HEX_LINE: usize = 16;

/// How the two sides of something differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Same,
    /// Only the second flow has it.
    Added,
    /// Only the first flow has it.
    Removed,
    Changed,
}

/// A name with its value in either flow: a header, or a field of the query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    pub name: String,
    pub a: Option<String>,
    pub b: Option<String>,
}

impl Pair {
    pub fn change(&self) -> Change {
        match (&self.a, &self.b) {
            (Some(a), Some(b)) if a == b => Change::Same,
            (Some(_), Some(_)) => Change::Changed,
            (Some(_), None) => Change::Removed,
            (None, Some(_)) => Change::Added,
            (None, None) => Change::Same,
        }
    }
}

/// Values by name, in the order the names first came. A name given more than
/// once has its values a line each.
fn grouped(list: &[(String, String)], any_case: bool) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = Vec::new();
    for (name, value) in list {
        let key = if any_case {
            name.to_ascii_lowercase()
        } else {
            name.clone()
        };
        match out.iter_mut().find(|(k, _, _)| *k == key) {
            Some((_, _, values)) => {
                values.push('\n');
                values.push_str(value);
            }
            None => out.push((key, name.clone(), value.clone())),
        }
    }
    out
}

/// The names of `a` matched with those of `b`, ignoring case if `any_case`:
/// `a`'s in its order, then those only `b` has.
pub fn pairs(a: &[(String, String)], b: &[(String, String)], any_case: bool) -> Vec<Pair> {
    let (a, mut b) = (grouped(a, any_case), grouped(b, any_case));
    let mut out = Vec::with_capacity(a.len() + b.len());
    for (key, name, value) in a {
        let other = b
            .iter()
            .position(|(k, _, _)| *k == key)
            .map(|at| b.remove(at).2);
        out.push(Pair {
            name,
            a: Some(value),
            b: other,
        });
    }
    out.extend(b.into_iter().map(|(_, name, value)| Pair {
        name,
        a: None,
        b: Some(value),
    }));
    out
}

/// A URL's query, each name and value decoded.
pub fn query_of(url: &str) -> Vec<(String, String)> {
    reqwest::Url::parse(url)
        .map(|url| {
            url.query_pairs()
                .map(|(name, value)| (name.into_owned(), value.into_owned()))
                .collect()
        })
        .unwrap_or_default()
}

/// A body as lines to compare, and what it was read as.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BodyText {
    /// `JSON`, `MessagePack`, `Text`, `Bytes`…; empty for no body.
    pub kind: String,
    pub lines: Vec<String>,
    /// Only its first [`BODY_MAX`] bytes were compared.
    pub cut: bool,
}

/// `data` as lines: JSON, and what MessagePack, CBOR and Protobuf read as,
/// written with its keys in order and a value a line; other text as it is;
/// anything else as hex.
pub fn body_text(
    data: &[u8],
    content_type: Option<&str>,
    message: Option<&MessageDescriptor>,
    cut: bool,
) -> BodyText {
    let text = |kind: &str, text: &str| BodyText {
        kind: kind.to_owned(),
        lines: text.lines().map(str::to_owned).collect(),
        cut,
    };
    if data.is_empty() {
        return BodyText::default();
    }
    if let Some(format) = binjson::format_of(content_type)
        && let Some(json) = binjson::to_json_as(data, format, message)
    {
        let pretty = serde_json::from_str(&json)
            .map(|value| pretty_json(&value))
            .unwrap_or(json);
        return text(format.name(), &pretty);
    }
    let starts_like_json = data
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| matches!(b, b'{' | b'['));
    if starts_like_json && let Ok(value) = serde_json::from_slice(data) {
        return text("JSON", &pretty_json(&value));
    }
    // A body cut short may end inside a character.
    let utf8 = match std::str::from_utf8(data) {
        Ok(text) => Some(text),
        Err(e) if cut && e.error_len().is_none() => {
            std::str::from_utf8(&data[..e.valid_up_to()]).ok()
        }
        Err(_) => None,
    };
    if let Some(utf8) = utf8 {
        return text("Text", utf8);
    }
    BodyText {
        kind: "Bytes".to_owned(),
        lines: data.chunks(HEX_LINE).enumerate().map(hex_line).collect(),
        cut,
    }
}

fn hex_line((n, chunk): (usize, &[u8])) -> String {
    let mut line = format!("{:08x} ", n * HEX_LINE);
    for byte in chunk {
        line.push_str(&format!(" {byte:02x}"));
    }
    line.push_str(&"   ".repeat(HEX_LINE - chunk.len()));
    line.push_str("  ");
    line.extend(chunk.iter().map(|&b| {
        if b.is_ascii_graphic() || b == b' ' {
            b as char
        } else {
            '.'
        }
    }));
    line
}

/// JSON two spaces an indent, with each object's keys in order.
pub fn pretty_json(value: &serde_json::Value) -> String {
    let mut out = String::new();
    write_json(value, 0, &mut out);
    out
}

fn write_json(value: &serde_json::Value, depth: usize, out: &mut String) {
    use serde_json::Value;
    let indent = |out: &mut String, depth: usize| out.push_str(&"  ".repeat(depth));
    match value {
        Value::Object(map) if !map.is_empty() => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push_str("{\n");
            for (i, key) in keys.iter().enumerate() {
                indent(out, depth + 1);
                out.push_str(&Value::String((*key).clone()).to_string());
                out.push_str(": ");
                write_json(&map[*key], depth + 1, out);
                out.push_str(if i + 1 < keys.len() { ",\n" } else { "\n" });
            }
            indent(out, depth);
            out.push('}');
        }
        Value::Array(items) if !items.is_empty() => {
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                indent(out, depth + 1);
                write_json(item, depth + 1, out);
                out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
            }
            indent(out, depth);
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// A line of either body, or of both: their numbers from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line {
    pub a: Option<usize>,
    pub b: Option<usize>,
    pub change: Change,
}

/// The lines of `a` and `b` lined up: those the same beside each other, those
/// changed beside what they became, and the rest beside nothing.
pub fn diff_lines(a: &[String], b: &[String]) -> Vec<Line> {
    let deadline = Instant::now() + DIFF_TIME;
    let ops = similar::capture_diff_slices_deadline(Algorithm::Patience, a, b, Some(deadline));
    let mut out = Vec::with_capacity(a.len().max(b.len()));
    let only_a = |out: &mut Vec<Line>, from: usize, len: usize| {
        out.extend((from..from + len).map(|i| Line {
            a: Some(i),
            b: None,
            change: Change::Removed,
        }));
    };
    let only_b = |out: &mut Vec<Line>, from: usize, len: usize| {
        out.extend((from..from + len).map(|i| Line {
            a: None,
            b: Some(i),
            change: Change::Added,
        }));
    };
    for op in ops {
        match op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => out.extend((0..len).map(|i| Line {
                a: Some(old_index + i),
                b: Some(new_index + i),
                change: Change::Same,
            })),
            DiffOp::Delete {
                old_index, old_len, ..
            } => only_a(&mut out, old_index, old_len),
            DiffOp::Insert {
                new_index, new_len, ..
            } => only_b(&mut out, new_index, new_len),
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let both = old_len.min(new_len);
                out.extend((0..both).map(|i| Line {
                    a: Some(old_index + i),
                    b: Some(new_index + i),
                    change: Change::Changed,
                }));
                only_a(&mut out, old_index + both, old_len - both);
                only_b(&mut out, new_index + both, new_len - both);
            }
        }
    }
    out
}

/// What is drawn: a line, or lines the same in both, folded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Line(Line),
    /// `count` lines from `at` in the lines given, which `open` can unfold.
    Folded {
        at: usize,
        count: usize,
    },
}

/// The lines to draw: the same ones more than [`CONTEXT`] from a change
/// folded, unless their fold, named by where it starts, is in `open`.
pub fn rows(lines: &[Line], open: &HashSet<usize>) -> Vec<Row> {
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if lines[i].change != Change::Same {
            out.push(Row::Line(lines[i]));
            i += 1;
            continue;
        }
        let end = lines[i..]
            .iter()
            .position(|line| line.change != Change::Same)
            .map_or(lines.len(), |n| i + n);
        // Context after the change before, and before the change after.
        let keep_start = if i == 0 { 0 } else { CONTEXT };
        let keep_end = if end == lines.len() { 0 } else { CONTEXT };
        let folded = (end - i).saturating_sub(keep_start + keep_end);
        // A fold of a line or two hides less than it costs.
        if folded > 2 && !open.contains(&(i + keep_start)) {
            out.extend(lines[i..i + keep_start].iter().copied().map(Row::Line));
            out.push(Row::Folded {
                at: i + keep_start,
                count: folded,
            });
            out.extend(lines[end - keep_end..end].iter().copied().map(Row::Line));
        } else {
            out.extend(lines[i..end].iter().copied().map(Row::Line));
        }
        i = end;
    }
    out
}

/// How many lines differ: those changed, and those only one side has.
pub fn changed_count(lines: &[Line]) -> usize {
    lines.iter().filter(|l| l.change != Change::Same).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect()
    }

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_owned).collect()
    }

    #[test]
    fn headers_are_matched_by_name_whatever_its_case() {
        let a = list(&[
            ("Content-Type", "application/json"),
            ("set-cookie", "a=1"),
            ("Set-Cookie", "b=2"),
            ("x-old", "1"),
            ("date", "Mon"),
        ]);
        let b = list(&[
            ("date", "Tue"),
            ("content-type", "application/json"),
            ("set-cookie", "a=1\nb=2"),
            ("x-new", "2"),
        ]);
        let got: Vec<(String, Change)> = pairs(&a, &b, true)
            .iter()
            .map(|p| (p.name.clone(), p.change()))
            .collect();
        assert_eq!(
            got,
            [
                ("Content-Type".to_owned(), Change::Same),
                ("set-cookie".to_owned(), Change::Same),
                ("x-old".to_owned(), Change::Removed),
                ("date".to_owned(), Change::Changed),
                ("x-new".to_owned(), Change::Added),
            ]
        );
        // The query's names keep their case.
        assert_eq!(
            pairs(&list(&[("Q", "1")]), &list(&[("q", "1")]), false).len(),
            2
        );
    }

    #[test]
    fn the_query_is_decoded() {
        assert_eq!(
            query_of("https://x.vn/s?q=red+shoes&city=H%C3%A0%20N%E1%BB%99i&page"),
            list(&[("q", "red shoes"), ("city", "Hà Nội"), ("page", "")])
        );
        assert!(query_of("not a url").is_empty());
    }

    #[test]
    fn json_is_written_the_same_way_whatever_its_order() {
        let a = body_text(br#"{"b":1,"a":{"y":[1,2],"x":null}}"#, None, None, false);
        let b = body_text(
            b"{\n  \"a\": {\"x\": null, \"y\": [1, 2]},\n  \"b\": 1\n}",
            Some("application/json"),
            None,
            false,
        );
        assert_eq!(a.kind, "JSON");
        assert_eq!(a.lines, b.lines);
        assert_eq!(
            a.lines.join("\n"),
            "{\n  \"a\": {\n    \"x\": null,\n    \"y\": [\n      1,\n      2\n    ]\n  },\n  \"b\": 1\n}"
        );
        assert_eq!(body_text(b"{}", None, None, false).lines, ["{}"]);
    }

    #[test]
    fn other_bodies_are_text_or_hex() {
        assert_eq!(body_text(b"", None, None, false), BodyText::default());
        let text = body_text("a\r\nb\nc".as_bytes(), None, None, false);
        assert_eq!((text.kind.as_str(), text.lines), ("Text", lines("a\nb\nc")));
        // Cut inside "à".
        let cut = body_text(&"xà".as_bytes()[..2], None, None, true);
        assert_eq!(
            (cut.kind.as_str(), cut.lines, cut.cut),
            ("Text", lines("x"), true)
        );
        let bytes = body_text(&[0xff, 0, b'A'], None, None, false);
        assert_eq!(bytes.kind, "Bytes");
        assert_eq!(
            bytes.lines,
            [format!("00000000  ff 00 41{}  ..A", "   ".repeat(13))]
        );
        let packed = body_text(
            &[0x81, 0xa2, b'i', b'd', 0x07],
            Some("application/msgpack"),
            None,
            false,
        );
        assert_eq!(packed.kind, "MessagePack");
        assert_eq!(packed.lines, lines("{\n  \"id\": 7\n}"));
    }

    #[test]
    fn lines_changed_stand_beside_what_they_became() {
        let a = lines("one\ntwo\nthree\nfour");
        let b = lines("one\n2\nthree\nfour\nfive");
        let got: Vec<(Option<usize>, Option<usize>, Change)> = diff_lines(&a, &b)
            .iter()
            .map(|l| (l.a, l.b, l.change))
            .collect();
        assert_eq!(
            got,
            [
                (Some(0), Some(0), Change::Same),
                (Some(1), Some(1), Change::Changed),
                (Some(2), Some(2), Change::Same),
                (Some(3), Some(3), Change::Same),
                (None, Some(4), Change::Added),
            ]
        );
        assert_eq!(changed_count(&diff_lines(&a, &b)), 2);
        assert_eq!(changed_count(&diff_lines(&a, &a)), 0);
    }

    #[test]
    fn the_same_lines_far_from_a_change_are_folded() {
        let a: Vec<String> = (0..30).map(|n| n.to_string()).collect();
        let mut b = a.clone();
        b[15] = "fifteen".to_owned();
        let found = diff_lines(&a, &b);
        let shown = rows(&found, &HashSet::new());
        let folds: Vec<Row> = shown
            .iter()
            .copied()
            .filter(|r| matches!(r, Row::Folded { .. }))
            .collect();
        // 0..12 folded, 12..15 kept, the change, 16..19 kept, 19..30 folded.
        assert_eq!(
            folds,
            [
                Row::Folded { at: 0, count: 12 },
                Row::Folded { at: 19, count: 11 }
            ]
        );
        assert_eq!(shown.len(), 2 + 3 + 1 + 3);
        let opened = rows(&found, &HashSet::from([0]));
        assert_eq!(opened.len(), 12 + 3 + 1 + 3 + 1);
        // Nothing changed: all of it is one fold.
        assert_eq!(
            rows(&diff_lines(&a, &a), &HashSet::new()),
            [Row::Folded { at: 0, count: 30 }]
        );
    }
}
