//! The settings screen: every storage limit, with the app's default until the user
//! says otherwise. Writes `settings.toml`, the same file a person could edit by
//! hand; what is in it applies from the next start on.

use crate::{
    colors::colors,
    input::TextInput,
    traffic::Traffic,
    widgets::{Reachable, eyebrow},
};
use gpui::{Context, Entity, FontWeight, Window, div, prelude::*, px, rgb};
use std::path::Path;
use tracewing_i18n::tr;
use tracewing_proxy::rules::HostRules;
use tracewing_store::settings::StorageSettings;

/// One setting. `key` is its name in the file; a test sees that none is forgotten.
struct Row {
    key: &'static str,
    label: String,
    help: String,
    unit: &'static str,
}

fn row(key: &'static str, label: String, help: String, unit: &'static str) -> Row {
    Row {
        key,
        label,
        help,
        unit,
    }
}

/// The settings by group, said in the language in use.
fn groups() -> Vec<(String, Vec<Row>)> {
    vec![
        (
            tr!("Other devices"),
            vec![row(
                "other_devices",
                tr!("Let other devices use the proxy"),
                tr!(
                    "Phones and other computers on your network can then send their traffic here. \
                     So can anyone else on it: leave this off on a network you do not trust. \
                     Takes effect when tracewing starts again."
                ),
                "",
            )],
        ),
        (
            tr!("Servers"),
            vec![row(
                "os_roots",
                tr!("Trust what this computer trusts"),
                tr!(
                    "Servers with a certificate from an authority installed on this computer, such as \
                     a company's own, are reached as this computer's browsers reach them. Off, only \
                     the public authorities are trusted. Takes effect when tracewing starts again."
                ),
                "",
            )],
        ),
        (
            tr!("AI tools"),
            vec![row(
                "ai_secrets",
                tr!("Let AI tools see tokens and cookies"),
                tr!(
                    "AI tools read recorded traffic through tracewing's MCP server. Off, the values of \
                     Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once."
                ),
                "",
            )],
        ),
        (
            tr!("HTTP/3"),
            vec![row(
                "hide_http3",
                tr!("Keep apps off HTTP/3"),
                tr!(
                    "HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around \
                     tracewing. This takes servers' offers of it out of their responses, and apps stay \
                     on HTTP/2. Flows still show the offers as servers sent them. \
                     Takes effect when tracewing starts again."
                ),
                "",
            )],
        ),
        (
            tr!("Sessions"),
            vec![
                row(
                    "session_max_mib",
                    tr!("Size of a session"),
                    tr!("Past this, the oldest flows are dropped."),
                    "MiB",
                ),
                row(
                    "disk_reserve_mib",
                    tr!("Room to leave on the disk"),
                    tr!(
                        "With less than this free, flows are still listed but bodies are not kept."
                    ),
                    "MiB",
                ),
                row(
                    "sessions_kept",
                    tr!("Earlier sessions to keep"),
                    tr!("Older ones are deleted when a new session starts."),
                    "",
                ),
            ],
        ),
        (
            tr!("Bodies"),
            vec![
                row(
                    "body_max_mib",
                    tr!("Most kept of one body"),
                    tr!("It still reaches the client whole."),
                    "MiB",
                ),
                row(
                    "view_max_mib",
                    tr!("Most unpacked for viewing"),
                    tr!("Stops a small download that unpacks to many GB."),
                    "MiB",
                ),
                row(
                    "log_max_kib",
                    tr!("Small body size"),
                    tr!(
                        "Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive."
                    ),
                    "KiB",
                ),
                row(
                    "compress_stored",
                    tr!("Compress plain text on disk"),
                    tr!(
                        "Takes about half the room, and is skipped when traffic is too fast for it."
                    ),
                    "",
                ),
                row(
                    "compress_level",
                    tr!("Compression level"),
                    tr!("1 to 19. Higher is smaller and slower."),
                    "",
                ),
            ],
        ),
        (
            tr!("Search"),
            vec![
                row(
                    "index_urls",
                    tr!("Search URLs"),
                    tr!("Finds any part of a URL as you type. Costs little."),
                    "",
                ),
                row(
                    "index_bodies",
                    tr!("Search bodies"),
                    tr!("Finds words in bodies. Can take as much disk as the bodies do."),
                    "",
                ),
                row(
                    "index_kib_per_body",
                    tr!("Searched part of each body"),
                    tr!("Words past this are not found. More takes more disk."),
                    "KiB",
                ),
            ],
        ),
        (
            tr!("Under heavy traffic"),
            vec![
                row(
                    "queue_budget_mib",
                    tr!("Memory for bodies waiting to be written"),
                    tr!(
                        "Past this, flows are listed without their bodies. Traffic is never held back."
                    ),
                    "MiB",
                ),
                row(
                    "spool_budget_mib",
                    tr!("Memory for large bodies waiting to be written"),
                    tr!("Past this, large bodies stop being kept."),
                    "MiB",
                ),
            ],
        ),
    ]
}

