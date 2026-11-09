//! The compose screen: a request written here, or a recorded one sent again,
//! edited or not. What comes back shows below it, and the flow shows in
//! Traffic like any other.

use crate::{
    colors::colors,
    compose::{self, Answer},
    detail::Side,
    input::TextInput,
    paused_page::{headers_text, parse_headers},
    traffic::{Screen, Traffic},
    widgets::eyebrow,
};
use gpui::{AppContext as _, Context, Entity, FontWeight, div, prelude::*, px, rgb};
use tracewing_proxy::pause::{self, Message};
use tracewing_store::session::BodyReader;

/// The most of an answer's body drawn as text.
const SHOWN_MAX: usize = 64 * 1024;

pub struct ComposePage {
    pub method: Entity<TextInput>,
    pub url: Entity<TextInput>,
    pub headers: Entity<TextInput>,
    pub body: Entity<TextInput>,
    pub sending: bool,
    pub answer: Option<Answer>,
    pub problem: Option<String>,
}

fn set(input: &Entity<TextInput>, text: &str, cx: &mut Context<Traffic>) {
    input.update(cx, |input, cx| input.set_text(text, 0..0, cx));
}

impl ComposePage {
    fn new(cx: &mut Context<Traffic>) -> Self {
        let page = Self {
            method: cx.new(|cx| TextInput::new("Method", cx)),
            url: cx.new(|cx| TextInput::new("https://api.example.com/v1/items", cx)),
            headers: cx.new(|cx| TextInput::new("name: value, one a line", cx).multiline(3, 14)),
            body: cx.new(|cx| TextInput::new("No body", cx).multiline(5, 24)),
            sending: false,
            answer: None,
            problem: None,
        };
        set(&page.method, "GET", cx);
        page
    }
}

impl Traffic {
    fn compose_page(&mut self, cx: &mut Context<Self>) -> &mut ComposePage {
        if self.compose_page.is_none() {
            self.compose_page = Some(ComposePage::new(cx));
        }
        self.compose_page.as_mut().expect("just made")
    }

    /// Puts `message` in the editor, and sends it at once if `now`.
    pub fn compose(&mut self, message: Message, now: bool, cx: &mut Context<Self>) {
        let page = self.compose_page(cx);
        page.answer = None;
        page.problem = None;
        let body = String::from_utf8_lossy(&message.body).into_owned();
        let fields = [
            (page.method.clone(), message.method.clone()),
            (page.url.clone(), message.url.clone()),
            (page.headers.clone(), headers_text(&message.headers)),
            (page.body.clone(), body),
        ];
        for (input, text) in fields {
            set(&input, &text, cx);
        }
        self.set_screen(Screen::Compose, cx);
        if now {
            self.send_composed(cx);
        }
    }

    /// The flow in the inspector as a message, if it can be sent again.
    fn chosen_flow_message(&mut self) -> Result<Message, String> {
        let detail = self.inspector.detail.as_ref().ok_or("No flow is chosen")?;
        let facts = &detail.request;
        let body = facts
            .stored
            .exists()
            .then(|| BodyReader::new(self.session_dir()).unpacked(&facts.stored, pause::HOLD_MAX))
            .flatten();
        let whole = !facts.shed
            && !facts.lost
            && (facts.bytes == 0 || body.is_some())
            && facts.bytes as usize <= pause::HOLD_MAX;
        compose::from_flow(
            &detail.method,
            &detail.url,
            detail.headers(Side::Request).unwrap_or_default(),
            body,
            whole,
        )
        .map_err(|e| format!("This flow: {e:#}"))
    }

    /// Sends the chosen flow's request again, as it was.
    pub fn send_flow_again(&mut self, cx: &mut Context<Self>) {
        match self.chosen_flow_message() {
            Ok(message) => self.compose(message, true, cx),
            Err(problem) => self.system.problem = Some(problem),
        }
        cx.notify();
    }

    /// Opens the chosen flow's request in the editor.
    pub fn edit_flow(&mut self, cx: &mut Context<Self>) {
        match self.chosen_flow_message() {
            Ok(message) => self.compose(message, false, cx),
            Err(problem) => self.system.problem = Some(problem),
        }
        cx.notify();
    }

    fn composed_message(&mut self, cx: &mut Context<Self>) -> Result<Message, String> {
        let page = self.compose_page(cx);
        let read = |input: &Entity<TextInput>| input.read(cx).text().to_owned();
        let message = Message {
            method: read(&page.method).trim().to_owned(),
            url: read(&page.url).trim().to_owned(),
            status: 0,
            headers: parse_headers(&read(&page.headers))?,
            body: read(&page.body).into_bytes(),
        };
        // The same checks a breakpoint's edit gets.
        pause::request_from(message.clone())?;
        Ok(message)
    }

