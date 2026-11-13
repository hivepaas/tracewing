//! The compose screen: a request written here, or a recorded one sent again,
//! edited or not. What comes back shows below it, and the flow shows in
//! Traffic like any other.

use crate::text_size::ui;
use crate::{
    colors::colors,
    compose::{self, Answer},
    detail::{Side, message_of},
    input::TextInput,
    paused_page::{headers_text, parse_headers},
    select::{Copied, Selection},
    traffic::{Screen, Traffic},
    widgets::{Reachable, eyebrow},
};
use gpui::{AppContext as _, Context, Entity, FontWeight, div, prelude::*, rgb};
use tracewing_i18n::{tr, tr_n};
use tracewing_proxy::pause::{self, Message};
use tracewing_store::{
    binjson, prost_reflect::MessageDescriptor, protojson, schema::Library, session::BodyReader,
};

/// An answer's Protobuf as JSON, with the message it is, if the `.proto`
/// files describe it.
fn answer_json(protos: &Library, url: &str, answer: &Answer) -> Option<(String, String)> {
    let content_type = answer
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, value)| value.as_str());
    let format = binjson::format_of(content_type).filter(|f| f.numbered())?;
    let message = message_of(protos, url, content_type, Side::Response)?;
    let json = protojson::to_editable(&answer.body, format, &message)?;
    Some((message.full_name().to_owned(), json))
}

/// What to say about a body whose signed transaction an edit has left
/// behind, if anything. Worked out as the body is drawn, so it is there
/// before the send rather than after it.
pub fn signature_trouble(page: &ComposePage, cx: &gpui::App) -> Option<String> {
    let (shown, bytes) = page.loaded.as_ref()?;
    let was = crate::signed::signed_in(bytes)?;
    let text = page.body.read(cx).text().to_owned();
    if &text == shown {
        return None;
    }
    crate::signed::trouble(&was, crate::signed::signed_in(text.as_bytes()).as_ref())
}

/// The most of an answer's body drawn as text.
const SHOWN_MAX: usize = 64 * 1024;

pub struct ComposePage {
    pub method: Entity<TextInput>,
    pub url: Entity<TextInput>,
    pub headers: Entity<TextInput>,
    pub body: Entity<TextInput>,
    pub sending: bool,
    /// Asks the send on its way to stop. Held while one is.
    pub stop: Option<compose::Stop>,
    /// Which send the page is showing: a send that finishes after it was
    /// stopped, or after another started, leaves the page alone.
    pub sent: u64,
    /// The last send was stopped part way, so there is no answer to show.
    pub stopped: bool,
    pub answer: Option<Answer>,
    /// The answer's Protobuf as JSON, and the message it is.
    pub answer_json: Option<(String, String)>,
    pub problem: Option<String>,
    /// The body as put in the editor, and the bytes it stands for: sent as
    /// they were while the text is not changed, so that bytes that are not
    /// text go as they came.
    pub loaded: Option<(String, Vec<u8>)>,
    /// The Protobuf message the body is written as, as JSON.
    pub proto: Option<String>,
    /// Why the body it came back with was not kept, where a request was
    /// taken up again from `compose.toml` and its body was left out.
    pub body_left: Option<compose::BodyLeft>,
    /// Which version the client asks with.
    pub speak: compose::Speak,
    /// The calls that were written out in the body and sent as calldata,
    /// by the signatures they were taken to mean.
    pub calls: Vec<String>,
    /// The headers written that the client works out for itself, so what
    /// was written for them did not go.
    pub left_out: Vec<String>,
    /// What is selected of the answer.
    pub select: Selection,
}

fn set(input: &Entity<TextInput>, text: &str, cx: &mut Context<Traffic>) {
    input.update(cx, |input, cx| input.set_text(text, 0..0, cx));
}

