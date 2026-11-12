//! About, the last part of Settings, where the rail's logo leads: what tracewing
//! is and which build this is, where its code is, and the other apps from
//! HivePaaS.

use std::time::Duration;

use gpui::{ClipboardItem, Context, Div, FontWeight, Stateful, div, prelude::*, relative, rgb};
use tracewing_i18n::tr;

use crate::colors::colors;
use crate::layout::SettingsSection;
use crate::text_size::ui;
use crate::traffic::{Screen, Traffic};
use crate::widgets::{Reachable, chip, eyebrow, icon};

const SOURCE: &str = "https://github.com/hivepaas/tracewing";
const LICENSE: &str = "https://github.com/hivepaas/tracewing/blob/main/LICENSE";
const ISSUES: &str = "https://github.com/hivepaas/tracewing/issues/new";
const RELEASES: &str = "https://github.com/hivepaas/tracewing/releases";
const PACKWING: &str = "https://packwing.app";
const PACKWING_SOURCE: &str = "https://github.com/hivepaas/packwing";
const SHELLWING: &str = "https://shellwing.app";
const SHELLWING_SOURCE: &str = "https://github.com/hivepaas/shellwing";
const HIVEPAAS: &str = "https://hivepaas.com";
const HIVEPAAS_SOURCE: &str = "https://github.com/hivepaas/hivepaas";
/// The year in the copyright line.
const YEAR: u32 = 2026;
/// How long Copy version info says it did.
const COPIED_FOR: Duration = Duration::from_secs(2);

