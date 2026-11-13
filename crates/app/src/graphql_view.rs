//! The inspector's GraphQL part: what a request asks for, laid out, with its
//! variables. Its body says it as JSON with the query in one escaped string,
//! which reads as nothing at all.

use crate::text_size::ui;
use crate::{
    body::{Token, tokens},
    bodyview::highlight,
    colors::colors,
    select::Selection,
    traffic::Traffic,
};
use gpui::{Context, FontWeight, HighlightStyle, IntoElement, div, prelude::*, rgb};
use std::ops::Range;
use tracewing_i18n::tr;
use tracewing_store::{graphql, session::BodyReader};

/// The most of a request's body read for it: a query is never this long.
const BODY_MAX: usize = 4 << 20;

impl Traffic {
    /// Reads what the chosen flow's request asks for, if it is GraphQL.
    pub(crate) fn load_graphql(&mut self, cx: &mut Context<Self>) {
        self.inspector.graphql = None;
        let Some(detail) = &self.inspector.detail else {
            return;
        };
        if detail.graphql.is_none() {
            return;
        }
        let id = detail.id;
        let (method, url) = (detail.method.clone(), detail.url.clone());
        let content_type = detail.req_content_type.clone();
        let stored = detail.request.stored.clone();
        let session = self.session_dir().to_owned();
        cx.spawn(async move |this, cx| {
            let sent = cx
                .background_spawn(async move {
                    let body = if stored.exists() {
                        BodyReader::new(&session)
                            .unpacked(&stored, BODY_MAX)
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    graphql::sent(&method, &url, content_type.as_deref(), &body)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.inspector.detail.as_ref().map(|d| d.id) == Some(id) {
                    this.inspector.graphql = Some(sent);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Each operation the request sends, its text selectable with `sel`.
    pub(crate) fn render_graphql(
        &self,
        sel: &Selection,
        scroll: &gpui::ScrollHandle,
    ) -> impl IntoElement {
        let c = colors();
        let mut view = div()
            .id("graphql")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(scroll)
            .p(ui(14.))
            .pr(gpui::px(crate::scrollbar::BAR_W + 4.))
            .flex()
            .flex_col()
            .gap(ui(8.));
        let Some(sent) = &self.inspector.graphql else {
            return view.child(
                div()
                    .text_color(rgb(c.muted))
                    .child(tr!("Reading the request…")),
            );
        };
        if sent.is_empty() {
            return view.child(div().text_color(rgb(c.muted)).child(tr!(
                "The request's body was not kept, so its query cannot be shown."
            )));
        }
        let code = |text: String, highlights: Vec<(Range<usize>, HighlightStyle)>| {
            div()
                .w_full()
                .px(ui(12.))
                .py(ui(8.))
                .rounded(ui(12.))
                .bg(rgb(c.input))
                .font_family(crate::mono_font())
                .font_features(gpui::FontFeatures::disable_ligatures())
                .text_size(ui(12.))
                .child(sel.piece(text, "\n", highlights))
        };
        // A title, said in the copy as it is: it shows in capitals.
        let section = |title: String| {
            div()
                .pt(ui(6.))
                .text_size(ui(11.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(c.muted))
                .child(sel.piece(title.to_uppercase(), "\n\n", Vec::new()))
        };
        for (at, one) in sent.iter().enumerate() {
            let name = one
                .name
                .clone()
                .unwrap_or_else(|| tr!("An operation without a name"));
            let title = if sent.len() > 1 {
                tr!(
                    "{n} of {count}: {name}",
                    n = (at + 1).to_string(),
                    count = sent.len().to_string(),
                    name = name
                )
            } else {
                name
            };
            view = view.child(
                div()
                    .when(at > 0, |d| d.pt(ui(10.)))
                    .font_weight(FontWeight::BOLD)
                    .child(sel.piece(title, if at == 0 { "" } else { "\n\n" }, Vec::new())),
            );
            view = view.child(section(tr!("Query")));
            view = match &one.query {
                Some(query) => {
                    let text = graphql::pretty(query);
                    let runs = query_runs(&text)
                        .into_iter()
                        .map(|(range, token)| (range, highlight(token)))
                        .collect();
                    view.child(code(text, runs))
                }
                None => view.child(div().text_color(rgb(c.muted)).child(sel.piece(
                    tr!("A persisted query: the server keeps its text, and the request sends only its hash."),
                    "\n",
                    Vec::new(),
                ))),
            };
            if let Some(variables) = &one.variables {
                let text = serde_json::to_string_pretty(variables).unwrap_or_default();
                let runs = tokens(&text)
                    .into_iter()
                    .map(|(range, token)| (range, highlight(token)))
                    .collect();
                view = view
                    .child(section(tr!("Variables")))
                    .child(code(text, runs));
            }
        }
        view
    }
}

/// The colored runs of a query: its keywords, variables, strings, numbers and
/// comments. Names of fields and types stay as they are.
pub fn query_runs(text: &str) -> Vec<(Range<usize>, Token)> {
    let bytes = text.as_bytes();
    let mut runs = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let token = match bytes[i] {
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                Some(Token::Comment)
            }
            b'"' => {
                let triple = bytes[i..].starts_with(b"\"\"\"");
                i += if triple { 3 } else { 1 };
                while i < bytes.len() {
                    if triple && bytes[i..].starts_with(b"\"\"\"") {
                        i += 3;
                        break;
                    }
                    if !triple && bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if !triple && (bytes[i] == b'"' || bytes[i] == b'\n') {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                Some(Token::Str)
            }
            b'$' => {
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                Some(Token::Literal)
            }
            b if b.is_ascii_alphabetic() || b == b'_' => {
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                matches!(
                    &text[start..i],
                    "query" | "mutation" | "subscription" | "fragment" | "on"
                )
                .then_some(Token::Keyword)
                .or_else(|| {
                    matches!(&text[start..i], "true" | "false" | "null").then_some(Token::Literal)
                })
            }
            b if b.is_ascii_digit() || b == b'-' => {
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                Some(Token::Number)
            }
            _ => {
                i += text[i..].chars().next().map_or(1, char::len_utf8);
                None
            }
        };
        if let Some(token) = token {
            runs.push((start..i.min(bytes.len()), token));
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_is_colored_by_what_each_word_is() {
        let text = "query A($id: ID = \"x\") { user(id: $id, n: 2) { ok } } # end";
        let runs: Vec<(&str, Token)> = query_runs(text)
            .into_iter()
            .map(|(range, token)| (&text[range], token))
            .collect();
        assert_eq!(
            runs,
            [
                ("query", Token::Keyword),
                ("$id", Token::Literal),
                ("\"x\"", Token::Str),
                ("$id", Token::Literal),
                ("2", Token::Number),
                ("# end", Token::Comment),
            ]
        );
    }
}