impl ComposePage {
    fn new(kept: &compose::Kept, cx: &mut Context<Traffic>) -> Self {
        let mut page = Self {
            method: cx.new(|cx| TextInput::said(|| tr!("Method"), cx)),
            url: cx.new(|cx| TextInput::new("https://api.example.com/v1/items", cx)),
            headers: cx
                .new(|cx| TextInput::said(|| tr!("name: value, one a line"), cx).multiline(3, 14)),
            body: cx.new(|cx| TextInput::said(|| tr!("No body"), cx).multiline(5, 24)),
            sending: false,
            stop: None,
            sent: 0,
            stopped: false,
            answer: None,
            answer_json: None,
            problem: None,
            loaded: None,
            proto: None,
            body_left: None,
            speak: compose::Speak::default(),
            calls: Vec::new(),
            left_out: Vec::new(),
            select: Selection::default(),
        };
        set(&page.method, "GET", cx);
        if !kept.is_empty() {
            // Taken up where it was left: the body only where keeping it
            // would send the same bytes again.
            page.body_left = kept.body_left.clone();
            page.speak = kept.speak;
            for (input, text) in [
                (page.method.clone(), &kept.method),
                (page.url.clone(), &kept.url),
                (page.headers.clone(), &kept.headers),
                (page.body.clone(), &kept.body),
            ] {
                set(&input, text, cx);
            }
        }
        page
    }
}

impl Traffic {
    fn compose_page(&mut self, cx: &mut Context<Self>) -> &mut ComposePage {
        if self.compose_page.is_none() {
            let kept = compose::Kept::load(&self.compose_path());
            self.compose_page = Some(ComposePage::new(&kept, cx));
        }
        self.compose_page.as_mut().expect("just made")
    }

    /// Where the composed request is kept between runs.
    pub(crate) fn compose_path(&self) -> std::path::PathBuf {
        self.data_dir.join("compose.toml")
    }

    /// Writes down what is in the editor, to be taken up next time. Called
    /// where the window knows something changed, and once more as the app
    /// quits, for what was typed and never sent.
    pub fn save_composed(&mut self, cx: &mut Context<Self>) {
        let Some(page) = &self.compose_page else {
            return;
        };
        let read = |input: &Entity<TextInput>| input.read(cx).text().to_owned();
        let body = read(&page.body);
        // The bytes the body would be sent as, where the text is not them.
        let bytes = page
            .loaded
            .as_ref()
            .filter(|(shown, _)| shown == &body)
            .map(|(_, bytes)| bytes.clone());
        let kept = compose::Kept::of(
            &read(&page.method),
            &read(&page.url),
            &read(&page.headers),
            &body,
            bytes.as_deref(),
            page.speak,
        );
        kept.save(&self.compose_path());
    }

    /// Which version the client asks tracewing with, kept for next time.
    pub fn set_speak(&mut self, speak: compose::Speak, cx: &mut Context<Self>) {
        self.compose_page(cx).speak = speak;
        self.save_composed(cx);
        cx.notify();
    }

    /// Puts what was kept back in the editor, as a start does.
    pub fn use_kept(&mut self, kept: &compose::Kept, cx: &mut Context<Self>) {
        self.compose_page = Some(ComposePage::new(kept, cx));
        cx.notify();
    }

