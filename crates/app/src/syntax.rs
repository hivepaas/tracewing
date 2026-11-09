//! Colors for HTML (and XML), JavaScript and CSS, a line at a time.
//!
//! A body view draws only the lines in view, and a body can be gigabytes, so a
//! line is colored on its own, from nothing but its text. A comment or a string
//! that runs over lines is colored on its first line only, but for the common
//! case of a block comment's middle lines, which start with `*`. A tag whose
//! attributes go on over lines shows them as text after the first.

use crate::body::{Kind, Token, tokens};
use std::ops::Range;

/// How a body's lines are colored.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Syntax {
    Json,
    /// HTML, and any XML: SVG, feeds, SOAP.
    Markup,
    Script,
    Style,
    #[default]
    Plain,
}

impl Syntax {
    /// From the body's type, or for one with none or a vague one, from how it
    /// begins.
    pub fn of(kind: Kind, content_type: Option<&str>, head: &[u8]) -> Self {
        match kind {
            Kind::Json => return Self::Json,
            Kind::Binary => return Self::Plain,
            Kind::Text => {}
        }
        let essence = content_type
            .and_then(|t| t.split(';').next())
            .map(|t| t.trim().to_ascii_lowercase())
            .unwrap_or_default();
        match essence.as_str() {
            "text/html" | "application/xhtml+xml" => return Self::Markup,
            "text/css" => return Self::Style,
            t if t.contains("javascript") || t.contains("ecmascript") => return Self::Script,
            t if t.ends_with("/xml") || t.ends_with("+xml") => return Self::Markup,
            _ => {}
        }
        let start = head
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .unwrap_or(head.len());
        let head =
            String::from_utf8_lossy(&head[start..head.len().min(start + 64)]).to_ascii_lowercase();
        if ["<!doctype", "<html", "<?xml", "<svg"]
            .iter()
            .any(|mark| head.starts_with(mark))
        {
            Self::Markup
        } else {
            Self::Plain
        }
    }

    /// The colored runs of one line.
    pub fn runs(self, line: &str) -> Vec<(Range<usize>, Token)> {
        match self {
            Self::Json => tokens(line),
            Self::Markup => markup(line),
            Self::Script => script(line),
            Self::Style => style(line),
            Self::Plain => Vec::new(),
        }
    }
}

/// The middle of a block comment, as it is usually written.
fn comment_middle(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed == "*" || trimmed.starts_with("* ") || trimmed.starts_with("*/")
}

/// Where a run that may not end on this line ends: just past `end`, or at the
/// end of the line.
fn run_to(bytes: &[u8], from: usize, end: &[u8]) -> usize {
    bytes[from..]
        .windows(end.len())
        .position(|w| w == end)
        .map_or(bytes.len(), |at| from + at + end.len())
}

/// A quoted string from `open`, with `\\` escapes, to its closing quote or the
/// end of the line.
fn quoted(bytes: &[u8], open: usize) -> usize {
    let quote = bytes[open];
    let mut i = open + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b if b == quote => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

fn markup(line: &str) -> Vec<(Range<usize>, Token)> {
    let bytes = line.as_bytes();
    let mut runs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        if bytes[i..].starts_with(b"<!--") {
            i = run_to(bytes, i + 4, b"-->");
            runs.push((start..i, Token::Comment));
        } else if bytes[i] == b'<'
            && bytes
                .get(i + 1)
                .is_some_and(|b| b.is_ascii_alphabetic() || matches!(b, b'/' | b'!' | b'?'))
        {
            i += 1;
            if matches!(bytes[i], b'/' | b'!' | b'?') {
                i += 1;
            }
            runs.push((start..i, Token::Punct));
            let name = i;
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && !matches!(bytes[i], b'>' | b'/')
            {
                i += 1;
            }
            runs.push((name..i, Token::Tag));
            i = attributes(bytes, i, &mut runs);
        } else if bytes[i] == b'&' {
            let end = bytes[i..]
                .iter()
                .take(12)
                .position(|b| *b == b';')
                .map(|at| i + at + 1);
            match end {
                Some(end)
                    if bytes[i + 1..end - 1]
                        .iter()
                        .all(|b| b.is_ascii_alphanumeric() || *b == b'#') =>
                {
                    runs.push((i..end, Token::Literal));
                    i = end;
                }
                _ => i += 1,
            }
        } else {
            i += 1;
        }
    }
    runs
}

