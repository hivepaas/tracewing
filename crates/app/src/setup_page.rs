//! The first thing a new user needs: HTTPS that works. Three steps, each saying
//! where it stands, and the last one checks for real: a browser opens a page only
//! the proxy can answer, over HTTPS, which proves both that it goes through the
//! proxy and that it trusts the certificate. Nothing else can tell, since browsers
//! keep trust stores of their own.

use crate::{
    colors::colors,
    devices::on_the_network,
    system::{CaState, ProxySwitch, QuicSwitch},
    traffic::{Screen, Traffic},
    widgets::icon,
};
use gpui::{Context, Div, FontWeight, SharedString, Stateful, div, prelude::*, px, rgb};
use std::net::{IpAddr, SocketAddr};
use tracewing_proxy::{
    ca_key::KeyPlace,
    capture::{CHECK_HOST, CheckVisit},
};
use tracewing_system::Scope;

/// The browser in a `User-Agent`, in the order that tells them apart: Chrome's says
/// Safari, and Edge's says both.
pub fn browser_name(user_agent: &str) -> &'static str {
    for (mark, name) in [
        ("Firefox/", "Firefox"),
        ("Edg/", "Edge"),
        ("OPR/", "Opera"),
        ("Chrome/", "Chrome"),
        ("Safari/", "Safari"),
    ] {
        if user_agent.contains(mark) {
            return name;
        }
    }
    "Your browser"
}

/// Where the CA's key is, in words.
fn key_text(place: &KeyPlace) -> String {
    match place {
        KeyPlace::Keychain => "Its private key is in the system keychain.".to_owned(),
        KeyPlace::File { why: None } => {
            "Its private key is in ca.key, which only you can read.".to_owned()
        }
        KeyPlace::File { why: Some(why) } => format!(
            "Its private key is in ca.key, which only you can read, as the system keychain \
             cannot take it: {why}."
        ),
    }
}

/// What the check has found, in words, and whether all is well.
fn check_result(visit: Option<&CheckVisit>) -> (String, bool) {
    match visit {
        None => (
            "Open the page. If the browser warns about the certificate, or cannot find the \
             page, one of the steps above is not done for that browser."
                .to_owned(),
            false,
        ),
        Some(visit) => {
            let name = browser_name(&visit.user_agent);
            if visit.secure {
                (
                    format!("{name} goes through tracewing and trusts it."),
                    true,
                )
            } else {
                (
                    format!(
                        "{name} goes through tracewing, but opened the page without HTTPS. \
                         Open it again over https://."
                    ),
                    false,
                )
            }
        }
    }
}

