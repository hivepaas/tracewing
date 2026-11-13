//! The first thing a new user needs: HTTPS that works. Three steps, each saying
//! where it stands, and the last one checks for real: a browser opens a page only
//! the proxy can answer, over HTTPS, which proves both that it goes through the
//! proxy and that it trusts the certificate. Nothing else can tell, since browsers
//! keep trust stores of their own.

use crate::text_size::ui;
use crate::{
    colors::colors,
    devices::on_the_network,
    qr::qr_code,
    system::{CaState, ProxySwitch, QuicSwitch},
    traffic::{Screen, Traffic},
    widgets::{Reachable, icon},
};
use gpui::{
    ClipboardItem, Context, Div, ElementId, FontWeight, SharedString, Stateful, div, prelude::*,
    rgb,
};
use std::net::{IpAddr, SocketAddr};
use tracewing_i18n::tr;
use tracewing_i18n::tr_n;
use tracewing_proxy::{
    ca_key::KeyPlace,
    capture::{CHECK_HOST, CheckVisit},
};
use tracewing_system::{
    BrowserTrust, Scope,
    android::{Device, SetUp, Trust},
    launch::Engine,
};

/// The browser that sent a `User-Agent` and a `Sec-CH-UA`. Chromium's family
/// all say Chrome in the first, so the brands in the second come first. The
/// rest go in the order that tells them apart: Chrome's says Safari, and Edge's
/// says both.
pub fn browser_name(user_agent: &str, brands: &str) -> String {
    for (brand, name) in [
        ("Brave", "Brave"),
        ("Microsoft Edge", "Edge"),
        ("Opera", "Opera"),
        ("Google Chrome", "Chrome"),
        ("Chromium", "Chromium"),
    ] {
        if brands.contains(brand) {
            return name.to_owned();
        }
    }
    for (mark, name) in [
        ("Firefox/", "Firefox"),
        ("Edg/", "Edge"),
        ("OPR/", "Opera"),
        ("Chrome/", "Chrome"),
        ("Safari/", "Safari"),
    ] {
        if user_agent.contains(mark) {
            return name.to_owned();
        }
    }
    tr!("Your browser")
}

/// Where the CA's key is, in words.
fn key_text(place: &KeyPlace) -> String {
    match place {
        KeyPlace::Keychain => tr!("Its private key is in the system keychain."),
        KeyPlace::File { why: None } => {
            tr!("Its private key is in ca.key, which only you can read.")
        }
        KeyPlace::File { why: Some(why) } => tr!(
            "Its private key is in ca.key, which only you can read, as the system keychain \
             cannot take it: {why}.",
            why = why.to_string()
        ),
    }
}

/// What the check has found, in words, and whether all is well.
fn check_result(visit: Option<&CheckVisit>) -> (String, bool) {
    match visit {
        None => (
            tr!(
                "Open the page. If the browser warns about the certificate, or cannot find the \
                 page, one of the steps above is not done for that browser."
            ),
            false,
        ),
        Some(visit) => {
            let name = browser_name(&visit.user_agent, &visit.brands);
            if visit.secure {
                (
                    tr!("{name} goes through tracewing and trusts it.", name = name),
                    true,
                )
            } else {
                (
                    tr!(
                        "{name} goes through tracewing, but opened the page without HTTPS. \
                         Open it again over https://.",
                        name = name
                    ),
                    false,
                )
            }
        }
    }
}