/// A tag's attributes, from after its name to past its `>`.
fn attributes(bytes: &[u8], mut i: usize, runs: &mut Vec<(Range<usize>, Token)>) -> usize {
    while i < bytes.len() {
        let start = i;
        match bytes[i] {
            b'>' => {
                runs.push((i..i + 1, Token::Punct));
                return i + 1;
            }
            b'/' | b'?' if bytes.get(i + 1) == Some(&b'>') => {
                runs.push((i..i + 2, Token::Punct));
                return i + 2;
            }
            b'=' => {
                i += 1;
                runs.push((start..i, Token::Punct));
            }
            b'"' | b'\'' => {
                i = quoted(bytes, i);
                runs.push((start..i, Token::Str));
            }
            b if b.is_ascii_whitespace() => i += 1,
            _ => {
                let after_equals = runs
                    .last()
                    .is_some_and(|(r, t)| *t == Token::Punct && bytes[r.start] == b'=');
                while i < bytes.len()
                    && !bytes[i].is_ascii_whitespace()
                    && !matches!(bytes[i], b'=' | b'>' | b'"' | b'\'')
                    && !(bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'>'))
                {
                    i += 1;
                }
                // `<a href=/x>`: a value without quotes.
                runs.push((
                    start..i,
                    if after_equals {
                        Token::Str
                    } else {
                        Token::Attr
                    },
                ));
            }
        }
    }
    i
}

const KEYWORDS: &[&str] = &[
    "as",
    "async",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "finally",
    "for",
    "from",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "of",
    "return",
    "static",
    "super",
    "switch",
    "throw",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];
const LITERALS: &[&str] = &[
    "true",
    "false",
    "null",
    "undefined",
    "this",
    "NaN",
    "Infinity",
];

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

fn script(line: &str) -> Vec<(Range<usize>, Token)> {
    if comment_middle(line) {
        return vec![(0..line.len(), Token::Comment)];
    }
    let bytes = line.as_bytes();
    let mut runs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                runs.push((i..bytes.len(), Token::Comment));
                break;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i = run_to(bytes, i + 2, b"*/");
                runs.push((start..i, Token::Comment));
            }
            b'"' | b'\'' | b'`' => {
                i = quoted(bytes, i);
                runs.push((start..i, Token::Str));
            }
            b'0'..=b'9' => {
                while i < bytes.len() && (is_ident(bytes[i]) || bytes[i] == b'.') {
                    i += 1;
                }
                runs.push((start..i, Token::Number));
            }
            b if is_ident_start(b) => {
                while i < bytes.len() && is_ident(bytes[i]) {
                    i += 1;
                }
                let word = &line[start..i];
                // `x.default`, `{ if: 1 }`: names, not keywords.
                let a_name = start > 0 && bytes[start - 1] == b'.'
                    || bytes.get(i) == Some(&b':') && bytes.get(i + 1) != Some(&b':');
                if LITERALS.contains(&word) {
                    runs.push((start..i, Token::Literal));
                } else if !a_name && KEYWORDS.contains(&word) {
                    runs.push((start..i, Token::Keyword));
                }
            }
            b'{' | b'}' | b'(' | b')' | b'[' | b']' | b';' | b',' => {
                i += 1;
                runs.push((start..i, Token::Punct));
            }
            _ => i += 1,
        }
    }
    runs
}

