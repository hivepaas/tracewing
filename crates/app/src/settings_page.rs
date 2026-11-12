//! The settings screen: every storage limit, with the app's default until the user
//! says otherwise. Each change is written to `settings.toml`, the same file a
//! person could edit by hand, as it is made. A few apply at once; the rest
//! from the next start on, and the screen says which are still waiting.

use crate::{
    colors::colors,
    input::TextInput,
    traffic::Traffic,
    widgets::{Reachable, eyebrow},
};
use gpui::{
    Context, Entity, Focusable, FontWeight, KeyDownEvent, Subscription, Window, div, prelude::*,
    px, rgb,
};
use tracewing_i18n::{tr, tr_n};
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
                     So can anyone else on it: leave this off on a network you do not trust."
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
                     on HTTP/2. Flows still show the offers as servers sent them."
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
    /// What `settings.toml` says, as last read or written.
    stored: StorageSettings,
    /// The setting last changed, under which a problem with the fields shows.
    editing: Option<&'static str>,
    pub problem: Option<String>,
    /// What there was before "Back to defaults", while it can be undone.
    undo: Option<StorageSettings>,
    /// What writes a field when it is left. Made at the first draw, which has
    /// the window.
    leaving: Vec<Subscription>,
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
            stored: settings.clone(),
            editing: None,
            problem: None,
            undo: None,
            leaving: Vec::new(),
        }
    }

    /// The settings written that wait for tracewing to start again: those
    /// that differ from what it runs with.
    pub fn waiting(&self, running: &StorageSettings) -> Vec<&'static str> {
        let stored = toml::Value::try_from(&self.stored).expect("settings are plain values");
        let running = toml::Value::try_from(running).expect("settings are plain values");
        self.fields
            .iter()
            .map(|(key, _)| *key)
            .filter(|key| stored.get(key) != running.get(key))
            .collect()
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

    /// Each field is written when it is left, so that what is half typed
    /// never applies.
    fn watch_fields(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = &self.settings_page else {
            return;
        };
        if !page.leaving.is_empty() {
            return;
        }
        let mut fields: Vec<(Option<&'static str>, gpui::FocusHandle)> = page
            .fields
            .iter()
            .filter_map(|(key, field)| match field {
                Field::Number(input) => Some((Some(*key), input.focus_handle(cx))),
                Field::Toggle(_) => None,
            })
            .collect();
        fields.push((None, page.no_decrypt.focus_handle(cx)));
        let leaving = fields
            .into_iter()
            .map(|(key, focus)| {
                cx.on_blur(&focus, window, move |this, _, cx| this.field_left(key, cx))
            })
            .collect();
        if let Some(page) = &mut self.settings_page {
            page.leaving = leaving;
        }
    }

    /// A field was left, or Enter pressed in it: what it says is written.
    fn field_left(&mut self, key: Option<&'static str>, cx: &mut Context<Self>) {
        if let Some(page) = &mut self.settings_page {
            page.editing = key;
        }
        self.save_settings(cx);
    }

    /// The screen is left: what is still unwritten is written. A field that
    /// cannot be goes back to what is written, and the footer says so, rather
    /// than keep showing what does not apply.
    pub fn leave_settings(&mut self, cx: &mut Context<Self>) {
        self.save_settings(cx);
        let Some(page) = &self.settings_page else {
            return;
        };
        if let Some(problem) = page.problem.clone() {
            let stored = page.stored.clone();
            self.settings_left = Some(problem);
            self.settings_page = Some(SettingsPage::new(&stored, cx));
            self.show_rules(cx);
        }
    }

    /// Writes what the fields say, if it can work and is not what is written
    /// already, and applies what applies at once. The hosts are a file of
    /// their own, written whatever the other fields say.
    pub fn save_settings(&mut self, cx: &mut Context<Self>) {
        let path = self.settings_path();
        let Some(page) = &mut self.settings_page else {
            return;
        };
        let rules = HostRules::from_lines(page.no_decrypt.read(cx).text());
        let mut written = None;
        page.problem = None;
        match page.read(cx) {
            Ok(settings) if settings == page.stored => {}
            Ok(settings) => {
                let write = settings
                    .to_toml()
                    .map_err(|e| format!("{e:#}"))
                    .and_then(|text| {
                        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
                    });
                match write {
                    Ok(()) => {
                        page.stored = settings.clone();
                        page.undo = None;
                        written = Some(settings);
                    }
                    Err(problem) => page.problem = Some(problem),
                }
            }
            Err(problem) => page.problem = Some(problem),
        }
        if let Some(settings) = written {
            self.engine.apply_settings(&settings);
        }
        let now = self
            .engine
            .rules
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if rules != now {
            self.change_rules(|now| *now = rules, cx);
        }
        cx.notify();
    }

    /// Turns a switch, and writes it.
    #[doc(hidden)]
    pub fn toggle_setting(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(page) = &mut self.settings_page else {
            return;
        };
        for (name, field) in &mut page.fields {
            if *name == key
                && let Field::Toggle(on) = field
            {
                *on = !*on;
                page.editing = Some(*name);
            }
        }
        self.save_settings(cx);
    }

    /// Every setting back to its default, written, with a way back.
    pub fn reset_settings(&mut self, cx: &mut Context<Self>) {
        let before = self.settings_page.as_ref().map(|page| page.stored.clone());
        self.replace_settings(&StorageSettings::default(), cx);
        if let (Some(page), Some(before)) = (&mut self.settings_page, before)
            && page.stored != before
        {
            page.undo = Some(before);
        }
        cx.notify();
    }

    /// What there was before "Back to defaults", written again.
    pub fn undo_reset_settings(&mut self, cx: &mut Context<Self>) {
        if let Some(before) = self
            .settings_page
            .as_mut()
            .and_then(|page| page.undo.take())
        {
            self.replace_settings(&before, cx);
        }
    }

    /// Shows `settings` in the fields and writes them.
    fn replace_settings(&mut self, settings: &StorageSettings, cx: &mut Context<Self>) {
        let stored = self.settings_page.as_ref().map(|page| page.stored.clone());
        let mut page = SettingsPage::new(settings, cx);
        if let Some(stored) = stored {
            page.stored = stored;
        }
        self.settings_page = Some(page);
        self.show_rules(cx);
        self.save_settings(cx);
    }

    /// Puts the cursor in a number field. For tests.
    #[doc(hidden)]
    pub fn focus_setting(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = &self.settings_page else {
            return;
        };
        for (name, field) in &page.fields {
            if *name == key
                && let Field::Number(input) = field
            {
                window.focus(&input.focus_handle(cx), cx);
            }
        }
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let c = colors();
        if self.settings_page.is_none() {
            self.open_settings(cx);
        }
        self.watch_fields(window, cx);
        let language = self.render_language(cx);
        let protos = self.render_protos(cx);
        let abis = self.render_abis(cx);
        let trusted = self.render_trusted(cx);
        let relay = self.render_relay(cx);
        let page = self.settings_page.as_ref().expect("just made");
        let waiting = page.waiting(&self.engine.settings);
        // A problem shows under the setting being changed, or else at the top.
        let problem_under = page.problem.as_ref().and(page.editing);
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
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.toggle_setting(key, cx)),
                                )
                                .into_any_element()
                        }
                        Field::Number(input) => div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.))
                            .on_key_down(cx.listener(move |this, ev: &KeyDownEvent, _, cx| {
                                if ev.keystroke.key == "enter" {
                                    this.field_left(Some(key), cx);
                                }
                            }))
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
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .flex_wrap()
                                        .items_center()
                                        .gap(px(8.))
                                        .child(
                                            div()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child(row.label),
                                        )
                                        .when(waiting.contains(&key), |d| {
                                            d.child(
                                                div()
                                                    .px(px(8.))
                                                    .rounded(px(8.))
                                                    .text_size(px(11.))
                                                    .bg(rgb(c.warn_ground))
                                                    .text_color(rgb(c.warn))
                                                    .child(tr!("After a restart")),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .text_color(rgb(c.muted))
                                        .child(row.help),
                                )
                                .when(problem_under == Some(key), |d| {
                                    d.children(page.problem.clone().map(|problem| {
                                        div()
                                            .text_size(px(12.))
                                            .text_color(rgb(c.error))
                                            .child(problem)
                                    }))
                                }),
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
        let status = match (&page.problem, problem_under) {
            (Some(problem), None) => Some((problem.clone(), c.error)),
            _ if page.undo.is_some() => {
                Some((tr!("Every setting is back to its default."), c.muted))
            }
            _ if !waiting.is_empty() => Some((
                tr_n!(
                    waiting.len(),
                    "{n} change takes effect when tracewing starts again.",
                    "{n} changes take effect when tracewing starts again."
                ),
                c.warn,
            )),
            _ => None,
        };
        let undo = page.undo.is_some();
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
                    .when(undo, |d| {
                        d.child(
                            button("settings-undo", tr!("Undo"), true).on_click(
                                cx.listener(|this, _, _, cx| this.undo_reset_settings(cx)),
                            ),
                        )
                    })
                    .when(!undo, |d| {
                        d.child(
                            button("settings-defaults", tr!("Back to defaults"), false)
                                .on_click(cx.listener(|this, _, _, cx| this.reset_settings(cx))),
                        )
                    }),
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
                            .child(tr!("One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies at once.")),
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
            .child(relay)
            .child(trusted)
            .child(protos)
            .child(abis)
            .children(groups)
    }

    /// The authorities trusted for servers from the folder, which a restart
    /// takes in.
    fn render_trusted(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let button = |id: &'static str, label: String| {
            div()
                .id(id)
                .h(px(32.))
                .px(px(14.))
                .rounded(px(16.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(px(12.5))
                .font_weight(FontWeight::BOLD)
                .bg(rgb(c.chip))
                .text_color(rgb(c.on_chip))
                .child(label)
        };
        let line = |name: String, file: String, color: u32| {
            div()
                .px(px(6.))
                .flex()
                .flex_row()
                .gap(px(10.))
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(color))
                        .child(name),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(c.muted))
                        .child(file),
                )
        };
        let found = &self.trusted;
        let changed = found.paths() != self.engine.extra_roots;
        let folder = crate::trusted::folder(&self.data_dir);
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(14.))
            .rounded(px(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Authorities trusted for servers")).px(px(6.)))
            .child(div().px(px(6.)).text_size(px(12.)).text_color(rgb(c.muted)).child(tr!(
                "tracewing trusts the servers the public authorities vouch for, and those this computer's do. It trusts these too: a company's own authority, or one of your own, put as a .pem file in {folder}. They are read when tracewing starts.",
                folder = folder.display().to_string()
            )))
            .children(found.good.iter().map(|authority| {
                let file = authority
                    .file
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                line(authority.name.clone(), file, c.text)
            }))
            .children(found.bad.iter().map(|(file, why)| {
                line(
                    file.clone(),
                    tr!("Left out: {why}", why = why.clone()),
                    c.error,
                )
            }))
            .when(found.good.is_empty() && found.bad.is_empty(), |d| {
                d.child(div().px(px(6.)).text_color(rgb(c.muted)).child(tr!("None yet.")))
            })
            .when(changed, |d| {
                d.child(div().px(px(6.)).text_color(rgb(c.warn)).child(tr!(
                    "The folder has changed since tracewing started: restart it to use what it holds now."
                )))
            })
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.))
                    .child(button("trusted-open", tr!("Open the folder")).on_click(cx.listener(
                        move |_, _, _, cx| {
                            let _ = std::fs::create_dir_all(&folder);
                            cx.open_with_system(&folder);
                        },
                    )))
                    .child(button("trusted-read", tr!("Read it again")).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.trusted = crate::trusted::read(&this.data_dir);
                            cx.notify();
                        },
                    ))),
            )
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