fn button(id: impl Into<ElementId>, label: impl Into<SharedString>, loud: bool) -> Stateful<Div> {
    let c = colors();
    div()
        .id(id)
        .flex_shrink_0()
        .h(ui(36.))
        .px(ui(16.))
        .rounded(ui(18.))
        .flex()
        .items_center()
        .cursor_pointer()
        .reachable()
        .font_weight(FontWeight::BOLD)
        .when(loud, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
        .when(!loud, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
        .child(label.into())
}

/// One step: its number or a tick, what it is, where it stands, and what can be done.
fn step(number: &'static str, done: bool, title: String, text: String) -> Div {
    let c = colors();
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(ui(16.))
        .p(ui(18.))
        .rounded(ui(22.))
        .bg(rgb(c.surface))
        .child(
            div()
                .size(ui(32.))
                .flex_shrink_0()
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .font_weight(FontWeight::BOLD)
                .when(done, |d| {
                    d.bg(rgb(c.ok_ground)).child(icon("check", 16., c.ok))
                })
                .when(!done, |d| {
                    d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)).child(number)
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w(ui(280.))
                .flex()
                .flex_col()
                .gap(ui(2.))
                .child(
                    div()
                        .text_size(ui(15.))
                        .font_weight(FontWeight::BOLD)
                        .child(title),
                )
                .child(div().text_color(rgb(c.muted)).child(text)),
        )
}

/// What another device is told to do, and whether it can yet.
fn other_devices_text(on: bool, port: u16, here: Option<IpAddr>) -> String {
    if !on {
        return tr!(
            "A phone or another computer can send its traffic here too. Turn on \
             \u{201c}Let other devices use the proxy\u{201d} in Settings."
        );
    }
    let addr = match here {
        Some(ip) => format!("{}", SocketAddr::new(ip, port)),
        None => tr!(
            "this computer's address, port {port}",
            port = port.to_string()
        ),
    };
    tr!(
        "On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get \
         the certificate and trust it. Its flows show its address, and can be filtered with \
         device:. Name it from one of them, in Connection.",
        addr = addr,
        host = CHECK_HOST
    )
}

/// Another way, beside the steps: what it is, and what can be done.
fn aside(title: String, text: impl Into<SharedString>) -> Div {
    let c = colors();
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(ui(10.))
        .p(ui(18.))
        .rounded(ui(22.))
        .border_1()
        .border_color(rgb(c.surface))
        .child(
            div()
                .flex_1()
                .min_w(ui(280.))
                .flex()
                .flex_col()
                .gap(ui(2.))
                .child(
                    div()
                        .text_size(ui(15.))
                        .font_weight(FontWeight::BOLD)
                        .child(title),
                )
                .child(
                    div()
                        .text_color(rgb(c.muted))
                        .child(text.into() as SharedString),
                ),
        )
}

impl Traffic {
    pub(crate) fn render_setup(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let ca = self.system.ca.clone();
        let installed = ca == CaState::Trusted;
        let ca_text = match &ca {
            CaState::Checking => tr!("Looking for it…"),
            CaState::Installing => tr!("Installing. The system may ask you to confirm."),
            CaState::Removing => tr!("Removing. The system may ask you to confirm."),
            CaState::Trusted if self.system.ca_places.is_empty() => {
                tr!("This computer trusts it.")
            }
            CaState::Trusted => tr!(
                "Installed in: {places}.",
                places = places_text(&self.system.ca_places)
            ),
            // Linux: in the system's store, which browsers do not read.
            CaState::NotTrusted if !self.system.ca_places.is_empty() => {
                tr!(
                    "Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser."
                )
            }
            CaState::NotTrusted => {
                tr!(
                    "tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails."
                )
            }
        };
        let ca_text = match &self.system.ca_key {
            Some(place) => format!("{ca_text} {}", key_text(place)),
            None => ca_text,
        };
        let busy = matches!(
            ca,
            CaState::Checking | CaState::Installing | CaState::Removing
        );
        let confirm = self.system.confirm_remove && !busy;
        let can_remove = !busy && !confirm && !self.system.ca_places.is_empty();
        let no_certutil = self.system.environment.no_certutil.filter(|_| !installed);
        let ca_step = step("1", installed, tr!("Trust the certificate"), ca_text)
            .when(!installed && !busy, |d| {
                d.child(
                    button("setup-install", tr!("Install"), true).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.install_certificate(Scope::User, cx);
                        },
                    )),
                )
            })
            // Linux only: browsers there read their own stores, command-line tools
            // the system's, which takes an administrator.
            .when(
                cfg!(target_os = "linux") && !busy && !self.system.ca_verified,
                |d| {
                    d.child(
                        button(
                            "setup-install-machine",
                            tr!("Also for command-line tools"),
                            false,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.install_certificate(Scope::Machine, cx);
                        })),
                    )
                },
            )
            .when(can_remove, |d| {
                d.child(button("setup-remove", tr!("Remove"), false).on_click(
                    cx.listener(|this, _, _, cx| this.ask_to_remove_certificate(true, cx)),
                ))
            })
            .when(confirm, |d| {
                d.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .items_center()
                        .gap(ui(10.))
                        .pl(ui(48.))
                        .child(div().flex_1().min_w(ui(240.)).child(tr!(
                            "Take the certificate out of every store on this computer? \
                             HTTPS through tracewing then fails until it is installed again. \
                             tracewing keeps it, to install again later."
                        )))
                        .child(
                            button("setup-remove-yes", tr!("Remove"), true).on_click(
                                cx.listener(|this, _, _, cx| this.remove_certificate(cx)),
                            ),
                        )
                        .child(button("setup-remove-no", tr!("Cancel"), false).on_click(
                            cx.listener(|this, _, _, cx| this.ask_to_remove_certificate(false, cx)),
                        )),
                )
            })
            // Linux: browsers there are told what to trust with certutil.
            .when_some(no_certutil, |d, command| {
                let says = div().text_color(rgb(c.muted)).child(match command {
                    Some(_) => tr!(
                        "Browsers on Linux need certutil to be told to trust it. Install it, \
                         then Install here:"
                    ),
                    None => tr!(
                        "Browsers on Linux need certutil to be told to trust it. Install your \
                         distribution's NSS tools, then Install here."
                    ),
                });
                d.child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(ui(8.))
                        .pl(ui(48.))
                        .child(says)
                        .when_some(command, |d, command| {
                            d.child(code_block("setup-certutil", command.to_owned()))
                        }),
                )
            });

        let addr = self.engine.addr;
        let (proxy_on, working) = match self.system.proxy {
            ProxySwitch::On(_) => (true, false),
            ProxySwitch::Working => (false, true),
            ProxySwitch::Off => (false, false),
        };
        // Some desktops, tiling ones mostly, have no setting to change.
        let no_system_proxy = self
            .system
            .environment
            .no_system_proxy
            .clone()
            .filter(|_| !proxy_on);
        let proxy_text = if let Some(desktop) = &no_system_proxy {
            if desktop.is_empty() {
                tr!(
                    "This desktop has no system proxy setting that tracewing can change. Open \
                     a browser on its own from Browsers, or a terminal, below."
                )
            } else {
                tr!(
                    "{desktop} has no system proxy setting that tracewing can change. Open a \
                     browser on its own from Browsers, or a terminal, below.",
                    desktop = desktop.clone()
                )
            }
        } else if proxy_on {
            tr!(
                "Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.",
                addr = addr.to_string()
            )
        } else {
            tr!(
                "Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.",
                addr = addr.to_string()
            )
        };
        let proxy_step = step("2", proxy_on, tr!("Turn on the system proxy"), proxy_text).when(
            !working && no_system_proxy.is_none(),
            |d| {
                d.child(
                    button(
                        "setup-proxy",
                        if proxy_on {
                            tr!("Turn off")
                        } else {
                            tr!("Turn on")
                        },
                        !proxy_on,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_system_proxy(cx))),
                )
            },
        );

        let (check_text, checked) = check_result(self.check_visit.as_ref());
        let check_step = step("3", checked, tr!("Check it in your browser"), check_text).child(
            button("setup-check", tr!("Open the test page"), !checked).on_click(
                cx.listener(|_, _, _, cx| cx.open_url(&format!("https://{CHECK_HOST}/"))),
            ),
        );

        div()
            .id("setup")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(ui(10.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(ui(10.))
                    .pb(ui(4.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(ui(280.))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(ui(20.))
                                    .font_weight(FontWeight::BOLD)
                                    .child(tr!("Set up")),
                            )
                            .child(div().text_color(rgb(c.muted)).child(tr!(
                                "Three steps to see HTTPS traffic from this computer."
                            ))),
                    )
                    .child(button("setup-done", tr!("See traffic"), checked).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.set_screen(Screen::Traffic, cx);
                        }),
                    )),
            )
            .child(ca_step)
            .child(proxy_step)
            .child(check_step)
            .child(self.render_browsers(cx))
            .child(self.render_one_app(cx))
            .child(self.render_other_devices(cx))
            .child(self.render_android(cx))
            .children(self.render_simulators(cx))
            .child(self.render_http3(cx))
            .child(self.render_ai_tools())
            .child(self.render_when_done(cx))
    }
}