enum Field {
    Toggle(bool),
    Number(Entity<TextInput>),
}

pub struct SettingsPage {
    fields: Vec<(&'static str, Field)>,
    /// Hosts not to decrypt, one a line. Applies at once, unlike the rest.
    no_decrypt: Entity<TextInput>,
    pub problem: Option<String>,
    pub saved: bool,
}

fn value_text(value: &toml::Value) -> String {
    match value {
        toml::Value::Integer(n) => n.to_string(),
        other => other.to_string(),
    }
}

impl SettingsPage {
    pub fn new(settings: &StorageSettings, cx: &mut Context<Traffic>) -> Self {
        let table = toml::Value::try_from(settings).expect("settings are plain values");
        let mut fields = Vec::new();
        for (_, rows) in groups() {
            for row in rows {
                let value = &table[row.key];
                let field = match value {
                    toml::Value::Boolean(on) => Field::Toggle(*on),
                    value => {
                        let text = value_text(value);
                        Field::Number(cx.new(|cx| {
                            let mut input = TextInput::new("", cx);
                            input.set_text(&text, text.len()..text.len(), cx);
                            input
                        }))
                    }
                };
                fields.push((row.key, field));
            }
        }
        let no_decrypt = cx.new(|cx| TextInput::new("", cx).multiline(5, 12));
        Self {
            fields,
            no_decrypt,
            problem: None,
            saved: false,
        }
    }

    /// What the fields say, if it can work.
    fn read(&self, cx: &gpui::App) -> Result<StorageSettings, String> {
        let mut table = toml::Table::new();
        for (key, field) in &self.fields {
            let value = match field {
                Field::Toggle(on) => toml::Value::Boolean(*on),
                Field::Number(input) => {
                    let text = input.read(cx).text().trim().to_owned();
                    let number: i64 = text.parse().map_err(|_| {
                        tr!(
                            "{key}: \"{text}\" is not a whole number",
                            key = *key,
                            text = text.as_str()
                        )
                    })?;
                    if number < 0 {
                        return Err(tr!("{key}: must not be negative", key = *key));
                    }
                    toml::Value::Integer(number)
                }
            };
            table.insert((*key).to_owned(), value);
        }
        let settings: StorageSettings = table.try_into().map_err(|e| format!("{e}"))?;
        settings.check().map_err(|e| format!("{e:#}"))?;
        Ok(settings)
    }

    fn save(&mut self, path: &Path, cx: &gpui::App) {
        self.saved = false;
        self.problem = None;
        let written = self.read(cx).and_then(|settings| {
            let text = settings.to_toml().map_err(|e| format!("{e:#}"))?;
            std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
        });
        match written {
            Ok(()) => self.saved = true,
            Err(problem) => self.problem = Some(problem),
        }
    }
}

impl Traffic {
    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        if self.settings_page.is_none() {
            let shown = StorageSettings::load(&self.settings_path())
                .unwrap_or_else(|_| self.engine.settings.clone());
            self.settings_page = Some(SettingsPage::new(&shown, cx));
            self.show_rules(cx);
        }
        cx.notify();
    }

