//! Settings' part for the relay: another proxy that tracewing's connections
//! to servers go through, as a company's network asks. Kept in `relay.toml`;
//! applied to the next connection.

use crate::{
    colors::colors,
    input::TextInput,
    traffic::Traffic,
    widgets::{Reachable, eyebrow},
};
use gpui::{AppContext as _, Context, Entity, FontWeight, div, prelude::*, px, rgb};
use tracewing_i18n::tr;
use tracewing_proxy::relay::{Kind, RelaySettings, Route};

/// The relay's fields, and what the last look or change said.
pub struct RelayFields {
    pub proxy: Entity<TextInput>,
    pub bypass: Entity<TextInput>,
    /// Whether it went well, and what to say.
    pub said: Option<(bool, String)>,
}

/// What a relay in use is, for the user.
fn in_use(settings: &RelaySettings) -> String {
    match Route::parse(settings) {
        Ok(Some(route)) => {
            let kind = match route.kind {
                Kind::Http => "HTTP",
                Kind::Socks5 => "SOCKS5",
            };
            tr!(
                "New connections go through the {kind} proxy {address}.",
                kind = kind,
                address = route.address()
            )
        }
        Ok(None) => tr!("Connections go straight to servers."),
        Err(e) => format!("{e:#}"),
    }
}

impl Traffic {
    fn relay_fields(&mut self, cx: &mut Context<Self>) -> &RelayFields {
        if self.relay_fields.is_none() {
            let (settings, said) = match RelaySettings::load(&self.data_dir) {
                Ok(settings) => {
                    let said = in_use(&settings);
                    (settings, Some((true, said)))
                }
                Err(e) => (
                    RelaySettings::default(),
                    Some((
                        false,
                        tr!(
                            "relay.toml cannot be used, so connections go straight: {problem}",
                            problem = format!("{e:#}")
                        ),
                    )),
                ),
            };
            let proxy = cx.new(|cx| {
                let mut input = TextInput::said(
                    || tr!("http://proxy.example.com:8080 or socks5://proxy.example.com:1080"),
                    cx,
                );
                input.set_text(&settings.proxy, 0..0, cx);
                input
            });
            let bypass = cx.new(|cx| {
                let mut input = TextInput::new("", cx).multiline(2, 6);
                input.set_text(&settings.bypass, 0..0, cx);
                input
            });
            self.relay_fields = Some(RelayFields {
                proxy,
                bypass,
                said,
            });
        }
        self.relay_fields.as_ref().expect("just made")
    }

    /// Saves what the fields say and uses it from the next connection.
    pub fn apply_relay(&mut self, cx: &mut Context<Self>) {
        let fields = self.relay_fields(cx);
        let settings = RelaySettings {
            proxy: fields.proxy.read(cx).text().trim().to_owned(),
            bypass: fields.bypass.read(cx).text().trim().to_owned(),
        };
        let said = settings
            .save(&self.data_dir)
            .and_then(|()| self.engine.relay.set(&settings))
            .map(|()| (true, in_use(&settings)))
            .unwrap_or_else(|e| (false, format!("{e:#}")));
        if let Some(fields) = &mut self.relay_fields {
            fields.said = Some(said);
        }
        cx.notify();
    }

    pub(crate) fn render_relay(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let fields = self.relay_fields(cx);
        let (proxy, bypass, said) = (
            fields.proxy.clone(),
            fields.bypass.clone(),
            fields.said.clone(),
        );
        let field = |input: Entity<TextInput>| {
            div()
                .px(px(12.))
                .py(px(8.))
                .rounded(px(12.))
                .bg(rgb(c.input))
                .font_family(crate::MONO_FONT)
                .font_features(gpui::FontFeatures::disable_ligatures())
                .child(input)
        };
        let note = |text: String| {
            div()
                .px(px(6.))
                .text_size(px(12.))
                .text_color(rgb(c.muted))
                .child(text)
        };
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(14.))
            .rounded(px(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Go through another proxy")).px(px(6.)))
            .child(note(tr!(
                "Where a company's network lets nothing out but through its own proxy, tracewing goes on to servers through it. Apps still send their traffic to tracewing. A user name and password go in the address, as http://name:password@proxy:8080, and are kept as written in relay.toml. Empty, tracewing goes straight to servers."
            )))
            .child(field(proxy))
            .child(note(tr!(
                "Hosts to reach straight, as NO_PROXY has them: example.com is it and everything under it, * is every host."
            )))
            .child(field(bypass))
            .children(said.map(|(ok, text)| {
                div()
                    .px(px(6.))
                    .text_color(rgb(if ok { c.text } else { c.error }))
                    .child(text)
            }))
            .child(
                div().flex().flex_row().child(
                    div()
                        .id("relay-apply")
                        .h(px(32.))
                        .px(px(14.))
                        .rounded(px(16.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .reachable()
                        .text_size(px(12.5))
                        .font_weight(FontWeight::BOLD)
                        .bg(rgb(c.accent))
                        .text_color(rgb(c.on_accent))
                        .child(tr!("Apply"))
                        .on_click(cx.listener(|this, _, _, cx| this.apply_relay(cx))),
                ),
            )
    }
}