    /// The Protobuf message a request to `url` with these headers sends, if
    /// the `.proto` files chosen say.
    fn request_proto(
        &self,
        url: &str,
        headers: &[(String, String)],
    ) -> Option<(binjson::Format, MessageDescriptor)> {
        let content_type = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, value)| value.as_str());
        let format = binjson::format_of(content_type).filter(|f| f.numbered())?;
        let message = message_of(&self.protos, url, content_type, Side::Request)?;
        Some((format, message))
    }

    /// Puts `message` in the editor, and sends it at once if `now`. A
    /// Protobuf body the `.proto` files describe is written as JSON.
    pub fn compose(&mut self, message: Message, now: bool, cx: &mut Context<Self>) {
        let proto =
            self.request_proto(&message.url, &message.headers)
                .and_then(|(format, schema)| {
                    let text = protojson::to_editable(&message.body, format, &schema)?;
                    Some((text, schema.full_name().to_owned()))
                });
        let page = self.compose_page(cx);
        // A send still on its way was for the request being replaced here.
        page.stop = None;
        page.sending = false;
        page.sent += 1;
        page.stopped = false;
        page.answer = None;
        page.answer_json = None;
        page.problem = None;
        page.calls = Vec::new();
        page.left_out = Vec::new();
        let body = match &proto {
            Some((text, _)) => text.clone(),
            None => String::from_utf8_lossy(&message.body).into_owned(),
        };
        page.loaded = Some((body.clone(), message.body.clone()));
        page.body_left = None;
        page.proto = proto.map(|(_, name)| name);
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
        self.save_composed(cx);
        if now {
            self.send_composed(cx);
        }
    }

    /// The flow in the inspector as a message, if it can be sent again.
    fn chosen_flow_message(&mut self) -> Result<Message, String> {
        let detail = self
            .inspector
            .detail
            .as_ref()
            .ok_or_else(|| tr!("No flow is chosen"))?;
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
        .map_err(|e| tr!("This flow: {e}", e = format!("{e:#}")))
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
        let text = read(&page.body);
        let loaded = page.loaded.clone();
        let mut message = Message {
            method: read(&page.method).trim().to_owned(),
            url: read(&page.url).trim().to_owned(),
            status: 0,
            headers: parse_headers(&read(&page.headers))?,
            body: Vec::new(),
        };
        let left_out = compose::left_out(&message.headers);
        let abi = self.abis.abi();
        let proto = self.request_proto(&message.url, &message.headers);
        let page = self.compose_page(cx);
        page.proto = proto.as_ref().map(|(_, m)| m.full_name().to_owned());
        page.left_out = left_out;
        page.calls = Vec::new();
        message.body = match (loaded, proto) {
            (Some((shown, bytes)), _) if shown == text => bytes,
            (_, Some((format, schema))) if text.trim_start().starts_with(['{', '[']) => {
                protojson::from_editable(&text, format, &schema).map_err(|e| {
                    tr!(
                        "The body is not a {message}: {e}",
                        message = schema.full_name().to_owned(),
                        e = e
                    )
                })?
            }
            // A call written out in a JSON-RPC body is sent as the calldata
            // it stands for, as a Protobuf body written as JSON is sent as
            // Protobuf.
            _ => {
                let signatures = crate::calldata::signatures_of(&abi);
                let (body, calls) = crate::calldata::as_calldata(&text, &signatures)?;
                self.compose_page(cx).calls = calls;
                body.into_bytes()
            }
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
        self.save_composed(cx);
        let (stop, stopped) = compose::stopper();
        let page = self.compose_page(cx);
        page.sending = true;
        page.stop = Some(stop);
        page.sent += 1;
        let mine = page.sent;
        page.stopped = false;
        page.problem = None;
        page.answer = None;
        page.answer_json = None;
        let (proxy, ca) = (self.engine.addr, self.data_dir.join("ca.pem"));
        let speak = self.compose_page(cx).speak;
        let protos = std::sync::Arc::clone(&self.protos);
        cx.spawn(async move |this, cx| {
            let (result, json) = cx
                .background_spawn(async move {
                    let result = compose::send(proxy, &ca, &message, speak, stopped);
                    let json = result
                        .as_ref()
                        .ok()
                        .and_then(|answer| answer.as_ref())
                        .and_then(|answer| answer_json(&protos, &message.url, answer));
                    (result, json)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let page = this.compose_page(cx);
                if page.sent != mine {
                    return;
                }
                page.sending = false;
                page.stop = None;
                match result {
                    // Stopped: the page says so already, and half an answer
                    // is worse than none.
                    Ok(None) => {}
                    Ok(Some(answer)) => {
                        page.answer = Some(answer);
                        page.answer_json = json;
                        page.select.clear();
                    }
                    Err(e) => page.problem = Some(format!("{e:#}")),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Stops the send on its way, which closes its connection rather than
    /// leave it waiting out `WAIT_MAX`. The page is free at once.
    pub fn stop_composed(&mut self, cx: &mut Context<Self>) {
        let page = self.compose_page(cx);
        if !page.sending {
            return;
        }
        // Dropping the other end is what the send waits on.
        page.stop = None;
        page.sending = false;
        // Whatever it comes back with is no longer this page's answer.
        page.sent += 1;
        page.stopped = true;
        cx.notify();
    }

    /// Copies what is selected of the answer. False when nothing is.
    pub fn copy_composed(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(page) = &self.compose_page else {
            return false;
        };
        page.select.set_menu(None);
        match page.select.selected_text() {
            Copied::Text(text) => {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                true
            }
            _ => false,
        }
    }

    /// Selects all of the answer, if there is one.
    pub fn select_all_composed(&mut self, cx: &mut Context<Self>) -> bool {
        match &self.compose_page {
            Some(page) if page.answer.is_some() => {
                page.select.select_all();
                cx.notify();
                true
            }
            _ => false,
        }
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
        let sel = page.select.clone();
        sel.begin();
        let field = |input: &Entity<TextInput>| {
            div()
                .w_full()
                .px(ui(12.))
                .py(ui(8.))
                .rounded(ui(12.))
                .bg(rgb(c.input))
                .font_family(crate::mono_font())
                .font_features(gpui::FontFeatures::disable_ligatures())
                .child(input.clone())
        };
        let chip = |id: &'static str, label: String, loud: bool| {
            div()
                .id(id)
                .flex_shrink_0()
                .h(ui(34.))
                .px(ui(16.))
                .rounded(ui(17.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .font_weight(FontWeight::BOLD)
                .when(loud, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!loud, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .child(label)
        };
        // Chosen or not, as the Messages box marks which way a message goes:
        // this picks, it does not act, so it is not the loud kind.
        let pick = |id: &'static str, label: String, on: bool| {
            div()
                .id(id)
                .flex_shrink_0()
                .h(ui(26.))
                .px(ui(10.))
                .rounded(ui(13.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(ui(12.))
                .font_weight(FontWeight::BOLD)
                .when(on, |d| d.bg(rgb(c.tonal)).text_color(rgb(c.on_tonal)))
                .when(!on, |d| d.text_color(rgb(c.muted)))
                .child(label)
        };
        let mono_block = |text: String| {
            div()
                .w_full()
                .px(ui(12.))
                .py(ui(8.))
                .rounded(ui(12.))
                .bg(rgb(c.input))
                .font_family(crate::mono_font())
                .font_features(gpui::FontFeatures::disable_ligatures())
                .text_size(ui(12.))
                .child(sel.piece(text, "\n\n", Vec::new()))
        };

        let editor = div()
            .flex()
            .flex_col()
            .gap(ui(10.))
            .p(ui(14.))
            .rounded(ui(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Request")))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(ui(8.))
                    .child(div().w(ui(110.)).flex_shrink_0().child(field(&page.method)))
                    .child(div().flex_1().min_w_0().child(field(&page.url))),
            )
            .child(eyebrow(tr!("Headers")))
            .child(field(&page.headers))
            .child(eyebrow(tr!("Body")))
            .children(page.body_left.clone().map(|why| {
                div()
                    .text_size(ui(12.))
                    .text_color(rgb(c.muted))
                    .child(match why {
                        compose::BodyLeft::TooBig(bytes) => tr!(
                            "The body of {bytes} bytes was too large to keep when the window last closed, so this request came back without it.",
                            bytes = bytes.to_string()
                        ),
                        compose::BodyLeft::NotText => tr!(
                            "This request came back without its body: the body it had was bytes that are not the text shown for them, and writing the text down would have sent something else."
                        ),
                    })
            }))
            .children(page.proto.clone().map(|message| {
                div().text_size(ui(12.)).text_color(rgb(c.muted)).child(tr!(
                    "Protobuf {message}, written as JSON: it is sent as Protobuf.",
                    message = message
                ))
            }))
            .child(field(&page.body))
            .children(
                page.problem
                    .clone()
                    .map(|problem| div().text_color(rgb(c.error)).child(problem)),
            )
            .children(
                signature_trouble(page, cx).map(|why| {
                    div().text_size(ui(12.)).text_color(rgb(c.warn)).child(why)
                }),
            )
            .children((!page.calls.is_empty()).then(|| {
                div().text_size(ui(12.)).text_color(rgb(c.muted)).child(tr!(
                    "Written out in the body and sent as calldata: {calls}",
                    calls = page.calls.join(", ")
                ))
            }))
            .children((!page.left_out.is_empty()).then(|| {
                div()
                    .text_size(ui(12.))
                    .text_color(rgb(c.muted))
                    .child(tr_n!(
                        page.left_out.len(),
                        "{names} is the client's to work out, so what was written for it did not go.",
                        "{names} are the client's to work out, so what was written for them did not go.",
                        names = page.left_out.join(", ")
                    ))
            }))
            .children((page.speak != compose::Speak::Agreed).then(|| {
                div().text_size(ui(12.)).text_color(rgb(c.muted)).child(tr!(
                    "{version} is how tracewing is asked. What the server is asked with is settled between tracewing and the server, as it is for every other flow.",
                    version = page.speak.text()
                ))
            }))
            .children(page.stopped.then(|| {
                div().text_size(ui(12.)).text_color(rgb(c.muted)).child(tr!(
                    "Stopped: no answer was read. What reached the server shows in Traffic."
                ))
            }))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui(4.))
                    .child(
                        div()
                            .text_size(ui(12.))
                            .text_color(rgb(c.muted))
                            .pr(ui(4.))
                            .child(tr!("Ask with")),
                    )
                    .children(compose::Speak::EVERY.map(|speak| {
                        let id: &'static str = match speak {
                            compose::Speak::Agreed => "compose-speak-agreed",
                            compose::Speak::Http1 => "compose-speak-http1",
                            compose::Speak::Http2 => "compose-speak-http2",
                        };
                        pick(id, speak.text(), page.speak == speak)
                            .on_click(cx.listener(move |this, _, _, cx| this.set_speak(speak, cx)))
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(ui(8.))
                    .child(if page.sending {
                        chip("compose-send", tr!("Sending…"), false)
                    } else {
                        chip("compose-send", tr!("Send"), true)
                            .on_click(cx.listener(|this, _, _, cx| this.send_composed(cx)))
                    })
                    .children(page.sending.then(|| {
                        chip("compose-stop", tr!("Stop"), true)
                            .on_click(cx.listener(|this, _, _, cx| this.stop_composed(cx)))
                    }))
                    .child(
                        chip("compose-new", tr!("New"), false)
                            .on_click(cx.listener(|this, _, _, cx| this.new_compose(cx))),
                    ),
            );

        let answer = page.answer.as_ref().map(|answer| {
            let (_, fg) = crate::widgets::status_colors(Some(answer.status));
            let shown = &answer.body[..answer.body.len().min(SHOWN_MAX)];
            let mut body = String::from_utf8_lossy(shown).into_owned();
            if answer.body.len() > SHOWN_MAX {
                body.push('\n');
                body.push_str(&tr!(
                    "… {count} more bytes: the flow in Traffic has them all.",
                    count = answer.body.len() - SHOWN_MAX
                ));
            }
            div()
                .flex()
                .flex_col()
                .gap(ui(10.))
                .p(ui(14.))
                .rounded(ui(22.))
                .bg(rgb(c.surface))
                .child(eyebrow(tr!("Response")))
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(fg))
                        .child(sel.piece(compose::summary(answer), "\n", Vec::new())),
                )
                .child(eyebrow(tr!("Headers")))
                .child(mono_block(headers_text(&answer.headers)))
                .child(eyebrow(match &page.answer_json {
                    Some((message, _)) => tr!("Body · {message}", message = message.clone()),
                    None => tr!("Body"),
                }))
                .child(mono_block(match &page.answer_json {
                    Some((_, json)) => json.clone(),
                    None if body.is_empty() => tr!("No body"),
                    None => body,
                }))
        });

        div()
            .id("compose")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(ui(10.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(ui(20.))
                            .font_weight(FontWeight::BOLD)
                            .child(tr!("Compose")),
                    )
                    .child(div().text_color(rgb(c.muted)).child(tr!(
                        "A request of your own, or one sent again. It goes through tracewing, \
                         so rules apply and it shows in Traffic too."
                    ))),
            )
            .child(editor)
            .children(answer.map(|answer| self.selectable(answer, &sel, cx)))
    }
}