/// What a problem report needs to say about this build.
pub fn version_info() -> String {
    format!(
        "tracewing {} ({} {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

impl Traffic {
    /// Opens About, where the rail's logo leads.
    pub fn show_about(&mut self, cx: &mut Context<Self>) {
        self.show_settings_section(SettingsSection::About, cx);
        self.set_screen(Screen::Settings, cx);
    }

    /// Puts the version info on the clipboard, and says so for a moment.
    fn copy_version_info(&mut self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(version_info()));
        self.about_copied = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.about_copied = false;
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn render_about(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .flex_col()
            .gap(ui(10.))
            .child(self.render_identity(cx))
            .child(
                group()
                    .gap(ui(2.))
                    .child(row(
                        tr!("Free software"),
                        tr!("Under the GNU General Public License, version 3 or later"),
                        link("about-license", tr!("View license"), LICENSE, false),
                    ))
                    .child(row(
                        tr!("Made by HivePaaS"),
                        tr!("Built in Rust, on GPUI"),
                        link("about-source", tr!("Source code"), SOURCE, false),
                    ))
                    .child(row(
                        tr!("Something wrong?"),
                        tr!("Tell us, with the version above"),
                        link("about-issue", tr!("Report a problem"), ISSUES, false),
                    ))
                    .child(row(
                        tr!("What changed"),
                        tr!("Every release, and what is new in it"),
                        link("about-releases", tr!("Release notes"), RELEASES, false),
                    )),
            )
            .child(other_apps())
            .child(
                div()
                    .px(ui(6.))
                    .pb(ui(6.))
                    .text_size(ui(12.))
                    .text_color(rgb(colors().muted))
                    .child(tr!(
                        "© {year} HivePaaS. The tracewing name and logo are trademarks of HivePaaS.",
                        year = YEAR.to_string()
                    )),
            )
    }

    /// The logo, the name, what it is and which build this is.
    fn render_identity(&self, cx: &mut Context<Self>) -> Div {
        let c = colors();
        let copy = if self.about_copied {
            tr!("Copied")
        } else {
            tr!("Copy version info")
        };
        group()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap(ui(18.))
            .py(ui(20.))
            .px(ui(20.))
            .child(logo_tile(72., 24., c.accent).child(icon("logo", 38., c.on_accent)))
            .child(
                div()
                    .flex_1()
                    .min_w(ui(220.))
                    .flex()
                    .flex_col()
                    .gap(ui(2.))
                    .child(
                        div()
                            .text_size(ui(28.))
                            .font_weight(FontWeight::BOLD)
                            .line_height(relative(1.2))
                            .child("tracewing"),
                    )
                    .child(div().text_color(rgb(c.muted)).child(tr!(
                        "Version {version} · {os} {arch}",
                        version = env!("CARGO_PKG_VERSION"),
                        os = std::env::consts::OS,
                        arch = std::env::consts::ARCH
                    )))
                    .child(div().pt(ui(6.)).text_size(ui(15.)).child(tr!(
                        "A native HTTP(S) debugging proxy that keeps up with big sessions."
                    ))),
            )
            .child(
                button("about-copy-version", copy, false)
                    .on_click(cx.listener(|this, _, _, cx| this.copy_version_info(cx))),
            )
    }
}

/// A panel of About, as Settings draws its groups.
fn group() -> Div {
    div()
        .flex()
        .flex_col()
        .p(ui(14.))
        .rounded(ui(22.))
        .bg(rgb(colors().surface))
}

/// A line of About: what it is, a word on it, and a way to it.
fn row(label: String, help: String, action: Stateful<Div>) -> Div {
    let c = colors();
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(ui(16.))
        .min_h(ui(52.))
        .px(ui(6.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().font_weight(FontWeight::SEMIBOLD).child(label))
                .child(
                    div()
                        .text_size(ui(12.))
                        .text_color(rgb(c.muted))
                        .child(help),
                ),
        )
        .child(action)
}

/// A button, loud or quiet, as Settings draws them.
fn button(id: &'static str, label: String, loud: bool) -> Stateful<Div> {
    let c = colors();
    div()
        .id(id)
        .h(ui(36.))
        .px(ui(16.))
        .rounded(ui(18.))
        .flex()
        .flex_shrink_0()
        .items_center()
        .cursor_pointer()
        .reachable()
        .font_weight(FontWeight::BOLD)
        .when(loud, |d| d.bg(rgb(c.tonal)).text_color(rgb(c.on_tonal)))
        .when(!loud, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
        .child(label)
}

/// A button that opens a web page.
fn link(id: &'static str, label: String, url: &'static str, loud: bool) -> Stateful<Div> {
    button(id, label, loud).on_click(move |_, _, cx| cx.open_url(url))
}

/// A logo's tile, of its own ground.
fn logo_tile(size: f32, radius: f32, ground: u32) -> Div {
    div()
        .size(ui(size))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(ui(radius))
        .bg(rgb(ground))
}

/// packwing's icon as it draws it: the zipper in white on its green tile, and the
/// zipper's pull, a feather, in its pale orange. An icon is drawn in one color,
/// so the two are icons of their own, one over the other.
fn packwing_logo() -> Div {
    let at = |name: &'static str, color: u32| {
        div()
            .absolute()
            .top_0()
            .left_0()
            .child(icon(name, 36., color))
    };
    logo_tile(56., 16., 0x1f6b4f).child(
        div()
            .relative()
            .size(ui(36.))
            .child(at("packwing", 0xffffff))
            .child(at("packwing-feather", 0xffe1cc)),
    )
}

/// shellwing's icon: the wing on a pale tile, as it draws it, in either look.
fn shellwing_logo() -> Div {
    logo_tile(56., 16., 0xeef1f5).child(icon("shellwing", 36., 0x0284c7))
}

/// HivePaaS's mark, seven cells of a hive in its brown and amber, on a pale tile
/// (the brown does not show on a dark one).
fn hivepaas_logo() -> Div {
    const BROWN: u32 = 0x543d2b;
    const AMBER: u32 = 0xf59e0b;
    // Centers in the mark's 36-unit box.
    const CELLS: [(f32, f32, u32); 7] = [
        (12.98, 9.30, BROWN),
        (23.02, 9.30, AMBER),
        (7.95, 18.0, BROWN),
        (18.0, 18.0, AMBER),
        (28.05, 18.0, AMBER),
        (12.98, 26.70, BROWN),
        (23.02, 26.70, AMBER),
    ];
    logo_tile(56., 16., 0xfbf7f0).child(div().relative().size(ui(36.)).children(CELLS.map(
        |(x, y, color)| {
            div()
                .absolute()
                .left(ui(x - 4.3))
                .top(ui(y - 4.3))
                .size(ui(8.6))
                .rounded_full()
                .bg(rgb(color))
        },
    )))
}

/// An app of the same family: its logo, name and what it is, its strong points,
/// and the ways to it.
fn app(mark: Div, name: &'static str, what: String, points: Vec<String>, ways: Div) -> Div {
    let c = colors();
    div()
        .flex()
        .flex_col()
        .gap(ui(12.))
        .p(ui(14.))
        .rounded(ui(18.))
        .bg(rgb(c.page))
        .child(
            div().flex().items_center().gap(ui(14.)).child(mark).child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(ui(2.))
                    .child(
                        div()
                            .text_size(ui(18.))
                            .font_weight(FontWeight::BOLD)
                            .child(name),
                    )
                    .child(
                        div()
                            .text_size(ui(12.))
                            .text_color(rgb(c.muted))
                            .child(what),
                    ),
            ),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(ui(6.))
                .children(points.into_iter().map(|p| chip(p, c.surface, c.on_chip))),
        )
        .child(ways)
}

/// A site and the code, side by side.
fn ways(
    id: &'static str,
    site: String,
    url: &'static str,
    source_id: &'static str,
    source: &'static str,
) -> Div {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(ui(8.))
        .child(link(id, site, url, true))
        .child(link(source_id, "GitHub".to_owned(), source, false))
}

/// The other apps from HivePaaS: what each is, and where to see more.
fn other_apps() -> Div {
    let c = colors();
    group()
        .gap(ui(12.))
        .child(
            div()
                .px(ui(6.))
                .flex()
                .flex_col()
                .gap(ui(4.))
                .child(eyebrow(tr!("More from HivePaaS")))
                .child(
                    div()
                        .text_size(ui(12.))
                        .text_color(rgb(c.muted))
                        .child(tr!("Free and open source, like tracewing.")),
                ),
        )
        .child(app(
            hivepaas_logo(),
            "HivePaaS",
            tr!(
                "A self-hosted platform for your apps, built on Docker Swarm: an open-source alternative to Heroku, Render and Coolify."
            ),
            vec![
                tr!("300+ one-click apps"),
                tr!("Deploy on push"),
                tr!("Pull request previews"),
                tr!("Automatic TLS"),
                tr!("Live logs and metrics"),
                tr!("Apache-2.0"),
            ],
            ways(
                "about-hivepaas",
                tr!("Visit hivepaas.com"),
                HIVEPAAS,
                "about-hivepaas-source",
                HIVEPAAS_SOURCE,
            ),
        ))
        .child(app(
            shellwing_logo(),
            "shellwing",
            tr!("A fast, native SSH client for Linux, macOS and Windows."),
            vec![
                tr!("SSH and SFTP"),
                tr!("Follows your ssh_config"),
                tr!("A real terminal"),
                tr!("GPL-3.0-or-later"),
            ],
            ways(
                "about-shellwing",
                tr!("Visit shellwing.app"),
                SHELLWING,
                "about-shellwing-source",
                SHELLWING_SOURCE,
            ),
        ))
        .child(app(
            packwing_logo(),
            "packwing",
            tr!("A file archiver that puts every core to work and opens archives safely."),
            vec![
                tr!("ZIP, 7z, RAR and tar"),
                tr!("Every core at work"),
                tr!("Safe by default"),
                tr!("Browse S3, WebDAV and SFTP"),
                tr!("GPL-3.0-or-later"),
            ],
            ways(
                "about-packwing",
                tr!("Visit packwing.app"),
                PACKWING,
                "about-packwing-source",
                PACKWING_SOURCE,
            ),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_info_names_the_build() {
        let info = version_info();
        assert!(info.starts_with(&format!("tracewing {} (", env!("CARGO_PKG_VERSION"))));
        assert!(info.ends_with(&format!("{})", std::env::consts::ARCH)));
    }
}