/// Where the certificate is, in words. On Linux the browsers' stores are one
/// per browser and profile, which the Browsers card tells apart.
fn places_text(places: &[tracewing_system::Place]) -> String {
    let mut names: Vec<String> = places
        .iter()
        .filter(|place| !cfg!(target_os = "linux") || !place.browsers)
        .map(|place| place.name.clone())
        .collect();
    if cfg!(target_os = "linux") && places.iter().any(|place| place.browsers) {
        names.push(tr!("the browsers' own stores"));
    }
    names.join(", ")
}

/// How a line of the Browsers card stands: well, not yet, or not known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Ok,
    Missing,
    Unknown,
}

/// Whether a browser trusts the certificate, in words.
pub fn trust_text(trust: BrowserTrust, ca: &CaState) -> (String, Mark) {
    let yes = || (tr!("Trusts the certificate."), Mark::Ok);
    let not_yet = || (tr!("Does not trust the certificate yet."), Mark::Missing);
    match trust {
        BrowserTrust::System => match ca {
            CaState::Trusted => yes(),
            CaState::NotTrusted => not_yet(),
            CaState::Checking | CaState::Installing | CaState::Removing => {
                (tr!("Looking…"), Mark::Unknown)
            }
        },
        BrowserTrust::Own { of: 0, .. } => (
            tr!("Has no profile yet. Open it once, then look again."),
            Mark::Missing,
        ),
        BrowserTrust::Own { holding, of } if holding >= of => yes(),
        BrowserTrust::Own { of: 1, .. } => not_yet(),
        BrowserTrust::Own { holding, of } => (
            tr_n!(
                of,
                "{holding} of its {n} profile trusts the certificate.",
                "{holding} of its {n} profiles trust the certificate.",
                holding = holding.to_string()
            ),
            Mark::Missing,
        ),
    }
}

/// Whether a browser has reached the test page, in words, from the latest
/// visit of a browser by that name.
pub fn seen_text(name: &str, visits: &[CheckVisit]) -> (String, Mark) {
    let visit = visits
        .iter()
        .rev()
        .find(|visit| browser_name(&visit.user_agent, &visit.brands) == name);
    match visit {
        None => (tr!("Has not opened the test page yet."), Mark::Unknown),
        Some(visit) if visit.secure => (
            tr!("Opened the test page through tracewing, over HTTPS."),
            Mark::Ok,
        ),
        Some(_) => (
            tr!("Opened the test page through tracewing, but without HTTPS."),
            Mark::Missing,
        ),
    }
}

/// What Safari needs that the other browsers do not. It has no profile of
/// its own to open, so it comes through tracewing only by the system proxy;
/// and it never sends a server on this computer through any proxy, by its
/// name `localhost` or its address — macOS's networking leaves them out
/// whatever the settings say. A name under `.localhost` comes through.
pub fn safari_text(proxy_on: bool) -> (String, Mark) {
    if proxy_on {
        (
            tr!(
                "Sends localhost and 127.0.0.1 straight to the server, as all of macOS's own \
                 networking does: a name under .localhost, such as playground.localhost, \
                 comes through tracewing."
            ),
            Mark::Unknown,
        )
    } else {
        (
            tr!(
                "Goes only where the system proxy sends it: nothing of it comes through \
                 tracewing until step 2 is on."
            ),
            Mark::Missing,
        )
    }
}

/// A line of the Browsers card, after a dot that says how it stands.
fn marked(text: String, mark: Mark) -> Div {
    let c = colors();
    let dot = match mark {
        Mark::Ok => c.ok,
        Mark::Missing => c.warn,
        Mark::Unknown => c.faint,
    };
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(ui(8.))
        .child(
            div()
                .size(ui(8.))
                .flex_shrink_0()
                .rounded_full()
                .bg(rgb(dot)),
        )
        .child(div().text_color(rgb(c.muted)).child(text))
}

impl Traffic {
    /// Each browser on this computer: whether it trusts the certificate, and
    /// whether it has reached the test page through tracewing.
    fn render_browsers(&mut self, cx: &mut Context<Self>) -> Div {
        let c = colors();
        let busy = matches!(
            self.system.ca,
            CaState::Checking | CaState::Installing | CaState::Removing
        );
        let proxy_on = matches!(self.system.proxy, ProxySwitch::On(_));
        let mut card = aside(
            tr!("Browsers"),
            tr!(
                "Each keeps its own idea of what to trust and where to send its traffic. \
                 Check opens the test page in it. Open on its own starts it with a profile of \
                 its own that goes through tracewing, the system proxy left alone."
            ),
        )
        .child(
            button("setup-browsers-look", tr!("Look again"), false)
                .on_click(cx.listener(|this, _, _, cx| this.start_ca_check(cx))),
        );
        if self.system.browsers.is_empty() && !busy {
            card = card.child(
                div()
                    .w_full()
                    .text_color(rgb(c.muted))
                    .child(tr!("No browser found on this computer.")),
            );
        }
        for (i, (browser, trust)) in self.system.browsers.iter().enumerate() {
            let name = browser.name;
            let (trust_line, trust_mark) = trust_text(*trust, &self.system.ca);
            let (seen_line, seen_mark) = seen_text(name, &self.check_visits);
            let safari = (browser.engine == Engine::Safari).then(|| safari_text(proxy_on));
            // Its own stores can take the certificate now; the system's is step 1.
            // Without certutil it would fail: step 1 says how to get it.
            let can_install = !busy
                && self.system.environment.no_certutil.is_none()
                && matches!(*trust, BrowserTrust::Own { holding, of } if of > 0 && holding < of);
            card = card.child(
                div()
                    .id(("setup-browser-row", i))
                    .w_full()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(ui(12.))
                    .pt(ui(10.))
                    .border_t_1()
                    .border_color(rgb(c.surface))
                    .child(
                        div()
                            .w(ui(96.))
                            .flex_shrink_0()
                            .font_weight(FontWeight::BOLD)
                            .child(name),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(ui(240.))
                            .flex()
                            .flex_col()
                            .gap(ui(2.))
                            .child(marked(trust_line, trust_mark))
                            .child(marked(seen_line, seen_mark))
                            .when_some(safari, |d, (line, mark)| d.child(marked(line, mark))),
                    )
                    .when(can_install, |d| {
                        d.child(
                            button(("setup-browser-install", i), tr!("Install"), true).on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.install_certificate(Scope::User, cx)
                                }),
                            ),
                        )
                    })
                    .child(
                        button(("setup-browser-check", i), tr!("Check"), false).on_click(
                            cx.listener(move |this, _, _, cx| this.check_in_browser(name, cx)),
                        ),
                    )
                    // Safari has one profile, the user's.
                    .when(browser.engine != Engine::Safari, |d| {
                        d.child(
                            button(("setup-browser-open", i), tr!("Open on its own"), false)
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.open_browser(name, cx)),
                                ),
                        )
                    }),
            );
        }
        card
    }
}

