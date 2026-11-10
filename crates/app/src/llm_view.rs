//! The inspector's LLM part: a call to a model, as a conversation and an
//! answer instead of JSON and a few hundred events.

use crate::{
    colors::colors,
    flows::group_digits,
    inspector::Part,
    llm::{self, Exchange, Provider},
    select::Selection,
    traffic::Traffic,
};
use gpui::{
    AppContext as _, ClipboardItem, Context, FontWeight, IntoElement, div, prelude::*, px, rgb,
};
use tracewing_i18n::{tr, tr_n};
use tracewing_store::session::{BodyReader, StoredBody};

/// The most of each body read to put a call back together.
const BODY_MAX: usize = 32 << 20;
/// Past this, a message sent is cut in the view: a long one is a document,
/// and the Body part has it whole.
const MESSAGE_MAX: usize = 4000;

/// `text`, cut at a character at most `max` bytes in, and what was cut.
fn cut(text: &str, max: usize) -> (&str, usize) {
    if text.len() <= max {
        return (text, 0);
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (&text[..end], text.len() - end)
}

/// The facts of a call, in a line: who, which model, how it came, how it
/// ended, and what it cost.
pub fn summary(provider: Provider, call: &Exchange) -> Vec<String> {
    let mut facts = vec![provider.name().to_owned()];
    facts.extend(call.model.clone());
    if call.chunks > 0 {
        facts.push(tr_n!(
            call.chunks,
            "streamed in {count} chunk",
            "streamed in {count} chunks",
            count = group_digits(call.chunks as u64)
        ));
    }
    facts.extend(
        call.stop
            .as_ref()
            .map(|stop| tr!("stopped: {stop}", stop = stop)),
    );
    match (call.input_tokens, call.output_tokens) {
        (None, None) => {}
        (input, output) => facts.push(tr!(
            "{input} in · {output} out tokens",
            input = input.map_or("?".to_owned(), group_digits),
            output = output.map_or("?".to_owned(), group_digits)
        )),
    }
    facts
}

impl Traffic {
    /// Reads the chosen flow's bodies, off the window's thread, and puts the
    /// call back together if it is one.
    pub(crate) fn load_llm(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = &self.inspector.detail else {
            return;
        };
        let Some(provider) = llm::provider_of(&detail.method, &detail.url) else {
            self.inspector.llm = None;
            return;
        };
        let id = detail.id;
        let url = detail.url.clone();
        let request = detail.request.stored.clone();
        let response = detail.response.stored.clone();
        let session = self.session_dir().to_owned();
        cx.spawn(async move |this, cx| {
            let read = cx
                .background_spawn(async move {
                    let mut reader = BodyReader::new(&session);
                    let mut whole = |stored: &StoredBody| {
                        if stored.exists() {
                            reader.unpacked(stored, BODY_MAX).unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    };
                    let (request, response) = (whole(&request), whole(&response));
                    llm::read(provider, &url, &request, &response)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.inspector.detail.as_ref().map(|d| d.id) != Some(id) {
                    return;
                }
                let first = this.inspector.llm.is_none();
                this.inspector.llm = read.map(|call| (provider, call));
                // Shown at once, unless another part was chosen.
                if first
                    && this.inspector.llm.is_some()
                    && matches!(this.inspector.part, Part::Body | Part::Messages)
                {
                    this.inspector.part = Part::Llm;
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The call, its text selectable with `sel`.
    pub(crate) fn render_llm(&self, sel: &Selection) -> impl IntoElement {
        let c = colors();
        let Some((provider, call)) = &self.inspector.llm else {
            return div()
                .id("llm")
                .p(px(16.))
                .text_color(rgb(c.muted))
                .child(tr!("Reading the call…"));
        };
        let piece = |text: String, before: &'static str| sel.piece(text, before, Vec::new());
        let block = |text: String, muted: bool| {
            div()
                .w_full()
                .px(px(12.))
                .py(px(8.))
                .rounded(px(12.))
                .bg(rgb(c.surface))
                .text_color(rgb(if muted { c.muted } else { c.text }))
                .child(piece(text, "\n"))
        };
        let code = |text: String| {
            div()
                .w_full()
                .px(px(12.))
                .py(px(8.))
                .rounded(px(12.))
                .bg(rgb(c.input))
                .font_family(crate::MONO_FONT)
                .font_features(gpui::FontFeatures::disable_ligatures())
                .text_size(px(12.))
                .child(piece(text, "\n"))
        };
        // A title, said in the copy as it is: the eyebrow shows it in capitals.
        let section = |title: String| {
            div()
                .pt(px(6.))
                .text_size(px(11.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(c.muted))
                .child(piece(title.to_uppercase(), "\n\n"))
        };

        let mut answer = div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(section(tr!("Answer")));
        if let Some(error) = &call.error {
            answer = answer.child(
                div()
                    .text_color(rgb(c.error))
                    .child(piece(error.clone(), "\n")),
            );
        }
        if !call.thinking.is_empty() {
            answer = answer.child(block(
                tr!("Thinking: {thinking}", thinking = &call.thinking),
                true,
            ));
        }
        if !call.text.is_empty() {
            answer = answer.child(block(call.text.clone(), false));
        }
        for tool in &call.tool_calls {
            let id = tool
                .id
                .as_deref()
                .map(|id| format!(" · {id}"))
                .unwrap_or_default();
            answer = answer
                .child(div().font_weight(FontWeight::BOLD).child(piece(
                    tr!("Calls {name}{id}", name = &tool.name, id = id),
                    "\n",
                )))
                .when(!tool.arguments.is_empty(), |d| {
                    d.child(code(tool.arguments.clone()))
                });
        }
        if call.error.is_none()
            && call.text.is_empty()
            && call.thinking.is_empty()
            && call.tool_calls.is_empty()
        {
            answer = answer.child(block(tr!("Nothing yet."), true));
        }

        let messages = call.messages.iter().map(|message| {
            let (text, more) = cut(&message.text, MESSAGE_MAX);
            let text = if more > 0 {
                tr_n!(
                    more,
                    "{text}… and {count} more character",
                    "{text}… and {count} more characters",
                    text = text,
                    count = group_digits(more as u64)
                )
            } else {
                text.to_owned()
            };
            div()
                .flex()
                .flex_col()
                .gap(px(3.))
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(c.muted))
                        .child(piece(message.role.to_uppercase(), "\n\n")),
                )
                .child(block(text, false))
        });
        let answer_text = call.text.clone();
        div()
            .id("llm")
            .size_full()
            .overflow_y_scroll()
            .p(px(14.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .text_size(px(13.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(px(6.))
                    .children(summary(*provider, call).into_iter().map(|fact| {
                        div()
                            .px(px(10.))
                            .py(px(3.))
                            .rounded(px(12.))
                            .bg(rgb(c.chip))
                            .text_color(rgb(c.on_chip))
                            .text_size(px(12.))
                            .child(fact)
                    }))
                    .when(!answer_text.is_empty(), |d| {
                        d.child(
                            div()
                                .id("copy-answer")
                                .px(px(10.))
                                .py(px(3.))
                                .rounded(px(12.))
                                .cursor_pointer()
                                .text_size(px(12.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(c.link))
                                .child(tr!("Copy the answer"))
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        answer_text.clone(),
                                    ))
                                }),
                        )
                    }),
            )
            .child(answer)
            .child(section(tr!("Asked")))
            .children(messages)
            .when(!call.tools.is_empty(), |d| {
                d.child(div().text_color(rgb(c.muted)).child(piece(
                    tr!("Tools offered: {tools}", tools = call.tools.join(", ")),
                    "\n\n",
                )))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_is_summed_up_in_a_line() {
        let call = Exchange {
            model: Some("claude-sonnet-4-5".into()),
            chunks: 1204,
            stop: Some("end_turn".into()),
            input_tokens: Some(12_000),
            output_tokens: None,
            ..Exchange::default()
        };
        assert_eq!(
            summary(Provider::Anthropic, &call),
            [
                "Anthropic Messages",
                "claude-sonnet-4-5",
                "streamed in 1,204 chunks",
                "stopped: end_turn",
                "12,000 in · ? out tokens",
            ]
        );
        assert_eq!(summary(Provider::Gemini, &Exchange::default()), ["Gemini"]);
    }

    #[test]
    fn long_messages_are_cut_at_a_character() {
        assert_eq!(cut("đđđ", 3), ("đ", 4));
        assert_eq!(cut("short", 10), ("short", 0));
    }
}