    /// The list as it stands, in its field.
    fn show_rules(&mut self, cx: &mut Context<Self>) {
        let lines = self
            .engine
            .rules
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .to_lines();
        if let Some(page) = &self.settings_page {
            page.no_decrypt.update(cx, |input, cx| {
                input.set_text(&lines, lines.len()..lines.len(), cx)
            });
        }
    }

    /// Types into the list. For tests.
    #[doc(hidden)]
    pub fn set_no_decrypt(&mut self, text: &str, cx: &mut Context<Self>) {
        if let Some(page) = &self.settings_page {
            page.no_decrypt.update(cx, |input, cx| {
                input.set_text(text, text.len()..text.len(), cx)
            });
        }
    }

    pub fn save_settings(&mut self, cx: &mut Context<Self>) {
        let path = self.settings_path();
        let mut rules = None;
        if let Some(page) = &mut self.settings_page {
            page.save(&path, cx);
            if page.problem.is_none() {
                rules = Some(HostRules::from_lines(page.no_decrypt.read(cx).text()));
            }
        }
        if let Some(rules) = rules {
            self.change_rules(|now| *now = rules, cx);
        }
        cx.notify();
    }

    pub fn reset_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_page = Some(SettingsPage::new(&StorageSettings::default(), cx));
        self.show_rules(cx);
        cx.notify();
    }

    /// Types into a number field. For tests.
    #[doc(hidden)]
    pub fn set_setting(&mut self, key: &str, text: &str, cx: &mut Context<Self>) {
        let Some(page) = &self.settings_page else {
            return;
        };
        for (name, field) in &page.fields {
            if *name == key
                && let Field::Number(input) = field
            {
                input.update(cx, |input, cx| {
                    input.set_text(text, text.len()..text.len(), cx)
                });
            }
        }
    }

    pub(crate) fn render_settings(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let c = colors();
        if self.settings_page.is_none() {
            self.open_settings(cx);
        }
        let language = self.render_language(cx);
        let page = self.settings_page.as_ref().expect("just made");
        let field_of = |key: &str| {
            page.fields
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, field)| field)
                .expect("every row has a field")
        };
        let groups = groups().into_iter().map(|(title, rows)| {
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .p(px(14.))
                .rounded(px(22.))
                .bg(rgb(c.surface))
                .child(eyebrow(title).px(px(6.)).pb(px(6.)))
                .children(rows.into_iter().map(|row| {
                    let key = row.key;
                    let control = match field_of(key) {
                        Field::Toggle(on) => {
                            let on = *on;
                            div()
                                .id(key)
                                .w(px(44.))
                                .h(px(24.))
                                .p(px(3.))
                                .rounded(px(12.))
                                .flex()
                                .cursor_pointer()
                                .reachable()
                                .when(on, |d| d.bg(rgb(c.accent)).justify_end())
                                .when(!on, |d| d.bg(rgb(c.input)))
                                .child(div().size(px(18.)).rounded(px(9.)).bg(rgb(if on {
                                    c.on_accent
                                } else {
                                    c.muted
                                })))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(page) = &mut this.settings_page {
                                        for (name, field) in &mut page.fields {
                                            if *name == key
                                                && let Field::Toggle(on) = field
                                            {
                                                *on = !*on;
                                            }
                                        }
                                        page.saved = false;
                                    }
                                    cx.notify();
                                }))
                                .into_any_element()
                        }
                        Field::Number(input) => div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.))
                            .child(
                                div()
                                    .w(px(96.))
                                    .h(px(32.))
                                    .px(px(10.))
                                    .rounded(px(12.))
                                    .flex()
                                    .items_center()
                                    .bg(rgb(c.input))
                                    .font_family(crate::MONO_FONT)
                                    .font_features(gpui::FontFeatures::disable_ligatures())
                                    .child(input.clone()),
                            )
                            .child(
                                div()
                                    .w(px(28.))
                                    .text_size(px(12.))
                                    .text_color(rgb(c.muted))
                                    .child(row.unit),
                            )
                            .into_any_element(),
                    };
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(16.))
                        .min_h(px(52.))
                        .px(px(6.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(div().font_weight(FontWeight::SEMIBOLD).child(row.label))
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(rgb(c.muted))
                                        .child(row.help),
                                ),
                        )
                        .child(control)
                }))
        });
        let button = |id: &'static str, label: String, loud: bool| {
            div()
                .id(id)
                .h(px(36.))
                .px(px(16.))
                .rounded(px(18.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .font_weight(FontWeight::BOLD)
                .when(loud, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!loud, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .child(label)
        };
        let status = match (&page.problem, page.saved) {
            (Some(problem), _) => Some((problem.clone(), c.error)),
            (None, true) => Some((
                tr!("Saved. The limits apply the next time tracewing starts."),
                c.ok,
            )),
            _ => None,
        };
        div()
            .id("settings")
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
                    .child(
                        div()
                            .text_size(px(20.))
                            .font_weight(FontWeight::BOLD)
                            .child(tr!("Settings")),
                    )
                    .child(div().flex_1())
                    .children(status.map(|(text, color)| div().text_color(rgb(color)).child(text)))
                    .child(
                        button("settings-defaults", tr!("Back to defaults"), false)
                            .on_click(cx.listener(|this, _, _, cx| this.reset_settings(cx))),
                    )
                    .child(
                        button("settings-save", tr!("Save"), true)
                            .on_click(cx.listener(|this, _, _, cx| this.save_settings(cx))),
                    ),
            )
            .child(language)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .p(px(14.))
                    .rounded(px(22.))
                    .bg(rgb(c.surface))
                    .child(eyebrow(tr!("Hosts not to decrypt")).px(px(6.)))
                    .child(
                        div()
                            .px(px(6.))
                            .text_size(px(12.))
                            .text_color(rgb(c.muted))
                            .child(tr!("One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies as soon as it is saved.")),
                    )
                    .child(
                        div()
                            .p(px(10.))
                            .rounded(px(12.))
                            .bg(rgb(c.input))
                            .font_family(crate::MONO_FONT)
.font_features(gpui::FontFeatures::disable_ligatures())
                            .child(page.no_decrypt.clone()),
                    ),
            )
            .children(groups)
    }
}