/// What blocking UDP port 443 does, before and while it is on.
fn http3_text(on: bool) -> String {
    if on {
        tr!(
            "UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 \
             and go through tracewing. It stops when tracewing closes, or if it crashes."
        )
    } else {
        tr!(
            "Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 \
             on this computer makes them fall back to HTTP/2 through tracewing. It takes an \
             administrator's password, and stops when tracewing closes. A VPN that uses UDP \
             port 443 stops working meanwhile."
        )
    }
}

impl Traffic {
    /// Apps on this computer that would speak HTTP/3 around the proxy.
    fn render_http3(&mut self, cx: &mut Context<Self>) -> Div {
        let (on, working) = match self.system.quic {
            QuicSwitch::On(_) => (true, false),
            QuicSwitch::Working => (false, true),
            QuicSwitch::Off => (false, false),
        };
        aside(tr!("Apps on HTTP/3"), http3_text(on)).when(!working, |d| {
            d.child(
                button(
                    "setup-quic",
                    if on {
                        tr!("Stop blocking")
                    } else {
                        tr!("Block HTTP/3")
                    },
                    false,
                )
                .on_click(cx.listener(|this, _, _, cx| this.toggle_quic_block(cx))),
            )
        })
    }
}

impl Traffic {
    /// A terminal or a browser through tracewing, for when the system proxy
    /// would take more than the one app being debugged.
    fn render_one_app(&mut self, cx: &mut Context<Self>) -> Div {
        aside(
            tr!("Or one app at a time"),
            tr!(
                "A terminal that goes through tracewing while the system proxy and every other \
                 app are left as they are. For a browser, Open on its own in Browsers does the \
                 same with a profile of its own."
            ),
        )
        .child(
            button("setup-terminal", tr!("Open a terminal"), false)
                .on_click(cx.listener(|this, _, _, cx| this.open_terminal(cx))),
        )
    }
}

/// Which device's guide the Set up page shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Guide {
    #[default]
    Apple,
    Android,
    Flutter,
}

impl Guide {
    pub const ALL: [Self; 3] = [Self::Apple, Self::Android, Self::Flutter];

    pub fn label(self) -> String {
        match self {
            Self::Apple => tr!("iPhone and iPad"),
            Self::Android => "Android".to_owned(),
            Self::Flutter => tr!("Flutter apps"),
        }
    }
}

/// A guide is said in text, with code to copy where code is needed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
    /// A part's own heading, as "When you are done".
    Title(String),
    Text(String),
    Code(String),
}

/// Trusts certificates the user added, in debug builds only.
const ANDROID_CONFIG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<network-security-config>
    <debug-overrides>
        <trust-anchors>
            <certificates src="user" />
            <certificates src="system" />
        </trust-anchors>
    </debug-overrides>
</network-security-config>"#;

const ANDROID_MANIFEST: &str = r#"<application
    android:networkSecurityConfig="@xml/network_security_config"
    ...>"#;

fn flutter_overrides(proxy: SocketAddr) -> String {
    format!(
        r#"import 'dart:io';
import 'package:flutter/foundation.dart';

class TracewingProxy extends HttpOverrides {{
  @override
  HttpClient createHttpClient(SecurityContext? context) {{
    return super.createHttpClient(context)
      ..findProxy = (uri) => 'PROXY {proxy}'
      ..badCertificateCallback = (cert, host, port) => kDebugMode;
  }}
}}

void main() {{
  if (kDebugMode) HttpOverrides.global = TracewingProxy();
  runApp(const MyApp());
}}"#
    )
}

