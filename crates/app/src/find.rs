//! Finding text in what the inspector shows: plain or as a regular
//! expression, with case or without, whole words or not.

use std::ops::Range;

use regex::{Regex, RegexBuilder};

/// How what is typed is matched. Kept while the inspector is open, from one
/// search and one flow to the next.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindOptions {
    /// `Abc` finds `Abc` only, not `abc`.
    pub case: bool,
    /// What is typed is a regular expression.
    pub regex: bool,
    /// Only where it is a word of its own: `id` in `id: 1`, not in `width`.
    pub word: bool,
}

/// The most a regular expression may grow to once compiled.
const SIZE_LIMIT: usize = 1 << 20;

/// What to look for, from what is typed: `None` for nothing typed, or why a
/// regular expression does not work.
pub fn matcher(query: &str, options: FindOptions) -> Result<Option<Regex>, String> {
    if query.is_empty() {
        return Ok(None);
    }
    let pattern = if options.regex {
        query.to_owned()
    } else {
        regex::escape(query)
    };
    let pattern = if options.word {
        format!(r"\b(?:{pattern})\b")
    } else {
        pattern
    };
    RegexBuilder::new(&pattern)
        .case_insensitive(!options.case)
        .size_limit(SIZE_LIMIT)
        .build()
        .map(Some)
        .map_err(|e| match e {
            regex::Error::Syntax(text) => text.lines().last().unwrap_or("").trim().to_owned(),
            regex::Error::CompiledTooBig(_) => "too large".to_owned(),
            other => other.to_string(),
        })
}

/// Behind what Find found, the one it is at brighter.
pub fn found_style(now: bool) -> gpui::HighlightStyle {
    let c = crate::colors::colors();
    gpui::HighlightStyle {
        background_color: Some(gpui::rgb(if now { c.found_now } else { c.found }).into()),
        ..Default::default()
    }
}

/// Where `re` finds something in `text`: what it finds empty, as `^`, is
/// passed over, as there is nothing to show of it.
pub fn found(re: &Regex, text: &str) -> impl Iterator<Item = Range<usize>> {
    re.find_iter(text)
        .map(|m| m.range())
        .filter(|range| !range.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(query: &str, options: FindOptions, text: &str) -> Vec<&'static str> {
        let re = matcher(query, options).unwrap().unwrap();
        found(&re, text)
            .map(|r| &*Box::leak(text[r].to_owned().into_boxed_str()))
            .collect()
    }

    #[test]
    fn plain_text_is_found_as_it_is_typed() {
        let plain = FindOptions::default();
        assert_eq!(all("a.b", plain, "a.b axb"), ["a.b"], "a dot is a dot");
        assert_eq!(all("ID", plain, "id Id ID"), ["id", "Id", "ID"]);
        let case = FindOptions {
            case: true,
            ..plain
        };
        assert_eq!(all("ID", case, "id Id ID"), ["ID"]);
    }

    #[test]
    fn whole_words_only_when_asked() {
        let word = FindOptions {
            word: true,
            ..Default::default()
        };
        assert_eq!(all("id", word, "id width uid id_x id."), ["id", "id"]);
        let regex_word = FindOptions {
            word: true,
            regex: true,
            ..Default::default()
        };
        assert_eq!(all("a|b", regex_word, "a ab b"), ["a", "b"]);
    }

    #[test]
    fn regular_expressions_and_their_mistakes() {
        let regex = FindOptions {
            regex: true,
            ..Default::default()
        };
        assert_eq!(all(r"\d{3}", regex, "1 22 333 4444"), ["333", "444"]);
        assert!(matcher("(", regex).is_err(), "a bad expression says why");
        assert!(
            matcher("", regex).unwrap().is_none(),
            "nothing typed, nothing found"
        );
        // Empty matches have nothing to show.
        let re = matcher("^", regex).unwrap().unwrap();
        assert_eq!(found(&re, "abc").count(), 0);
    }
}