impl Traffic {
    /// The language: the system's, or one of those there are, each named in
    /// itself so that it is found by someone who cannot read the others.
    fn render_language(&self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let chosen = self.layout.language.clone();
        let chip = |id: String, label: String, on: bool| {
            div()
                .id(gpui::ElementId::Name(id.into()))
                .h(px(30.))
                .px(px(12.))
                .rounded(px(15.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(px(12.5))
                .font_weight(FontWeight::BOLD)
                .when(on, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!on, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .child(label)
        };
        let system = tracewing_i18n::system_language();
        let system_name = tracewing_i18n::LANGUAGES
            .iter()
            .find(|(tag, _)| *tag == system)
            .map_or("English", |(_, name)| name);
        let languages = tracewing_i18n::LANGUAGES.iter().map(|&(tag, name)| {
            chip(
                format!("language-{tag}"),
                name.to_owned(),
                chosen.as_deref() == Some(tag),
            )
            .on_click(
                cx.listener(move |this, _, window, cx| this.set_language(Some(tag), window, cx)),
            )
        });
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(14.))
            .rounded(px(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Language")).px(px(6.)))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(6.))
                    .child(
                        chip(
                            "language-system".to_owned(),
                            tr!("System ({system_name})", system_name = system_name),
                            chosen.is_none(),
                        )
                        .on_click(
                            cx.listener(|this, _, window, cx| this.set_language(None, window, cx)),
                        ),
                    )
                    .children(languages),
            )
            .child(
                div()
                    .px(px(6.))
                    .text_size(px(12.))
                    .text_color(rgb(c.muted))
                    .child(tr!("Applies at once.")),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_setting_has_a_row() {
        let table = toml::Value::try_from(StorageSettings::default()).unwrap();
        let mut keys: Vec<&str> = table
            .as_table()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let mut rows: Vec<&str> = groups()
            .into_iter()
            .flat_map(|(_, rows)| rows.into_iter().map(|row| row.key))
            .collect();
        keys.sort_unstable();
        rows.sort_unstable();
        assert_eq!(keys, rows);
    }
}