/// How to get a device through tracewing and trusting it, once the proxy is
/// set on it. `page` is the proxy's own address as a URL, where the
/// certificate is.
pub fn guide(guide: Guide, proxy: SocketAddr, page: &str) -> Vec<Part> {
    let text = |t: String| Part::Text(t);
    match guide {
        Guide::Apple => vec![
            text(tr!(
                "1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.",
                server = proxy.ip().to_string(),
                port = proxy.port().to_string()
            )),
            text(tr!(
                "2. Scan the code with the camera, or open {page} in Safari, and get the \
                 certificate. Allow the download.",
                page = page
            )),
            text(tr!(
                "3. In Settings, General, VPN & Device Management, choose the tracewing CA \
                 profile and install it."
            )),
            text(tr!(
                "4. In Settings, General, About, Certificate Trust Settings, turn on full trust \
                 for tracewing CA."
            )),
            text(tr!(
                "5. Open https://{host} in Safari. If it says it works, apps that use the \
                 system's settings go through tracewing.",
                host = CHECK_HOST
            )),
            Part::Title(tr!("When you are done")),
            text(tr!(
                "1. In Settings, Wi-Fi, choose the network, then Configure Proxy: Off. Left on, \
                 the device has no Internet on this network once tracewing closes."
            )),
            text(tr!(
                "2. In Settings, General, VPN & Device Management, choose tracewing CA and \
                 remove the profile. Its full trust goes with it."
            )),
        ],
        Guide::Android => vec![
            text(tr!(
                "1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to \
                 Manual: host {host}, port {port}.",
                host = proxy.ip().to_string(),
                port = proxy.port().to_string()
            )),
            text(tr!(
                "2. Scan the code, or open {page} in the browser, and get the certificate.",
                page = page
            )),
            text(tr!(
                "3. In Settings, Security, Encryption & credentials (the names vary by maker), \
                 choose Install a certificate, then CA certificate, and pick tracewing.crt."
            )),
            text(tr!(
                "4. Since Android 7, apps trust certificates the user added only if they say \
                 so. For a debug build of your own app, add res/xml/network_security_config.xml:"
            )),
            Part::Code(ANDROID_CONFIG.to_owned()),
            text(tr!("and point to it from AndroidManifest.xml:")),
            Part::Code(ANDROID_MANIFEST.to_owned()),
            text(tr!(
                "Other apps keep refusing the certificate: their traffic shows as failed flows."
            )),
            Part::Title(tr!("When you are done")),
            text(tr!(
                "1. In Settings, Wi-Fi, edit the network, and set the proxy back to None. Left \
                 on, the device has no Internet on this network once tracewing closes."
            )),
            text(tr!(
                "2. In Settings, Security, Encryption & credentials, User credentials, choose \
                 tracewing and remove it. Delete tracewing.crt from Downloads too."
            )),
        ],
        Guide::Flutter => vec![
            text(tr!(
                "Dart's HttpClient, which Flutter's http and Dio use, does not follow the \
                 phone's proxy setting, and on Android does not trust certificates the user \
                 added. In a debug build, send it here:"
            )),
            Part::Code(flutter_overrides(proxy)),
            text(tr!(
                "badCertificateCallback lets any certificate through, which is why it is for \
                 debug builds only."
            )),
            Part::Title(tr!("When you are done")),
            text(tr!(
                "Take TracewingProxy and its line in main() out before a release, though they \
                 do nothing outside debug builds. Nothing on the phone was changed."
            )),
        ],
    }
}

/// Code in a box, with a button that copies it.
fn code_block(id: impl Into<ElementId>, code: String) -> Div {
    let c = colors();
    let copied = code.clone();
    div()
        .relative()
        .w_full()
        .px(ui(14.))
        .py(ui(10.))
        .rounded(ui(12.))
        .bg(rgb(c.input))
        .font_family(crate::mono_font())
        .font_features(gpui::FontFeatures::disable_ligatures())
        .text_size(ui(12.))
        .child(code)
        .child(
            div()
                .id(id.into())
                .absolute()
                .top(ui(8.))
                .right(ui(8.))
                .px(ui(10.))
                .py(ui(3.))
                .rounded(ui(10.))
                .cursor_pointer()
                .reachable()
                .font_family(crate::ui_font())
                .bg(rgb(c.chip))
                .text_color(rgb(c.on_chip))
                .child(tr!("Copy"))
                .on_click(move |_, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(copied.clone()))
                }),
        )
}

impl Traffic {
    pub fn set_guide(&mut self, guide: Guide, cx: &mut Context<Self>) {
        self.system.guide = guide;
        cx.notify();
    }

    /// A phone or another computer, through the proxy here: a code to scan
    /// for the certificate, and how to trust it on each kind of device.
    fn render_other_devices(&mut self, cx: &mut Context<Self>) -> Div {
        let on = self.engine.settings.other_devices;
        let port = self.engine.addr.port();
        let here = on.then(on_the_network).flatten();
        let Some(ip) = here else {
            let text = other_devices_text(on, port, None);
            return aside(tr!("Other devices"), text).when(!on, |d| {
                d.child(
                    button("setup-devices", tr!("Open settings"), false).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.show_settings_section(crate::layout::SettingsSection::Network, cx);
                            this.set_screen(Screen::Settings, cx)
                        },
                    )),
                )
            });
        };
        let c = colors();
        let proxy = SocketAddr::new(ip, port);
        let page = format!("http://{proxy}/");
        let chosen = self.system.guide;
        let tabs = Guide::ALL.into_iter().map(|guide| {
            let on = guide == chosen;
            div()
                .id(ElementId::Name(format!("guide-{guide:?}").into()))
                .px(ui(14.))
                .py(ui(6.))
                .rounded(ui(16.))
                .cursor_pointer()
                .reachable()
                .font_weight(FontWeight::BOLD)
                .when(on, |d| d.bg(rgb(c.tab_on)))
                .when(!on, |d| d.text_color(rgb(c.muted)))
                .child(guide.label())
                .on_click(cx.listener(move |this, _, _, cx| this.set_guide(guide, cx)))
        });
        let parts =
            guide(chosen, proxy, &page)
                .into_iter()
                .enumerate()
                .map(|(at, part)| match part {
                    Part::Title(title) => {
                        div().pt(ui(6.)).font_weight(FontWeight::BOLD).child(title)
                    }
                    Part::Text(text) => div().child(text),
                    Part::Code(code) => code_block(("copy-guide", at), code),
                });
        div()
            .flex()
            .flex_col()
            .gap(ui(14.))
            .p(ui(18.))
            .rounded(ui(22.))
            .border_1()
            .border_color(rgb(c.surface))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(ui(18.))
                    .children(qr_code(&page, 168.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(ui(280.))
                            .flex()
                            .flex_col()
                            .gap(ui(4.))
                            .child(
                                div()
                                    .text_size(ui(15.))
                                    .font_weight(FontWeight::BOLD)
                                    .child(tr!("Phones and other devices")),
                            )
                            .child(div().text_color(rgb(c.muted)).child(tr!(
                                "A phone, a tablet or another computer on this network can go \
                                 through tracewing at {proxy}. The code opens the page with its \
                                 certificate. Its flows show its address, can be filtered with \
                                 device:, and can be named from one of them, in Connection.",
                                proxy = proxy.to_string()
                            ))),
                    ),
            )
            .children(self.engine.other_devices.problem().map(|problem| {
                div()
                    .px(ui(14.))
                    .py(ui(10.))
                    .rounded(ui(14.))
                    .bg(rgb(c.warn_ground))
                    .text_color(rgb(c.warn))
                    .child(tr!(
                        "Other devices cannot reach tracewing at one of this computer's \
                         addresses: {problem}",
                        problem = problem
                    ))
            }))
            .children(self.render_devices_to_undo())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(ui(4.))
                    .children(tabs),
            )
            .child(div().flex().flex_col().gap(ui(8.)).children(parts))
    }

    /// The devices this session has seen, which keep their proxy set until
    /// someone takes it off there: tracewing cannot.
    fn render_devices_to_undo(&self) -> Option<Div> {
        if self.device_counts().is_empty() {
            return None;
        }
        let c = colors();
        let names: Vec<String> = self
            .device_counts()
            .iter()
            .map(|(ip, _)| self.devices().shown(ip).to_owned())
            .collect();
        let count = names.len();
        Some(
            div()
                .flex()
                .flex_row()
                .gap(ui(10.))
                .px(ui(14.))
                .py(ui(10.))
                .rounded(ui(14.))
                .bg(rgb(c.warn_ground))
                .text_color(rgb(c.warn))
                .child(tr_n!(
                    count,
                    "{n} device went through tracewing ({names}). When you are done, turn its \
                     Wi-Fi proxy off, as below: left on, it has no Internet on this network once \
                     tracewing closes.",
                    "{n} devices went through tracewing ({names}). When you are done, turn their \
                     Wi-Fi proxy off, as below: left on, they have no Internet on this network \
                     once tracewing closes.",
                    names = names.join(", ")
                )),
        )
    }
}

