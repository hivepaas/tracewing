//! Two flows side by side: what differs between them. Headers and the query
//! are matched by name; bodies line by line, JSON written the same way on
//! both sides first, so that only what it says can differ.

use similar::{Algorithm, DiffOp};
use std::{
    collections::HashSet,
    ops::Range,
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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BodyText {
    /// `JSON`, `MessagePack`, `Text`, `Bytes`…; empty for no body.
    pub kind: String,
    pub lines: Vec<String>,
    /// Only its first [`BODY_MAX`] bytes were compared.
    pub cut: bool,
    /// What it holds, when it is JSON or reads as JSON: compared by field.
    pub json: Option<serde_json::Value>,
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
        json: None,
    };
    if data.is_empty() {
        return BodyText::default();
    }
    if let Some(format) = binjson::format_of(content_type)
        && let Some(json) = binjson::to_json_as(data, format, message)
    {
        return match serde_json::from_str::<serde_json::Value>(&json) {
            Ok(value) => text(format.name(), &pretty_json(&value)).with_json(value),
            Err(_) => text(format.name(), &json),
        };
    }
    let starts_like_json = data
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| matches!(b, b'{' | b'['));
    if starts_like_json && let Ok(value) = serde_json::from_slice::<serde_json::Value>(data) {
        return text("JSON", &pretty_json(&value)).with_json(value);
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
        json: None,
    }
}

impl BodyText {
    fn with_json(mut self, value: serde_json::Value) -> Self {
        self.json = Some(value);
        self
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

/// What changed between two lines, a word at a time: where in each the
/// words differ, as byte ranges. Nothing if they share no word, as the
/// whole line then says it.
pub fn changed_spans(a: &str, b: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let (wa, wb) = (words(a), words(b));
    let ta: Vec<&str> = wa.iter().map(|r| &a[r.clone()]).collect();
    let tb: Vec<&str> = wb.iter().map(|r| &b[r.clone()]).collect();
    let deadline = Instant::now() + Duration::from_millis(20);
    let ops = similar::capture_diff_slices_deadline(Algorithm::Myers, &ta, &tb, Some(deadline));
    if !ops.iter().any(|op| matches!(op, DiffOp::Equal { .. })) {
        return (Vec::new(), Vec::new());
    }
    let span = |of: &[Range<usize>], from: usize, len: usize| -> Option<Range<usize>> {
        (len > 0).then(|| of[from].start..of[from + len - 1].end)
    };
    let (mut out_a, mut out_b) = (Vec::new(), Vec::new());
    for op in ops {
        let (old, new) = match op {
            DiffOp::Equal { .. } => continue,
            DiffOp::Delete {
                old_index, old_len, ..
            } => (span(&wa, old_index, old_len), None),
            DiffOp::Insert {
                new_index, new_len, ..
            } => (None, span(&wb, new_index, new_len)),
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => (span(&wa, old_index, old_len), span(&wb, new_index, new_len)),
        };
        out_a.extend(old);
        out_b.extend(new);
    }
    (merged(out_a), merged(out_b))
}

/// A line's words: runs of letters, digits and `_`, and each other
/// character alone, as byte ranges.
fn words(line: &str) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    let mut in_word = false;
    for (at, c) in line.char_indices() {
        let word = c.is_alphanumeric() || c == '_';
        match out.last_mut() {
            Some(last) if word && in_word => last.end = at + c.len_utf8(),
            _ => out.push(at..at + c.len_utf8()),
        }
        in_word = word;
    }
    out
}

/// Ranges that touch, made one.
fn merged(ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        match out.last_mut() {
            Some(last) if last.end >= range.start => last.end = last.end.max(range.end),
            _ => out.push(range),
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

/// The most fields a tree lists; the rest are left out, and that is said.
pub const TREE_MAX: usize = 20_000;
/// The most of a value written in a tree.
const VALUE_SHOWN: usize = 200;
/// Fields an array's objects are matched by, when each has one of its own.
const ID_FIELDS: [&str; 2] = ["id", "_id"];

/// A field that differs, or an object or array that holds some that do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub depth: usize,
    /// `user`, `[3]`, or `[id: 7]` for an item matched by its id.
    pub key: String,
    /// Its value in each flow, written short; `None` where it is not.
    pub a: Option<String>,
    pub b: Option<String>,
    pub change: Change,
    /// For an object or an array in both: how many fields under it differ.
    pub inside: Option<usize>,
}

/// The fields of `a` and `b` that differ, a node each, under the objects
/// and arrays that hold them, in the order of their keys. Arrays of objects
/// with ids are matched by id; others by place.
pub fn diff_tree(a: &serde_json::Value, b: &serde_json::Value) -> Vec<Node> {
    let mut out = Vec::new();
    if same_kind(a, b) {
        children(a, b, 0, &mut out);
    } else {
        walk("$".to_owned(), Some(a), Some(b), 0, &mut out);
    }
    out
}

/// How many fields differ: the nodes that are not objects or arrays.
pub fn fields_changed(nodes: &[Node]) -> usize {
    nodes.iter().filter(|n| n.inside.is_none()).count()
}

fn same_kind(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value;
    matches!(
        (a, b),
        (Value::Object(_), Value::Object(_)) | (Value::Array(_), Value::Array(_))
    )
}

fn short(value: &serde_json::Value) -> String {
    let text = value.to_string();
    match text.char_indices().nth(VALUE_SHOWN) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text,
    }
}

fn walk(
    key: String,
    a: Option<&serde_json::Value>,
    b: Option<&serde_json::Value>,
    depth: usize,
    out: &mut Vec<Node>,
) {
    if out.len() >= TREE_MAX {
        return;
    }
    let change = match (a, b) {
        (Some(a), Some(b)) if a == b => return,
        (Some(a), Some(b)) if same_kind(a, b) => {
            let at = out.len();
            out.push(Node {
                depth,
                key,
                a: None,
                b: None,
                change: Change::Changed,
                inside: Some(0),
            });
            children(a, b, depth + 1, out);
            out[at].inside = Some(fields_changed(&out[at + 1..]));
            return;
        }
        (Some(_), Some(_)) => Change::Changed,
        (Some(_), None) => Change::Removed,
        (None, Some(_)) => Change::Added,
        (None, None) => return,
    };
    out.push(Node {
        depth,
        key,
        a: a.map(short),
        b: b.map(short),
        change,
        inside: None,
    });
}

fn children(a: &serde_json::Value, b: &serde_json::Value, depth: usize, out: &mut Vec<Node>) {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a
                .keys()
                .chain(b.keys().filter(|k| !a.contains_key(*k)))
                .collect();
            keys.sort();
            for key in keys {
                walk(key.clone(), a.get(key), b.get(key), depth, out);
            }
        }
        (Value::Array(a), Value::Array(b)) => match id_field(a, b) {
            Some(field) => {
                let label = |item: &Value| format!("[{field}: {}]", item[field]);
                let ids_b: std::collections::HashMap<String, &Value> = b
                    .iter()
                    .map(|item| (item[field].to_string(), item))
                    .collect();
                let ids_a: HashSet<String> = a.iter().map(|item| item[field].to_string()).collect();
                for item in a {
                    let other = ids_b.get(&item[field].to_string()).copied();
                    walk(label(item), Some(item), other, depth, out);
                }
                for item in b
                    .iter()
                    .filter(|item| !ids_a.contains(&item[field].to_string()))
                {
                    walk(label(item), None, Some(item), depth, out);
                }
            }
            None => {
                for i in 0..a.len().max(b.len()) {
                    walk(format!("[{i}]"), a.get(i), b.get(i), depth, out);
                }
            }
        },
        _ => {}
    }
}