    pub fn send_composed(&mut self, cx: &mut Context<Self>) {
        if self.compose_page(cx).sending {
            return;
        }
        let message = match self.composed_message(cx) {
            Ok(message) => message,
            Err(problem) => {
                self.compose_page(cx).problem = Some(problem);
                cx.notify();
                return;
            }
        };
        let page = self.compose_page(cx);
        page.sending = true;
        page.problem = None;
        page.answer = None;
        let (proxy, ca) = (self.engine.addr, self.data_dir.join("ca.pem"));
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { compose::send(proxy, &ca, &message) })
                .await;
            let _ = this.update(cx, |this, cx| {
                let page = this.compose_page(cx);
                page.sending = false;
                match result {
                    Ok(answer) => page.answer = Some(answer),
                    Err(e) => page.problem = Some(format!("{e:#}")),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub fn new_compose(&mut self, cx: &mut Context<Self>) {
        let blank = Message {
            method: "GET".to_owned(),
            url: String::new(),
            status: 0,
            headers: Vec::new(),
            body: Vec::new(),
        };
        self.compose(blank, false, cx);
    }

    pub(crate) fn render_compose(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        self.compose_page(cx);
        let page = self.compose_page.as_ref().expect("just made");
        let field = |input: &Entity<TextInput>| {
            div()
                .w_full()
                .px(px(12.))
                .py(px(8.))
                .rounded(px(12.))
                .bg(rgb(c.input))
                .font_family(crate::MONO_FONT)
                .font_features(gpui::FontFeatures::disable_ligatures())
                .child(input.clone())
        };
        let chip = |id: &'static str, label: &'static str, loud: bool| {
            div()
                .id(id)
                .flex_shrink_0()
                .h(px(34.))
                .px(px(16.))
                .rounded(px(17.))
                .flex()
                .items_center()
                .cursor_pointer()
                .font_weight(FontWeight::BOLD)
                .when(loud, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!loud, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .child(label)
        };
        let mono_block = |text: String| {
            div()
                .w_full()
                .px(px(12.))
                .py(px(8.))
                .rounded(px(12.))
                .bg(rgb(c.input))
                .font_family(crate::MONO_FONT)
                .font_features(gpui::FontFeatures::disable_ligatures())
                .text_size(px(12.))
                .child(text)
        };

        let editor = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(14.))
            .rounded(px(22.))
            .bg(rgb(c.surface))
            .child(eyebrow("Request"))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.))
                    .child(div().w(px(110.)).flex_shrink_0().child(field(&page.method)))
                    .child(div().flex_1().min_w_0().child(field(&page.url))),
            )
            .child(eyebrow("Headers"))
            .child(field(&page.headers))
            .child(eyebrow("Body"))
            .child(field(&page.body))
            .children(
                page.problem
                    .clone()
                    .map(|problem| div().text_color(rgb(c.error)).child(problem)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.))
                    .child(if page.sending {
                        chip("compose-send", "Sending…", false)
                    } else {
                        chip("compose-send", "Send", true)
                            .on_click(cx.listener(|this, _, _, cx| this.send_composed(cx)))
                    })
                    .child(
                        chip("compose-new", "New", false)
                            .on_click(cx.listener(|this, _, _, cx| this.new_compose(cx))),
                    ),
            );

        let answer = page.answer.as_ref().map(|answer| {
            let (_, fg) = crate::widgets::status_colors(Some(answer.status));
            let shown = &answer.body[..answer.body.len().min(SHOWN_MAX)];
            let mut body = String::from_utf8_lossy(shown).into_owned();
            if answer.body.len() > SHOWN_MAX {
                body.push_str(&format!(
                    "\n… {} more bytes: the flow in Traffic has them all.",
                    answer.body.len() - SHOWN_MAX
                ));
            }
            div()
                .flex()
                .flex_col()
                .gap(px(10.))
                .p(px(14.))
                .rounded(px(22.))
                .bg(rgb(c.surface))
                .child(eyebrow("Response"))
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(fg))
                        .child(compose::summary(answer)),
                )
                .child(eyebrow("Headers"))
                .child(mono_block(headers_text(&answer.headers)))
                .child(eyebrow("Body"))
                .child(mono_block(if body.is_empty() {
                    "No body".to_owned()
                } else {
                    body
                }))
        });

        div()
            .id("compose")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(20.))
                            .font_weight(FontWeight::BOLD)
                            .child("Compose"),
                    )
                    .child(div().text_color(rgb(c.muted)).child(
                        "A request of your own, or one sent again. It goes through tracewing, \
                         so rules apply and it shows in Traffic too.",
                    )),
            )
            .child(editor)
            .children(answer)
    }
}