/// Where a device stands, under its name.
fn android_note(device: &Device, set: Option<&SetUp>) -> String {
    match (device.state.as_str(), set) {
        (
            _,
            Some(SetUp {
                trust: Trust::System,
                ..
            }),
        ) => tr!("Set up. The certificate is among the system's until the device restarts."),
        (_, Some(SetUp { sdk, .. })) => {
            let mut note = tr!(
                "Set up. The certificate is in Downloads: in the Settings just opened, \
                 choose Encryption & credentials, Install a certificate, CA \
                 certificate, and pick tracewing.crt."
            );
            if *sdk >= 34 {
                note.push(' ');
                note.push_str(&tr!(
                    "Apps that trust only the system's certificates will still refuse it. On a device adb can root, as an emulator with a Google APIs image, tracewing puts it among the system's instead."
                ));
            }
            note
        }
        ("device", None) => {
            if device.emulator {
                tr!("An emulator.")
            } else {
                tr!("On USB.")
            }
        }
        ("unauthorized", None) => {
            tr!("Accept the USB debugging prompt on the device, then look again.")
        }
        ("offline", None) => tr!("Still starting. Look again in a moment."),
        (state, None) => tr!("adb says it is {state}.", state = state),
    }
}

impl Traffic {
    /// Emulators and phones on USB, set up over ADB in one go.
    fn render_android(&mut self, cx: &mut Context<Self>) -> Div {
        let c = colors();
        let state = &self.system.android;
        let text = match &state.adb {
            None => tr!(
                "Sets the proxy on an Android emulator, or a phone plugged in with USB \
                 debugging on, and installs the certificate. The traffic comes over ADB, so \
                 neither has to be on this network."
            ),
            Some(None) => tr!(
                "adb was not found. Install Android Studio, or Android's platform \
                 tools, then look again."
            ),
            Some(Some(_)) if state.devices.is_empty() => tr!(
                "No device is connected. Start an emulator, or plug in a phone with USB \
                 debugging on, then look again."
            ),
            Some(Some(_)) => tr!(
                "Set up a device to send its traffic here. Undo, or closing \
                 tracewing, takes the proxy off it again."
            ),
        };
        let looking = state.looking;
        let looked = state.adb.is_some();
        let rows =
            state.devices.iter().enumerate().map(|(at, device)| {
                let set = state.set.get(&device.serial);
                let working = state.working.as_deref() == Some(device.serial.as_str());
                let busy = state.working.is_some();
                let serial = device.serial.clone();
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(ui(12.))
                    .py(ui(6.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(ui(260.))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!("{} · {}", device.model, device.serial)),
                            )
                            .child(div().text_color(rgb(c.muted)).child(if working {
                                tr!("Working…")
                            } else {
                                android_note(device, set)
                            })),
                    )
                    .when(device.ready() && !busy, |d| {
                        let label = if set.is_some() {
                            tr!("Undo")
                        } else {
                            tr!("Set up")
                        };
                        d.child(button(("android", at), label, set.is_none()).on_click(
                            cx.listener(move |this, _, _, cx| {
                                if this.system.android.set.contains_key(&serial) {
                                    this.undo_android(serial.clone(), cx);
                                } else {
                                    this.set_up_android(serial.clone(), cx);
                                }
                            }),
                        ))
                    })
            });
        let problem = state.problem.clone();
        aside(tr!("Android emulators and USB devices"), text)
            .when(!looking, |d| {
                d.child(
                    button(
                        "setup-android",
                        if looked {
                            tr!("Look again")
                        } else {
                            tr!("Find devices")
                        },
                        false,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.find_android_devices(cx))),
                )
            })
            .child(div().w_full().flex().flex_col().children(rows))
            .children(problem.map(|problem| div().w_full().text_color(rgb(c.error)).child(problem)))
    }
}