fn style(line: &str) -> Vec<(Range<usize>, Token)> {
    if comment_middle(line) {
        return vec![(0..line.len(), Token::Comment)];
    }
    let bytes = line.as_bytes();
    let trimmed = line.trim_end();
    // A formatted file's declarations sit on lines of their own, inside a rule
    // that opened on a line before. A selector list goes on with commas.
    let mut inside = !line.contains('{') && !trimmed.ends_with(',');
    let mut runs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i = run_to(bytes, i + 2, b"*/");
                runs.push((start..i, Token::Comment));
            }
            b'"' | b'\'' => {
                i = quoted(bytes, i);
                runs.push((start..i, Token::Str));
            }
            b'{' => {
                inside = true;
                i += 1;
                runs.push((start..i, Token::Punct));
            }
            b'}' => {
                inside = false;
                i += 1;
                runs.push((start..i, Token::Punct));
            }
            b';' | b':' | b',' if inside => {
                i += 1;
                runs.push((start..i, Token::Punct));
            }
            b'@' => {
                i += 1;
                while i < bytes.len() && (is_ident(bytes[i]) || bytes[i] == b'-') {
                    i += 1;
                }
                runs.push((start..i, Token::Keyword));
            }
            b'!' if inside => {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                runs.push((start..i, Token::Keyword));
            }
            b'#' if inside => {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                    i += 1;
                }
                runs.push((start..i, Token::Number));
            }
            b'0'..=b'9' if inside => {
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'.' | b'%'))
                {
                    i += 1;
                }
                runs.push((start..i, Token::Number));
            }
            b'.' if inside && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) => {
                i += 1;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'.' | b'%'))
                {
                    i += 1;
                }
                runs.push((start..i, Token::Number));
            }
            b if inside && (is_ident_start(b) || b == b'-') => {
                while i < bytes.len() && (is_ident(bytes[i]) || bytes[i] == b'-') {
                    i += 1;
                }
                let mut next = i;
                while next < bytes.len() && bytes[next] == b' ' {
                    next += 1;
                }
                if bytes.get(next) == Some(&b':') {
                    runs.push((start..i, Token::Key));
                }
            }
            b if !inside && !b.is_ascii_whitespace() && !matches!(b, b',' | b'>' | b'+' | b'~') => {
                // A selector, as far as the next space or combinator.
                while i < bytes.len()
                    && !bytes[i].is_ascii_whitespace()
                    && !matches!(bytes[i], b'{' | b',' | b'>' | b'+' | b'~' | b'"' | b'\'')
                    && !bytes[i..].starts_with(b"/*")
                {
                    i += 1;
                }
                runs.push((start..i, Token::Tag));
            }
            _ => i += 1,
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each run's text and what it is.
    fn said(syntax: Syntax, line: &str) -> Vec<(String, Token)> {
        syntax
            .runs(line)
            .into_iter()
            .map(|(range, token)| (line[range].to_owned(), token))
            .collect()
    }

    fn of(runs: &[(String, Token)], token: Token) -> Vec<&str> {
        runs.iter()
            .filter(|(_, t)| *t == token)
            .map(|(text, _)| text.as_str())
            .collect()
    }

    #[test]
    fn a_syntax_comes_from_the_type_or_the_start() {
        let text = |t: Option<&str>, head: &str| Syntax::of(Kind::Text, t, head.as_bytes());
        assert_eq!(text(Some("text/html; charset=utf-8"), ""), Syntax::Markup);
        assert_eq!(text(Some("application/javascript"), ""), Syntax::Script);
        assert_eq!(text(Some("text/javascript"), ""), Syntax::Script);
        assert_eq!(text(Some("text/css"), ""), Syntax::Style);
        assert_eq!(text(Some("image/svg+xml"), ""), Syntax::Markup);
        assert_eq!(text(Some("application/xml"), ""), Syntax::Markup);
        assert_eq!(text(None, "\n  <!DOCTYPE html><html>"), Syntax::Markup);
        assert_eq!(text(Some("text/plain"), "hello"), Syntax::Plain);
        assert_eq!(
            Syntax::of(Kind::Json, Some("text/html"), b"{"),
            Syntax::Json
        );
        assert_eq!(
            Syntax::of(Kind::Binary, Some("text/css"), b""),
            Syntax::Plain
        );
    }

    #[test]
    fn markup_shows_tags_attributes_and_comments() {
        let runs = said(
            Syntax::Markup,
            r#"<a href="/x" class='y' hidden data-n=3>Fish &amp; chips</a><!-- note -->"#,
        );
        assert_eq!(of(&runs, Token::Tag), ["a", "a"]);
        assert_eq!(
            of(&runs, Token::Attr),
            ["href", "class", "hidden", "data-n"]
        );
        assert_eq!(of(&runs, Token::Str), [r#""/x""#, "'y'", "3"]);
        assert_eq!(of(&runs, Token::Literal), ["&amp;"]);
        assert_eq!(of(&runs, Token::Comment), ["<!-- note -->"]);
        let closing = said(Syntax::Markup, "<br/><?xml version=\"1.0\"?>");
        assert_eq!(of(&closing, Token::Tag), ["br", "xml"]);
        assert!(of(&closing, Token::Punct).contains(&"/>"));
        // Text with a lone `<` or `&` is left alone.
        assert!(said(Syntax::Markup, "if a < b && c").is_empty());
    }

    #[test]
    fn script_shows_keywords_strings_and_comments() {
        let runs = said(
            Syntax::Script,
            r#"const x = await fetch("/a\"b", { if: 1, n: 0x1f }); // done"#,
        );
        assert_eq!(of(&runs, Token::Keyword), ["const", "await"]);
        assert_eq!(of(&runs, Token::Str), [r#""/a\"b""#]);
        assert_eq!(of(&runs, Token::Number), ["1", "0x1f"]);
        assert_eq!(of(&runs, Token::Comment), ["// done"]);
        let more = said(Syntax::Script, "return a.default ?? null; /* x */ `t`");
        assert_eq!(of(&more, Token::Keyword), ["return"]);
        assert_eq!(of(&more, Token::Literal), ["null"]);
        assert_eq!(of(&more, Token::Comment), ["/* x */"]);
        assert_eq!(of(&more, Token::Str), ["`t`"]);
        assert_eq!(said(Syntax::Script, "   * @param x")[0].1, Token::Comment);
    }

    #[test]
    fn style_shows_selectors_properties_and_values() {
        let rule = said(
            Syntax::Style,
            "a:hover, .nav > li{color:#fff;margin:0 1.5em!important}",
        );
        assert_eq!(of(&rule, Token::Tag), ["a:hover", ".nav", "li"]);
        assert_eq!(of(&rule, Token::Key), ["color", "margin"]);
        assert_eq!(of(&rule, Token::Number), ["#fff", "0", "1.5em"]);
        assert_eq!(of(&rule, Token::Keyword), ["!important"]);
        // A declaration on a line of its own is inside a rule.
        let line = said(Syntax::Style, "  font-family: \"Inter\", sans-serif;");
        assert_eq!(of(&line, Token::Key), ["font-family"]);
        assert_eq!(of(&line, Token::Str), ["\"Inter\""]);
        let at = said(Syntax::Style, "@media (max-width: 600px) {");
        assert_eq!(of(&at, Token::Keyword), ["@media"]);
        assert_eq!(said(Syntax::Style, " * a note")[0].1, Token::Comment);
    }

    #[test]
    fn runs_land_on_characters() {
        for syntax in [Syntax::Markup, Syntax::Script, Syntax::Style] {
            for line in [
                "<p title=\"héllo\">ñ</p>",
                "s = 'ünï' // é",
                "p{content:'→'}",
            ] {
                for (range, _) in syntax.runs(line) {
                    assert!(line.is_char_boundary(range.start) && line.is_char_boundary(range.end));
                    assert!(range.end <= line.len());
                }
            }
        }
    }
}
