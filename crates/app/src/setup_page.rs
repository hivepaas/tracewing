//! The first thing a new user needs: HTTPS that works. Three steps, each saying
//! where it stands, and the last one checks for real: a browser opens a page only
//! the proxy can answer, over HTTPS, which proves both that it goes through the
//! proxy and that it trusts the certificate. Nothing else can tell, since browsers
//! keep trust stores of their own.

use crate::{
    colors::colors,
    system::{CaState, ProxySwitch},
    traffic::{Screen, Traffic},
    widgets::icon,
};
use gpui::{Context, Div, FontWeight, SharedString, Stateful, div, prelude::*, px, rgb};
use tracewing_proxy::capture::{CHECK_HOST, CheckVisit};
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
            CaState::Trusted => format!("Installed in: {}.", self.system.ca_places.join(", ")),
            CaState::NotTrusted => {
                "tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.".to_owned()
            }
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