impl Traffic {
    /// iOS simulators on this Mac: nothing where there can be none.
    fn render_simulators(&mut self, cx: &mut Context<Self>) -> Option<Div> {
        let c = colors();
        let state = &self.system.ios;
        if !state.possible() {
            return None;
        }
        let text = match &state.xcrun {
            None => tr!(
                "Trusts the certificate in the iOS simulators running on this Mac. They go \
                 through this Mac's proxy, so that is all they need."
            ),
            Some(None) => tr!(
                "xcrun was not found. Install Xcode, or its command line tools, then look again."
            ),
            Some(Some(_)) if state.simulators.is_empty() => {
                tr!("No simulator is running. Start one from Xcode, then look again.")
            }
            Some(Some(_)) => tr!(
                "Trust the certificate in a simulator. It stays until the simulator is erased; \
                 an app already open may need opening again."
            ),
        };
        let looked = state.xcrun.is_some();
        let looking = state.looking;
        let busy = state.working.is_some();
        let rows = state.simulators.iter().enumerate().map(|(at, sim)| {
            let trusted = state.trusted.contains(&sim.udid);
            let working = state.working.as_deref() == Some(sim.udid.as_str());
            let udid = sim.udid.clone();
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(ui(12.))
                .py(ui(6.))
                .child(
                    div()
                        .flex_1()
                        .min_w(ui(260.))
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .child(format!("{} · {}", sim.name, sim.runtime)),
                        )
                        .child(div().text_color(rgb(c.muted)).child(if working {
                            tr!("Working…")
                        } else if trusted {
                            tr!("The certificate is trusted.")
                        } else {
                            tr!("Running.")
                        })),
                )
                .when(!trusted && !busy, |d| {
                    d.child(
                        button(("simulator", at), tr!("Trust the certificate"), true).on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.trust_simulator(udid.clone(), cx)
                            }),
                        ),
                    )
                })
        });
        let problem = state.problem.clone();
        Some(
            aside(tr!("iOS simulators"), text)
                .when(!looking, |d| {
                    d.child(
                        button(
                            "setup-simulators",
                            if looked {
                                tr!("Look again")
                            } else {
                                tr!("Find simulators")
                            },
                            false,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.find_simulators(cx))),
                    )
                })
                .child(div().w_full().flex().flex_col().children(rows))
                .children(
                    problem.map(|problem| div().w_full().text_color(rgb(c.error)).child(problem)),
                ),
        )
    }
}

/// How an AI tool starts tracewing's MCP server: Claude Code's command, and the
/// JSON most other tools read.
pub fn mcp_setup(exe: &str) -> (String, String) {
    let quoted = if exe.contains(' ') {
        format!("\"{exe}\"")
    } else {
        exe.to_owned()
    };
    let claude = format!("claude mcp add tracewing -- {quoted} mcp");
    let config = serde_json::to_string_pretty(&serde_json::json!({
        "mcpServers": { "tracewing": { "command": exe, "args": ["mcp"] } }
    }))
    .unwrap_or_default();
    (claude, config)
}

