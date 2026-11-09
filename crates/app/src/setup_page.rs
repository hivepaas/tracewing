//! The first thing a new user needs: HTTPS that works. Three steps, each saying
//! where it stands, and the last one checks for real: a browser opens a page only
//! the proxy can answer, over HTTPS, which proves both that it goes through the
//! proxy and that it trusts the certificate. Nothing else can tell, since browsers
//! keep trust stores of their own.

use crate::{
    colors::colors,
    devices::on_the_network,
    qr::qr_code,
    system::{CaState, ProxySwitch, QuicSwitch},
    traffic::{Screen, Traffic},
    widgets::icon,
};
use gpui::{
    ClipboardItem, Context, Div, ElementId, FontWeight, SharedString, Stateful, div, prelude::*,
    px, rgb,
};
use std::net::{IpAddr, SocketAddr};
use tracewing_proxy::{
    ca_key::KeyPlace,
    capture::{CHECK_HOST, CheckVisit},
};
use tracewing_system::{
    Scope,
    android::{Device, SetUp, Trust},
};

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

fn button(id: impl Into<ElementId>, label: impl Into<SharedString>, loud: bool) -> Stateful<Div> {
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
            .child(self.render_android(cx))
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Apple => "iPhone and iPad",
            Self::Android => "Android",
            Self::Flutter => "Flutter apps",
        }
    }
}

/// A guide is said in text, with code to copy where code is needed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part {
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
            text(format!(
                "1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {}, port {}.",
                proxy.ip(),
                proxy.port()
            )),
            text(format!(
                "2. Scan the code with the camera, or open {page} in Safari, and get the \
                 certificate. Allow the download."
            )),
            text(
                "3. In Settings, General, VPN & Device Management, choose the tracewing CA \
                 profile and install it."
                    .to_owned(),
            ),
            text(
                "4. In Settings, General, About, Certificate Trust Settings, turn on full trust \
                 for tracewing CA."
                    .to_owned(),
            ),
            text(format!(
                "5. Open https://{CHECK_HOST} in Safari. If it says it works, apps that use the \
                 system's settings go through tracewing."
            )),
        ],
        Guide::Android => vec![
            text(format!(
                "1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to \
                 Manual: host {}, port {}.",
                proxy.ip(),
                proxy.port()
            )),
            text(format!(
                "2. Scan the code, or open {page} in the browser, and get the certificate."
            )),
            text(
                "3. In Settings, Security, Encryption & credentials (the names vary by maker), \
                 choose Install a certificate, then CA certificate, and pick tracewing.crt."
                    .to_owned(),
            ),
            text(
                "4. Since Android 7, apps trust certificates the user added only if they say \
                 so. For a debug build of your own app, add res/xml/network_security_config.xml:"
                    .to_owned(),
            ),
            Part::Code(ANDROID_CONFIG.to_owned()),
            text("and point to it from AndroidManifest.xml:".to_owned()),
            Part::Code(ANDROID_MANIFEST.to_owned()),
            text(
                "Other apps keep refusing the certificate: their traffic shows as failed flows."
                    .to_owned(),
            ),
        ],
        Guide::Flutter => vec![
            text(
                "Dart's HttpClient, which Flutter's http and Dio use, does not follow the \
                 phone's proxy setting, and on Android does not trust certificates the user \
                 added. In a debug build, send it here:"
                    .to_owned(),
            ),
            Part::Code(flutter_overrides(proxy)),
            text(
                "badCertificateCallback lets any certificate through, which is why it is for \
                 debug builds only."
                    .to_owned(),
            ),
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
        .px(px(14.))
        .py(px(10.))
        .rounded(px(12.))
        .bg(rgb(c.input))
        .font_family(crate::MONO_FONT)
        .font_features(gpui::FontFeatures::disable_ligatures())
        .text_size(px(12.))
        .child(code)
        .child(
            div()
                .id(id.into())
                .absolute()
                .top(px(8.))
                .right(px(8.))
                .px(px(10.))
                .py(px(3.))
                .rounded(px(10.))
                .cursor_pointer()
                .font_family(crate::FONT)
                .bg(rgb(c.chip))
                .text_color(rgb(c.on_chip))
                .child("Copy")
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
            return aside("Other devices", text).when(!on, |d| {
                d.child(
                    button("setup-devices", "Open settings", false).on_click(
                        cx.listener(|this, _, _, cx| this.set_screen(Screen::Settings, cx)),
                    ),
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
                .px(px(14.))
                .py(px(6.))
                .rounded(px(16.))
                .cursor_pointer()
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
                    Part::Text(text) => div().child(text),
                    Part::Code(code) => code_block(("copy-guide", at), code),
                });
        div()
            .flex()
            .flex_col()
            .gap(px(14.))
            .p(px(18.))
            .rounded(px(22.))
            .border_1()
            .border_color(rgb(c.surface))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(px(18.))
                    .children(qr_code(&page, 168.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(280.))
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Phones and other devices"),
                            )
                            .child(div().text_color(rgb(c.muted)).child(format!(
                                "A phone, a tablet or another computer on this network can go \
                                 through tracewing at {proxy}. The code opens the page with its \
                                 certificate. Its flows show its address, can be filtered with \
                                 device:, and can be named from one of them, in Connection."
                            ))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(4.))
                    .children(tabs),
            )
            .child(div().flex().flex_col().gap(px(8.)).children(parts))
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
        ) => "Set up. The certificate is among the system's until the device restarts.".to_owned(),
        (_, Some(SetUp { sdk, .. })) => {
            let mut note = "Set up. The certificate is in Downloads: in the Settings just opened, \
                            choose Encryption & credentials, Install a certificate, CA \
                            certificate, and pick tracewing.crt."
                .to_owned();
            if *sdk >= 34 {
                note.push_str(
                    " Apps that trust only the system's certificates will still refuse it.",
                );
            }
            note
        }
        ("device", None) => if device.emulator {
            "An emulator."
        } else {
            "On USB."
        }
        .to_owned(),
        ("unauthorized", None) => {
            "Accept the USB debugging prompt on the device, then look again.".to_owned()
        }
        ("offline", None) => "Still starting. Look again in a moment.".to_owned(),
        (state, None) => format!("adb says it is {state}."),
    }
}

