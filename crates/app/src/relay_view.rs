//! Settings' part for the relay: another proxy that tracewing's connections
//! to servers go through, as a company's network asks. Kept in `relay.toml`;
//! applied to the next connection.

use crate::text_size::ui;
use crate::{
    colors::colors,
    input::TextInput,
    traffic::Traffic,
    widgets::{Reachable, eyebrow},
};
use gpui::{AppContext as _, Context, Entity, FontWeight, div, prelude::*, rgb};
use tracewing_i18n::tr;
use tracewing_proxy::{
    ca_key::{KeyPlace, OsKeychain},
    relay::{Kind, RelaySettings, Route},
};

/// The relay's fields, and what the last look or change said.
pub struct RelayFields {
    pub proxy: Entity<TextInput>,
    pub bypass: Entity<TextInput>,
    /// Whether it went well, and what to say.
    pub said: Option<(bool, String)>,
}

/// Where the password is, for the user, if there is one.
fn password_place(place: Option<&KeyPlace>) -> Option<String> {
    Some(match place? {
        KeyPlace::Keychain => tr!("The password is kept in the system's keychain."),
        KeyPlace::File { why: None } => {
            tr!("The password is kept in relay.toml, which only you can read.")
        }
        KeyPlace::File { why: Some(why) } => tr!(
            "The password is kept in relay.toml, which only you can read: the keychain cannot be used ({why}).",
            why = why.clone()
        ),
    })
}

/// What a relay in use is, and where its password is, for the user.
fn said_of(settings: &RelaySettings, place: Option<&KeyPlace>) -> String {
    match password_place(place) {
        Some(password) => format!("{} {password}", in_use(settings)),
        None => in_use(settings),
    }
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
            let loaded =
                RelaySettings::load_kept(&self.data_dir, self.engine.key_store, &OsKeychain);
            let (settings, said) = match loaded {
                Ok((settings, place)) => {
                    let said = said_of(&settings, place.as_ref());
                    // The address is shown without its password.
                    (settings.without_password(), Some((true, said)))
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
        let (store, data_dir) = (self.engine.key_store, self.data_dir.clone());
        // Saved, then read back: a password kept from before comes with it.
        let applied = settings
            .save_kept(&data_dir, store, &OsKeychain)
            .and_then(|_| RelaySettings::load_kept(&data_dir, store, &OsKeychain))
            .and_then(|(used, place)| {
                self.engine.relay.set(&used)?;
                Ok((used, place))
            });
        let said = match &applied {
            Ok((used, place)) => (true, said_of(used, place.as_ref())),
            Err(e) => (false, format!("{e:#}")),
        };
        if let Some(fields) = &mut self.relay_fields {
            fields.said = Some(said);
            // A password written is not left on show.
            if let Ok((used, _)) = &applied {
                let shown = used.without_password().proxy;
                fields.proxy.update(cx, |input, cx| {
                    let all = 0..input.text().len();
                    input.set_text(&shown, all, cx);
                });
            }
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
                .px(ui(12.))
                .py(ui(8.))
                .rounded(ui(12.))
                .bg(rgb(c.input))
                .font_family(crate::mono_font())
                .font_features(gpui::FontFeatures::disable_ligatures())
                .child(input)
        };
        let note = |text: String| {
            div()
                .px(ui(6.))
                .text_size(ui(12.))
                .text_color(rgb(c.muted))
                .child(text)
        };
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .p(ui(14.))
            .rounded(ui(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Go through another proxy")).px(ui(6.)))
            .child(note(tr!(
                "Where a company's network lets nothing out but through its own proxy, tracewing goes on to servers through it. Apps still send their traffic to tracewing. A user name and password go in the address, as http://name:password@proxy:8080, or http://DOMAIN%5Cname:password@proxy:8080 for a Windows domain; the password is then kept in the system's keychain, not shown again. Basic or NTLM, as the proxy asks. Empty, tracewing goes straight to servers."
            )))
            .child(field(proxy))
            .child(note(tr!(
                "Hosts to reach straight, as NO_PROXY has them: example.com is it and everything under it, * is every host."
            )))
            .child(field(bypass))
            .children(said.map(|(ok, text)| {
                div()
                    .px(ui(6.))
                    .text_color(rgb(if ok { c.text } else { c.error }))
                    .child(text)
            }))
            .child(
                div().flex().flex_row().child(
                    div()
                        .id("relay-apply")
                        .h(ui(32.))
                        .px(ui(14.))
                        .rounded(ui(16.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .reachable()
                        .text_size(ui(12.5))
                        .font_weight(FontWeight::BOLD)
                        .bg(rgb(c.accent))
                        .text_color(rgb(c.on_accent))
                        .child(tr!("Apply"))
                        .on_click(cx.listener(|this, _, _, cx| this.apply_relay(cx))),
                ),
            )
    }
}