impl Traffic {
    /// AI tools, reading what tracewing records through its MCP server.
    /// What goes back by itself, and what is the user's to undo, gathered.
    fn render_when_done(&mut self, cx: &mut Context<Self>) -> Div {
        let other_devices = self.engine.settings.other_devices;
        aside(
            tr!("When you are done"),
            tr!(
                "The system proxy and the block on HTTP/3 go back when tracewing closes, or if \
                 it crashes, and so does an Android device set up over USB. The certificate \
                 stays trusted until Remove, in step 1, takes it out. A phone set up by hand \
                 keeps its Wi-Fi proxy and certificate until they are taken off there: each \
                 device's guide says how. Other devices can reach this computer's proxy until \
                 that is turned off in Settings."
            ),
        )
        .when(other_devices, |d| {
            d.child(
                button("setup-done-devices", tr!("Open settings"), false).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.show_settings_section(crate::layout::SettingsSection::Network, cx);
                        this.set_screen(Screen::Settings, cx)
                    },
                )),
            )
        })
    }

    pub(crate) fn render_ai_tools(&self) -> Div {
        let exe = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "tracewing".to_owned());
        let (claude, config) = mcp_setup(&exe);
        aside(
            tr!("AI tools"),
            tr!(
                "Claude Code, Cursor, VS Code and other AI tools can list and read the flows \
                 tracewing records, through its MCP server. Tokens and cookies stay out unless \
                 Settings let them in. In Claude Code:"
            ),
        )
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(ui(8.))
                .child(code_block("mcp-claude", claude))
                .child(div().child(tr!("Elsewhere, in the tool's MCP settings:")))
                .child(code_block("mcp-config", config)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_tools_are_told_how_to_start_the_server() {
        let (claude, config) = mcp_setup("/Applications/My Apps/tracewing");
        assert_eq!(
            claude,
            "claude mcp add tracewing -- \"/Applications/My Apps/tracewing\" mcp"
        );
        let config: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(config["mcpServers"]["tracewing"]["args"][0], "mcp");
        assert_eq!(
            mcp_setup("/usr/bin/tracewing").0,
            "claude mcp add tracewing -- /usr/bin/tracewing mcp"
        );
    }

    #[test]
    fn a_device_says_where_it_stands() {
        let device = |serial: &str, state: &str| Device {
            serial: serial.to_owned(),
            state: state.to_owned(),
            model: "Pixel".to_owned(),
            emulator: serial.starts_with("emulator-"),
        };
        let emulator = device("emulator-5554", "device");
        assert_eq!(android_note(&emulator, None), "An emulator.");
        assert!(android_note(&device("X", "unauthorized"), None).contains("prompt"));
        let system = SetUp {
            sdk: 33,
            trust: Trust::System,
        };
        assert!(android_note(&emulator, Some(&system)).contains("system's"));
        let downloads = |sdk| SetUp {
            sdk,
            trust: Trust::Downloads,
        };
        let old = android_note(&emulator, Some(&downloads(33)));
        assert!(old.contains("Downloads") && !old.contains("still refuse"));
        assert!(android_note(&emulator, Some(&downloads(34))).contains("still refuse"));
    }

    #[test]
    fn each_guide_says_where_to_go() {
        let proxy: SocketAddr = "192.168.1.5:9090".parse().unwrap();
        let page = "http://192.168.1.5:9090/";
        let said = |guide| {
            super::guide(guide, proxy, page)
                .into_iter()
                .map(|part| match part {
                    Part::Title(t) | Part::Text(t) | Part::Code(t) => t,
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let apple = said(Guide::Apple);
        assert!(apple.contains("server 192.168.1.5, port 9090"), "{apple}");
        assert!(apple.contains(page));
        assert!(apple.contains("Certificate Trust Settings"));
        let android = said(Guide::Android);
        assert!(android.contains(r#"<certificates src="user" />"#));
        assert!(
            android.contains("debug-overrides"),
            "user certificates in debug builds only"
        );
        let flutter = said(Guide::Flutter);
        assert!(flutter.contains("'PROXY 192.168.1.5:9090'"), "{flutter}");
        assert!(flutter.contains("kDebugMode"));
        // Each ends by saying how to undo it.
        for guide in Guide::ALL {
            let parts = super::guide(guide, proxy, page);
            assert!(
                parts.contains(&Part::Title(tr!("When you are done"))),
                "{guide:?}"
            );
        }
        assert!(apple.contains("Configure Proxy: Off"), "{apple}");
        assert!(android.contains("set the proxy back to None"), "{android}");
    }

    #[test]
    fn other_devices_are_told_where_to_go() {
        let ip = "192.168.1.5".parse().unwrap();
        let text = other_devices_text(true, 9090, Some(ip));
        assert!(text.contains("192.168.1.5:9090"), "{text}");
        assert!(text.contains("http://tracewing.test"), "{text}");
        assert!(other_devices_text(false, 9090, Some(ip)).contains("Settings"));
        assert!(other_devices_text(true, 9090, None).contains("port 9090"));
    }

    #[test]
    fn browsers_are_told_apart() {
        for (agent, name) in [
            (
                "Mozilla/5.0 (X11; Linux x86_64; rv:144.0) Gecko/20100101 Firefox/144.0",
                "Firefox",
            ),
            (
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36",
                "Chrome",
            ),
            (
                "Mozilla/5.0 (Windows NT 10.0) AppleWebKit/537.36 Chrome/142.0.0.0 Safari/537.36 Edg/142.0.0.0",
                "Edge",
            ),
            (
                "Mozilla/5.0 (Macintosh) AppleWebKit/605.1.15 Version/26.0 Safari/605.1.15",
                "Safari",
            ),
            ("curl/8.5", "Your browser"),
        ] {
            assert_eq!(browser_name(agent, ""), name);
        }
        // Brave, Chromium and Chrome send the same user agent.
        let chrome =
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/142.0.0.0 Safari/537.36";
        for (brands, name) in [
            (
                r#""Chromium";v="142", "Brave";v="142", "Not_A Brand";v="99""#,
                "Brave",
            ),
            (
                r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#,
                "Chrome",
            ),
            (r#""Chromium";v="142", "Not_A Brand";v="99""#, "Chromium"),
            (r#""Opera";v="124", "Chromium";v="142""#, "Opera"),
        ] {
            assert_eq!(browser_name(chrome, brands), name);
        }
    }

    #[test]
    fn a_browser_says_what_it_trusts() {
        let own = |holding, of| BrowserTrust::Own { holding, of };
        assert_eq!(trust_text(own(1, 1), &CaState::NotTrusted).1, Mark::Ok);
        assert_eq!(trust_text(own(0, 1), &CaState::Trusted).1, Mark::Missing);
        let (text, mark) = trust_text(own(1, 2), &CaState::Trusted);
        assert_eq!(
            (text.as_str(), mark),
            ("1 of its 2 profiles trust the certificate.", Mark::Missing)
        );
        assert!(
            trust_text(own(0, 0), &CaState::Trusted)
                .0
                .contains("no profile")
        );
        let system = BrowserTrust::System;
        assert_eq!(trust_text(system, &CaState::Trusted).1, Mark::Ok);
        assert_eq!(trust_text(system, &CaState::NotTrusted).1, Mark::Missing);
        assert_eq!(trust_text(system, &CaState::Checking).1, Mark::Unknown);
    }

    #[test]
    fn a_browser_says_whether_it_reached_the_test_page() {
        let visit = |agent: &str, secure| CheckVisit {
            user_agent: agent.into(),
            brands: String::new(),
            secure,
        };
        let visits = [
            visit("Firefox/144.0", false),
            visit("Chrome/142.0", true),
            visit("Firefox/145.0", true),
        ];
        assert_eq!(
            seen_text("Firefox", &visits).1,
            Mark::Ok,
            "the latest counts"
        );
        assert_eq!(seen_text("Firefox", &visits[..2]).1, Mark::Missing);
        assert_eq!(seen_text("Edge", &visits).1, Mark::Unknown);
    }

    /// Safari has no profile of its own, so the system proxy is the whole of
    /// how it comes through; with it on, what is left to say is localhost.
    #[test]
    fn safari_says_what_it_needs() {
        let (text, mark) = safari_text(false);
        assert_eq!(mark, Mark::Missing);
        assert!(text.contains("step 2"), "{text}");
        let (text, mark) = safari_text(true);
        assert_eq!(mark, Mark::Unknown);
        assert!(text.contains(".localhost"), "{text}");
    }

    #[test]
    fn the_check_says_what_it_found() {
        assert!(!check_result(None).1);
        let plain = CheckVisit {
            user_agent: "Firefox/144.0".into(),
            brands: String::new(),
            secure: false,
        };
        let (text, ok) = check_result(Some(&plain));
        assert!(!ok && text.contains("without HTTPS"), "{text}");
        let secure = CheckVisit {
            secure: true,
            ..plain
        };
        let (text, ok) = check_result(Some(&secure));
        assert!(ok && text.starts_with("Firefox"), "{text}");
    }
}