impl Traffic {
    /// Emulators and phones on USB, set up over ADB in one go.
    fn render_android(&mut self, cx: &mut Context<Self>) -> Div {
        let c = colors();
        let state = &self.system.android;
        let text = match &state.adb {
            None => "Sets the proxy on an Android emulator, or a phone plugged in with USB \
                     debugging on, and installs the certificate. The traffic comes over ADB, so \
                     neither has to be on this network."
                .to_owned(),
            Some(None) => "adb was not found. Install Android Studio, or Android's platform \
                           tools, then look again."
                .to_owned(),
            Some(Some(_)) if state.devices.is_empty() => {
                "No device is connected. Start an emulator, or plug in a phone with USB \
                 debugging on, then look again."
                    .to_owned()
            }
            Some(Some(_)) => "Set up a device to send its traffic here. Undo, or closing \
                              tracewing, takes the proxy off it again."
                .to_owned(),
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
                    .gap(px(12.))
                    .py(px(6.))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(260.))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!("{} · {}", device.model, device.serial)),
                            )
                            .child(div().text_color(rgb(c.muted)).child(if working {
                                "Working…".to_owned()
                            } else {
                                android_note(device, set)
                            })),
                    )
                    .when(device.ready() && !busy, |d| {
                        let label = if set.is_some() { "Undo" } else { "Set up" };
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
        aside("Android emulators and USB devices", text)
            .when(!looking, |d| {
                d.child(
                    button(
                        "setup-android",
                        if looked { "Look again" } else { "Find devices" },
                        false,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.find_android_devices(cx))),
                )
            })
            .child(div().w_full().flex().flex_col().children(rows))
            .children(problem.map(|problem| div().w_full().text_color(rgb(c.error)).child(problem)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                    Part::Text(t) | Part::Code(t) => t,
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
