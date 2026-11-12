//! The compose screen: a request written here, or a recorded one sent again,
//! edited or not. What comes back shows below it, and the flow shows in
//! Traffic like any other.

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
use gpui::{AppContext as _, Context, Entity, FontWeight, div, prelude::*, px, rgb};
use tracewing_i18n::{tr, tr_n};
use tracewing_proxy::pause::{self, Message};
use tracewing_store::{
    binjson, prost_reflect::MessageDescriptor, protojson, schema::Library, session::BodyReader,
    web3,
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

/// The most of an answer's body drawn as text.
const SHOWN_MAX: usize = 64 * 1024;

/// The JSON-RPC methods whose first parameter carries calldata, where a call
/// written out in the body stands for the bytes to send.
const CARRY_CALLDATA: [&str; 4] = [
    "eth_call",
    "eth_estimateGas",
    "eth_sendTransaction",
    "eth_createAccessList",
];

/// Every call written out in the `data` or `input` of a JSON-RPC body, in
/// the order they are written. Hex, as a node is sent, is not one of them.
fn written_calls(body: &serde_json::Value) -> Vec<String> {
    let each = match body.as_array() {
        Some(batch) => batch.iter().collect(),
        None => vec![body],
    };
    let mut found = Vec::new();
    for message in each {
        let method = message.get("method").and_then(serde_json::Value::as_str);
        if !method.is_some_and(|method| CARRY_CALLDATA.contains(&method)) {
            continue;
        }
        let Some(first) = message.get("params").and_then(|params| params.get(0)) else {
            continue;
        };
        for key in ["data", "input"] {
            match first.get(key).and_then(serde_json::Value::as_str) {
                Some(text) if web3::written(text).is_some() => found.push(text.to_owned()),
                _ => {}
            }
        }
    }
    found
}

/// The body to send, where a call is written out in it rather than sent as
/// hex: `balanceOf(0xd8dA…)` becomes the calldata it stands for, and the
/// signatures it was taken to mean come back with it to be shown.
///
/// Only those strings are replaced, so the rest of the body goes exactly as
/// it was written.
fn as_calldata(
    text: &str,
    signatures: &dyn Fn(&str) -> Vec<String>,
) -> Result<(String, Vec<String>), String> {
    let Ok(body) = serde_json::from_str::<serde_json::Value>(text) else {
        return Ok((text.to_owned(), Vec::new()));
    };
    let written = written_calls(&body);
    if written.is_empty() {
        return Ok((text.to_owned(), Vec::new()));
    }
    let mut out = text.to_owned();
    let mut signed = Vec::new();
    for call in written {
        let encoded = web3::calldata(&call, signatures).map_err(trouble_text)?;
        let was = serde_json::Value::String(call).to_string();
        let now = serde_json::Value::String(web3::hex_text(&encoded.data)).to_string();
        out = out.replacen(&was, &now, 1);
        signed.push(encoded.signature);
    }
    // Nothing is sent as a call written out: a body this could not rewrite,
    // because it writes its strings another way, would reach the node as
    // text where it wants hex.
    let left = serde_json::from_str::<serde_json::Value>(&out)
        .map(|body| written_calls(&body))
        .unwrap_or_default();
    if let Some(call) = left.first() {
        return Err(tr!(
            "{call} could not be put in the body as calldata. Write it as hex, or let tracewing write the body.",
            call = call.clone()
        ));
    }
    Ok((out, signed))
}

/// Why a written call could not be turned into calldata, in words.
fn trouble_text(trouble: web3::Trouble) -> String {
    match trouble {
        web3::Trouble::NotACall => tr!("That is not a call written out."),
        web3::Trouble::Unknown(name) => tr!(
            "No call named {name} is known here. Write its types with it, as transfer(address,uint256)(…), or add its ABI under Contract ABIs in Settings.",
            name = name
        ),
        web3::Trouble::Overloaded(found) => tr!(
            "{name} is more than one call: {found}. Write the types with it, as {first}(…).",
            name = web3::name_of(found.first().map(String::as_str).unwrap_or_default()).to_owned(),
            found = found.join(", "),
            first = found.first().cloned().unwrap_or_default()
        ),
        web3::Trouble::Types(types) => tr!(
            "{types} are not types a call can be written with.",
            types = types
        ),
        web3::Trouble::Values(signature, why) => tr!(
            "The values do not fit {signature}: {why}",
            signature = signature,
            why = why
        ),
    }
}

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
    fn new(cx: &mut Context<Traffic>) -> Self {
        let page = Self {
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
            calls: Vec::new(),
            left_out: Vec::new(),
            select: Selection::default(),
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
                let signatures = |name: &str| {
                    let mut found = abi.signatures(name);
                    found.extend(web3::known_signatures(name));
                    found
                };
                let (body, calls) = as_calldata(&text, &signatures)?;
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
        let protos = std::sync::Arc::clone(&self.protos);
        cx.spawn(async move |this, cx| {
            let (result, json) = cx
                .background_spawn(async move {
                    let result = compose::send(proxy, &ca, &message, stopped);
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
                .px(px(12.))
                .py(px(8.))
                .rounded(px(12.))
                .bg(rgb(c.input))
                .font_family(crate::MONO_FONT)
                .font_features(gpui::FontFeatures::disable_ligatures())
                .child(input.clone())
        };
        let chip = |id: &'static str, label: String, loud: bool| {
            div()
                .id(id)
                .flex_shrink_0()
                .h(px(34.))
                .px(px(16.))
                .rounded(px(17.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
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
                .child(sel.piece(text, "\n\n", Vec::new()))
        };

        let editor = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(14.))
            .rounded(px(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Request")))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.))
                    .child(div().w(px(110.)).flex_shrink_0().child(field(&page.method)))
                    .child(div().flex_1().min_w_0().child(field(&page.url))),
            )
            .child(eyebrow(tr!("Headers")))
            .child(field(&page.headers))
            .child(eyebrow(tr!("Body")))
            .children(page.proto.clone().map(|message| {
                div().text_size(px(12.)).text_color(rgb(c.muted)).child(tr!(
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
            .children((!page.calls.is_empty()).then(|| {
                div().text_size(px(12.)).text_color(rgb(c.muted)).child(tr!(
                    "Written out in the body and sent as calldata: {calls}",
                    calls = page.calls.join(", ")
                ))
            }))
            .children((!page.left_out.is_empty()).then(|| {
                div()
                    .text_size(px(12.))
                    .text_color(rgb(c.muted))
                    .child(tr_n!(
                        page.left_out.len(),
                        "{names} is the client's to work out, so what was written for it did not go.",
                        "{names} are the client's to work out, so what was written for them did not go.",
                        names = page.left_out.join(", ")
                    ))
            }))
            .children(page.stopped.then(|| {
                div().text_size(px(12.)).text_color(rgb(c.muted)).child(tr!(
                    "Stopped: no answer was read. What reached the server shows in Traffic."
                ))
            }))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.))
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
                .gap(px(10.))
                .p(px(14.))
                .rounded(px(22.))
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
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(20.))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The samples the inspector's tests read, which were captured from
    /// real nodes: `crates/store/testdata/jsonrpc/README.md`.
    macro_rules! sample {
        ($name:literal) => {
            include_str!(concat!("../../store/testdata/jsonrpc/", $name))
        };
    }

    /// Only the calls every chain shares: no ABI of the user's here.
    fn known(name: &str) -> Vec<String> {
        web3::known_signatures(name)
    }

    #[test]
    fn a_call_written_in_the_body_is_sent_as_the_calldata_it_stands_for() {
        let sent = sample!("call_balanceof.req.json");
        let json: serde_json::Value = serde_json::from_str(sent).unwrap();
        let hex = json["params"][0]["data"].as_str().unwrap();
        // Written out as the inspector shows it, in place of the hex.
        let data = web3::bytes_of(hex).unwrap();
        let shown = web3::arguments("balanceOf(address)", &data).unwrap();
        let written = sent.replace(hex, &format!("balanceOf({})", shown[0].text));
        assert!(written.contains("balanceOf(0x"), "{written}");

        let (body, calls) = as_calldata(&written, &known).unwrap();
        assert_eq!(body, sent, "the body goes as the node had it, hex and all");
        assert_eq!(calls, ["balanceOf(address)"]);
    }

    #[test]
    fn a_body_with_no_call_written_in_it_goes_as_it_is() {
        // Hex, as it is sent.
        let sent = sample!("call_balanceof.req.json");
        assert_eq!(
            as_calldata(sent, &known).unwrap(),
            (sent.to_owned(), vec![])
        );

        // Not JSON at all.
        let text = "balanceOf(0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045)";
        assert_eq!(
            as_calldata(text, &known).unwrap(),
            (text.to_owned(), vec![]),
            "the body is the body: only JSON-RPC calldata is written out"
        );

        // A method that carries no calldata, whatever its parameters hold.
        let other = r#"{"jsonrpc":"2.0","id":1,"method":"eth_getBalance","params":[{"data":"balanceOf(0x00)"}]}"#;
        assert_eq!(
            as_calldata(other, &known).unwrap(),
            (other.to_owned(), vec![])
        );
    }

    #[test]
    fn every_call_in_a_batch_is_written_out() {
        let one = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
        let body = format!(
            r#"[{{"jsonrpc":"2.0","id":1,"method":"eth_call","params":[{{"to":"{one}","data":"balanceOf({one})"}},"latest"]}},
               {{"jsonrpc":"2.0","id":2,"method":"eth_sendTransaction","params":[{{"to":"{one}","input":"transfer({one}, 1000000)"}}]}}]"#
        );
        let (sent, calls) = as_calldata(&body, &known).unwrap();
        assert_eq!(
            calls,
            ["balanceOf(address)", "transfer(address,uint256)"],
            "both, and `input` as well as `data`"
        );
        // Each is the hex that web3 makes of it, and nothing else moved.
        let before: serde_json::Value = serde_json::from_str(&body).unwrap();
        let json: serde_json::Value = serde_json::from_str(&sent).unwrap();
        for (at, key) in [(0usize, "data"), (1usize, "input")] {
            let written = before[at]["params"][0][key].as_str().unwrap();
            let hex = web3::hex_text(&web3::calldata(written, &known).unwrap().data);
            assert_eq!(json[at]["params"][0][key].as_str().unwrap(), hex);
        }
        assert_eq!(json[0]["params"][1], "latest");
        assert_eq!(json[1]["id"], 2);
    }

    #[test]
    fn a_call_that_cannot_be_written_stops_the_send_and_says_why() {
        let body =
            r#"{"jsonrpc":"2.0","id":1,"method":"eth_call","params":[{"data":"frobnicate(1)"}]}"#;
        let problem = as_calldata(body, &known).unwrap_err();
        assert!(problem.contains("frobnicate"), "{problem}");

        let overloaded = r#"{"jsonrpc":"2.0","id":1,"method":"eth_call","params":[{"data":"safeTransferFrom(0x00, 0x00, 1)"}]}"#;
        let problem = as_calldata(overloaded, &known).unwrap_err();
        assert!(
            problem.contains("safeTransferFrom(address,address,uint256)"),
            "{problem}"
        );
    }
}