/// The field both arrays' items are matched by: one every item has, a
/// number or text, no two items in an array alike.
fn id_field(a: &[serde_json::Value], b: &[serde_json::Value]) -> Option<&'static str> {
    ID_FIELDS.into_iter().find(|field| {
        [a, b].iter().all(|items| {
            let mut seen = HashSet::new();
            items.iter().all(|item| match item.get(*field) {
                Some(id @ (serde_json::Value::Number(_) | serde_json::Value::String(_))) => {
                    seen.insert(id.to_string())
                }
                _ => false,
            })
        }) && !(a.is_empty() && b.is_empty())
    })
}

/// The nodes shown: all but those under a node in `closed`.
pub fn shown_nodes(nodes: &[Node], closed: &HashSet<usize>) -> Vec<usize> {
    let mut out = Vec::with_capacity(nodes.len());
    let mut skip_deeper_than = None;
    for (i, node) in nodes.iter().enumerate() {
        if let Some(depth) = skip_deeper_than {
            if node.depth > depth {
                continue;
            }
            skip_deeper_than = None;
        }
        out.push(i);
        if closed.contains(&i) {
            skip_deeper_than = Some(node.depth);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_changed_in_a_line_is_found_word_by_word() {
        let (a, b) = changed_spans(r#"  "price": 100,"#, r#"  "price": 120,"#);
        assert_eq!((a.len(), b.len()), (1, 1));
        assert_eq!((a[0].clone(), b[0].clone()), (11..14, 11..14));
        // A word put in, and one taken out.
        let (a, b) = changed_spans("GET /v1/orders", "GET /v2/orders?page=2");
        assert_eq!(&"GET /v1/orders"[a[0].clone()], "v1");
        assert_eq!(&"GET /v2/orders?page=2"[b[0].clone()], "v2");
        assert_eq!(&"GET /v2/orders?page=2"[b[1].clone()], "?page=2");
        // Nothing in common: the line says it all.
        assert_eq!(changed_spans("abc", "xyz"), (vec![], vec![]));
        // Words of any script.
        let (a, b) = changed_spans("Hà Nội", "Hà Giang");
        assert_eq!(
            (&"Hà Nội"[a[0].clone()], &"Hà Giang"[b[0].clone()]),
            ("Nội", "Giang")
        );
    }

    fn node(depth: usize, key: &str, a: Option<&str>, b: Option<&str>, change: Change) -> Node {
        Node {
            depth,
            key: key.to_owned(),
            a: a.map(str::to_owned),
            b: b.map(str::to_owned),
            change,
            inside: None,
        }
    }

    #[test]
    fn fields_that_differ_are_listed_under_what_holds_them() {
        let a = serde_json::json!({
            "user": {"id": 42, "name": "An", "tags": ["a", "b"]},
            "note": "first",
            "old": true,
            "same": [1, 2, 3]
        });
        let b = serde_json::json!({
            "user": {"id": 42, "name": "Bình", "tags": ["a", "b", "c"]},
            "note": 7,
            "new": null,
            "same": [1, 2, 3]
        });
        let tree = diff_tree(&a, &b);
        let mut user = node(0, "user", None, None, Change::Changed);
        user.inside = Some(2);
        let mut tags = node(1, "tags", None, None, Change::Changed);
        tags.inside = Some(1);
        assert_eq!(
            tree,
            [
                node(0, "new", None, Some("null"), Change::Added),
                node(0, "note", Some("\"first\""), Some("7"), Change::Changed),
                node(0, "old", Some("true"), None, Change::Removed),
                user,
                node(1, "name", Some("\"An\""), Some("\"Bình\""), Change::Changed),
                tags,
                node(2, "[2]", None, Some("\"c\""), Change::Added),
            ]
        );
        assert_eq!(fields_changed(&tree), 5);
        assert!(diff_tree(&a, &a).is_empty());
        // Not the same kind at the top: one node.
        assert_eq!(
            diff_tree(&serde_json::json!([1]), &serde_json::json!({"a": 1})),
            [node(
                0,
                "$",
                Some("[1]"),
                Some("{\"a\":1}"),
                Change::Changed
            )]
        );
    }

    #[test]
    fn items_with_ids_are_matched_by_id() {
        let a = serde_json::json!([{"id": 1, "n": "a"}, {"id": 2, "n": "b"}]);
        // One put in front, and one changed.
        let b = serde_json::json!([{"id": 9, "n": "z"}, {"id": 1, "n": "a"}, {"id": 2, "n": "B"}]);
        let keys: Vec<(String, Change)> = diff_tree(&a, &b)
            .into_iter()
            .map(|n| (n.key, n.change))
            .collect();
        assert_eq!(
            keys,
            [
                ("[id: 2]".to_owned(), Change::Changed),
                ("n".to_owned(), Change::Changed),
                ("[id: 9]".to_owned(), Change::Added),
            ]
        );
        // Ids that repeat cannot match: by place then.
        let twice = serde_json::json!([{"id": 1}, {"id": 1}]);
        assert_eq!(
            diff_tree(&twice, &serde_json::json!([{"id": 1}]))[0].key,
            "[1]"
        );
    }

    #[test]
    fn a_closed_node_hides_what_is_under_it() {
        let a = serde_json::json!({"x": {"y": {"z": 1}, "w": 1}, "v": 1});
        let b = serde_json::json!({"x": {"y": {"z": 2}, "w": 2}, "v": 2});
        let tree = diff_tree(&a, &b);
        let keys = |shown: Vec<usize>| -> Vec<&str> {
            shown.iter().map(|&i| tree[i].key.as_str()).collect()
        };
        assert_eq!(
            keys(shown_nodes(&tree, &HashSet::new())),
            ["v", "x", "w", "y", "z"]
        );
        assert_eq!(keys(shown_nodes(&tree, &HashSet::from([1]))), ["v", "x"]);
        assert_eq!(
            keys(shown_nodes(&tree, &HashSet::from([3]))),
            ["v", "x", "w", "y"]
        );
    }

    #[test]
    fn json_bodies_keep_what_they_hold() {
        let json = body_text(br#"{"a":1}"#, None, None, false);
        assert_eq!(json.json, Some(serde_json::json!({"a": 1})));
        assert_eq!(body_text(b"plain", None, None, false).json, None);
        let packed = body_text(
            &[0x81, 0xa1, b'a', 0x01],
            Some("application/msgpack"),
            None,
            false,
        );
        assert_eq!(packed.json, Some(serde_json::json!({"a": 1})));
    }

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