fn button(id: &'static str, label: impl Into<SharedString>, loud: bool) -> Stateful<Div> {
    let c = colors();
    div()
        .id(id)
        .flex_shrink_0()
        .h(px(36.))
        .px(px(16.))
        .rounded(px(18.))
        .flex()
        .items_center()
        .cursor_pointer()
        .font_weight(FontWeight::BOLD)
        .when(loud, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
        .when(!loud, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
        .child(label.into())
}

/// One step: its number or a tick, what it is, where it stands, and what can be done.
fn step(number: &'static str, done: bool, title: &'static str, text: String) -> Div {
    let c = colors();
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(16.))
        .p(px(18.))
        .rounded(px(22.))
        .bg(rgb(c.surface))
        .child(
            div()
                .size(px(32.))
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
                .min_w(px(280.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::BOLD)
                        .child(title),
                )
                .child(div().text_color(rgb(c.muted)).child(text)),
        )
}

/// What another device is told to do, and whether it can yet.
fn other_devices_text(on: bool, port: u16, here: Option<IpAddr>) -> String {
    if !on {
        return "A phone or another computer can send its traffic here too. Turn on \
                \u{201c}Let other devices use the proxy\u{201d} in Settings, then start \
                tracewing again."
            .to_owned();
    }
    let addr = match here {
        Some(ip) => format!("{}", SocketAddr::new(ip, port)),
        None => format!("this computer's address, port {port}"),
    };
    format!(
        "On the device, set the Wi-Fi proxy to {addr}. Then open http://{CHECK_HOST} there, get \
         the certificate and trust it. Its flows show its address, and can be filtered with \
         device:. Name it from one of them, in Connection."
    )
}

/// Another way, beside the steps: what it is, and what can be done.
fn aside(title: &'static str, text: impl Into<SharedString>) -> Div {
    let c = colors();
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(10.))
        .p(px(18.))
        .rounded(px(22.))
        .border_1()
        .border_color(rgb(c.surface))
        .child(
            div()
                .flex_1()
                .min_w(px(280.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(15.))
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
            CaState::Checking => "Looking for it…".to_owned(),
            CaState::Installing => {
                "Installing. The system may ask you to confirm.".to_owned()
            }
            CaState::Trusted if self.system.ca_places.is_empty() => {
                "This computer trusts it.".to_owned()
            }
            CaState::Trusted => format!(
                "Installed in: {}.",
                self.system
                    .ca_places
                    .iter()
                    .map(|place| place.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            // Linux: in the system's store, which browsers do not read.
            CaState::NotTrusted if !self.system.ca_places.is_empty() => {
                "Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.".to_owned()
            }
            CaState::NotTrusted => {
                "tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.".to_owned()
            }
        };
        let ca_text = match &self.system.ca_key {
            Some(place) => format!("{ca_text} {}", key_text(place)),
            None => ca_text,
        };
        let busy = matches!(ca, CaState::Checking | CaState::Installing);
        let ca_step = step("1", installed, "Trust the certificate", ca_text)
            .when(!installed && !busy, |d| {
                d.child(
                    button("setup-install", "Install", true).on_click(cx.listener(
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
                            "Also for command-line tools",
                            false,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.install_certificate(Scope::Machine, cx);
                        })),
                    )
                },
            );

        let addr = self.engine.addr;
        let (proxy_on, working) = match self.system.proxy {
            ProxySwitch::On(_) => (true, false),
            ProxySwitch::Working => (false, true),
            ProxySwitch::Off => (false, false),
        };
        let proxy_text = if proxy_on {
            format!(
                "Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes."
            )
        } else {
            format!(
                "Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself."
            )
        };
        let proxy_step =
            step("2", proxy_on, "Turn on the system proxy", proxy_text).when(!working, |d| {
                d.child(
                    button(
                        "setup-proxy",
                        if proxy_on { "Turn off" } else { "Turn on" },
                        !proxy_on,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_system_proxy(cx))),
                )
            });

        let (check_text, checked) = check_result(self.check_visit.as_ref());
        let check_step = step("3", checked, "Check it in your browser", check_text).child(
            button("setup-check", "Open the test page", !checked).on_click(
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
            .gap(px(10.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(px(10.))
                    .pb(px(4.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(280.))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(20.))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Set up"),
                            )
                            .child(
                                div()
                                    .text_color(rgb(c.muted))
                                    .child("Three steps to see HTTPS traffic from this computer."),
                            ),
                    )
                    .child(
                        button("setup-done", "See traffic", checked).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.set_screen(Screen::Traffic, cx);
                            },
                        )),
                    ),
            )
            .child(ca_step)
            .child(proxy_step)
            .child(check_step)
            .child(self.render_one_app(cx))
            .child(self.render_other_devices(cx))
            .child(self.render_http3(cx))
    }
}

/// What blocking UDP port 443 does, before and while it is on.
fn http3_text(on: bool) -> &'static str {
    if on {
        "UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 \
         and go through tracewing. It stops when tracewing closes, or if it crashes."
    } else {
        "Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 \
         on this computer makes them fall back to HTTP/2 through tracewing. It takes an \
         administrator's password, and stops when tracewing closes. A VPN that uses UDP \
         port 443 stops working meanwhile."
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
        aside("Apps on HTTP/3", http3_text(on)).when(!working, |d| {
            d.child(
                button(
                    "setup-quic",
                    if on { "Stop blocking" } else { "Block HTTP/3" },
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
        let browser = self.system.browser.as_ref().map(|b| b.name);
        aside(
            "Or one app at a time",
            "A terminal, or a browser with a profile of its own, that goes through tracewing \
             while the system proxy and every other app are left as they are.",
        )
        .child(
            button("setup-terminal", "Open a terminal", false)
                .on_click(cx.listener(|this, _, _, cx| this.open_terminal(cx))),
        )
        .when_some(browser, |d, name| {
            d.child(
                button("setup-browser", format!("Open {name}"), false)
                    .on_click(cx.listener(|this, _, _, cx| this.open_browser(cx))),
            )
        })
    }
}

impl Traffic {
    /// A phone or another computer, through the proxy here.
    fn render_other_devices(&mut self, cx: &mut Context<Self>) -> Div {
        let on = self.engine.settings.other_devices;
        let text = other_devices_text(
            on,
            self.engine.addr.port(),
            on.then(on_the_network).flatten(),
        );
        aside("Other devices", text).when(!on, |d| {
            d.child(
                button("setup-devices", "Open settings", false)
                    .on_click(cx.listener(|this, _, _, cx| this.set_screen(Screen::Settings, cx))),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert_eq!(browser_name(agent), name);
        }
    }

    #[test]
    fn the_check_says_what_it_found() {
        assert!(!check_result(None).1);
        let plain = CheckVisit {
            user_agent: "Firefox/144.0".into(),
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
