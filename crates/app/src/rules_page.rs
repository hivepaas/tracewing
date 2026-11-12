//! The rules screen: what changes traffic on its way. They apply from the next
//! request, and a flow says which rules touched it.

use crate::{
    colors::colors,
    input::TextInput,
    traffic::Traffic,
    widgets::{Reachable, eyebrow},
};
use gpui::{
    AppContext as _, Context, Div, Entity, FontWeight, PathPromptOptions, Stateful, div,
    prelude::*, px, rgb,
};
use std::path::{Path, PathBuf};
use tracewing_i18n::{tr, tr_n};
use tracewing_proxy::traffic_rules::{Action, Imported, TrafficRule, TrafficRules, export_set};

/// What a rule does, as the form asks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleKind {
    Block,
    MapLocal,
    MapRemote,
    RequestHeader,
    ResponseHeader,
    RemoveRequestHeader,
    RemoveResponseHeader,
    SetQuery,
    RemoveQuery,
    RequestBody,
    ResponseBody,
    Status,
    PauseRequest,
    PauseResponse,
    PauseMessagesToServer,
    PauseMessagesToApp,
}

impl RuleKind {
    pub const ALL: [Self; 16] = [
        Self::Block,
        Self::MapLocal,
        Self::MapRemote,
        Self::RequestHeader,
        Self::ResponseHeader,
        Self::RemoveRequestHeader,
        Self::RemoveResponseHeader,
        Self::SetQuery,
        Self::RemoveQuery,
        Self::RequestBody,
        Self::ResponseBody,
        Self::Status,
        Self::PauseRequest,
        Self::PauseResponse,
        Self::PauseMessagesToServer,
        Self::PauseMessagesToApp,
    ];

    /// Its second field is a body, written over several lines.
    pub fn takes_a_body(self) -> bool {
        matches!(self, Self::RequestBody | Self::ResponseBody)
    }

    pub fn label(self) -> String {
        match self {
            Self::Block => tr!("Block"),
            Self::MapLocal => tr!("Answer from a file"),
            Self::MapRemote => tr!("Send to another host"),
            Self::RequestHeader => tr!("Set a request header"),
            Self::ResponseHeader => tr!("Set a response header"),
            Self::RemoveRequestHeader => tr!("Remove a request header"),
            Self::RemoveResponseHeader => tr!("Remove a response header"),
            Self::SetQuery => tr!("Set a query parameter"),
            Self::RemoveQuery => tr!("Remove a query parameter"),
            Self::RequestBody => tr!("Change the request body"),
            Self::ResponseBody => tr!("Change the response body"),
            Self::Status => tr!("Change the status"),
            Self::PauseRequest => tr!("Pause the request"),
            Self::PauseResponse => tr!("Pause the response"),
            Self::PauseMessagesToServer => tr!("Pause a socket's messages to the server"),
            Self::PauseMessagesToApp => tr!("Pause a socket's messages to the app"),
        }
    }

    /// What the form's fields ask for, as many as it needs.
    fn fields(self) -> (Option<String>, Option<String>) {
        match self {
            Self::Block => (Some(tr!("Status to answer with, like 403")), None),
            Self::MapLocal => (Some(tr!("Full path of a file or folder")), None),
            Self::MapRemote => (Some(tr!("Where to, like http://localhost:3000")), None),
            Self::RequestHeader | Self::ResponseHeader => {
                (Some(tr!("Header's name")), Some(tr!("Its value")))
            }
            Self::RemoveRequestHeader | Self::RemoveResponseHeader => {
                (Some(tr!("Header's name")), None)
            }
            Self::SetQuery => (Some(tr!("Parameter's name")), Some(tr!("Its value"))),
            Self::RemoveQuery => (Some(tr!("Parameter's name")), None),
            Self::RequestBody | Self::ResponseBody => (
                Some(tr!("Text to find, or empty for the whole body")),
                Some(tr!("What to put in its place")),
            ),
            Self::Status => (Some(tr!("Status the client gets, like 500")), None),
            Self::PauseRequest
            | Self::PauseResponse
            | Self::PauseMessagesToServer
            | Self::PauseMessagesToApp => (None, None),
        }
    }

    fn action(self, first: &str, second: &str) -> Result<Action, String> {
        let untrimmed = first;
        let first = first.trim();
        Ok(match self {
            Self::Block => Action::Block {
                status: first
                    .parse()
                    .map_err(|_| tr!("\"{first}\" is not a status", first = first))?,
            },
            Self::MapLocal => Action::MapLocal {
                path: PathBuf::from(first),
            },
            Self::MapRemote => Action::MapRemote {
                to: first.to_owned(),
            },
            Self::RequestHeader => Action::SetRequestHeader {
                header: first.to_owned(),
                value: second.to_owned(),
            },
            Self::ResponseHeader => Action::SetResponseHeader {
                header: first.to_owned(),
                value: second.to_owned(),
            },
            Self::RemoveRequestHeader => Action::RemoveRequestHeader {
                header: first.to_owned(),
            },
            Self::RemoveResponseHeader => Action::RemoveResponseHeader {
                header: first.to_owned(),
            },
            Self::SetQuery => Action::SetQuery {
                name: first.to_owned(),
                value: second.to_owned(),
            },
            Self::RemoveQuery => Action::RemoveQuery {
                name: first.to_owned(),
            },
            // What to find is taken as typed: spaces may be what is looked for.
            Self::RequestBody => Action::ReplaceRequestBody {
                find: untrimmed.to_owned(),
                replace: second.to_owned(),
            },
            Self::ResponseBody => Action::ReplaceResponseBody {
                find: untrimmed.to_owned(),
                replace: second.to_owned(),
            },
            Self::Status => Action::SetStatus {
                status: first
                    .parse()
                    .map_err(|_| tr!("\"{first}\" is not a status", first = first))?,
            },
            Self::PauseRequest => Action::PauseRequest,
            Self::PauseResponse => Action::PauseResponse,
            Self::PauseMessagesToServer => Action::PauseMessagesToServer,
            Self::PauseMessagesToApp => Action::PauseMessagesToApp,
        })
    }
}

/// What a rule does, in words, for the list.
fn action_text(action: &Action) -> String {
    match action {
        Action::Block { status } => tr!("Block with {status}", status = *status),
        Action::MapLocal { path } => {
            tr!("Answer from {path}", path = path.display().to_string())
        }
        Action::MapRemote { to } => tr!("Send to {to}", to = to),
        Action::SetRequestHeader { header, value } => {
            tr!("Request: {header}: {value}", header = header, value = value)
        }
        Action::SetResponseHeader { header, value } => {
            tr!(
                "Response: {header}: {value}",
                header = header,
                value = value
            )
        }
        Action::RemoveRequestHeader { header } => tr!("Request: no {header}", header = header),
        Action::RemoveResponseHeader { header } => {
            tr!("Response: no {header}", header = header)
        }
        Action::SetQuery { name, value } => {
            tr!("Query: {name}={value}", name = name, value = value)
        }
        Action::RemoveQuery { name } => tr!("Query: no {name}", name = name),
        Action::ReplaceRequestBody { find, replace } => body_text(true, find, replace),
        Action::ReplaceResponseBody { find, replace } => body_text(false, find, replace),
        Action::SetStatus { status } => tr!("Status {status}", status = *status),
        Action::PauseRequest => tr!("Pause the request"),
        Action::PauseResponse => tr!("Pause the response"),
        Action::PauseMessagesToServer => tr!("Pause a socket's messages to the server"),
        Action::PauseMessagesToApp => tr!("Pause a socket's messages to the app"),
    }
}

/// A body rule in words, its texts cut to a line.
fn body_text(request: bool, find: &str, replace: &str) -> String {
    let short = |text: &str| {
        let line = text.lines().next().unwrap_or_default();
        let cut: String = line.chars().take(40).collect();
        if cut.len() < text.len() {
            format!("{cut}…")
        } else {
            cut
        }
    };
    match (request, find.is_empty()) {
        (true, true) => tr!("Request body: {replace}", replace = short(replace)),
        (false, true) => tr!("Response body: {replace}", replace = short(replace)),
        (true, false) => tr!(
            "Request body: {find} → {replace}",
            find = short(find),
            replace = short(replace)
        ),
        (false, false) => tr!(
            "Response body: {find} → {replace}",
            find = short(find),
            replace = short(replace)
        ),
    }
}

/// The form's fields for a rule being edited.
fn fields_of(action: &Action) -> (RuleKind, String, String) {
    match action {
        Action::Block { status } => (RuleKind::Block, status.to_string(), String::new()),
        Action::MapLocal { path } => (
            RuleKind::MapLocal,
            path.display().to_string(),
            String::new(),
        ),
        Action::MapRemote { to } => (RuleKind::MapRemote, to.clone(), String::new()),
        Action::SetRequestHeader { header, value } => {
            (RuleKind::RequestHeader, header.clone(), value.clone())
        }
        Action::SetResponseHeader { header, value } => {
            (RuleKind::ResponseHeader, header.clone(), value.clone())
        }
        Action::RemoveRequestHeader { header } => {
            (RuleKind::RemoveRequestHeader, header.clone(), String::new())
        }
        Action::RemoveResponseHeader { header } => (
            RuleKind::RemoveResponseHeader,
            header.clone(),
            String::new(),
        ),
        Action::SetQuery { name, value } => (RuleKind::SetQuery, name.clone(), value.clone()),
        Action::RemoveQuery { name } => (RuleKind::RemoveQuery, name.clone(), String::new()),
        Action::ReplaceRequestBody { find, replace } => {
            (RuleKind::RequestBody, find.clone(), replace.clone())
        }
        Action::ReplaceResponseBody { find, replace } => {
            (RuleKind::ResponseBody, find.clone(), replace.clone())
        }
        Action::SetStatus { status } => (RuleKind::Status, status.to_string(), String::new()),
        Action::PauseRequest => (RuleKind::PauseRequest, String::new(), String::new()),
        Action::PauseResponse => (RuleKind::PauseResponse, String::new(), String::new()),
        Action::PauseMessagesToServer => (
            RuleKind::PauseMessagesToServer,
            String::new(),
            String::new(),
        ),
        Action::PauseMessagesToApp => (RuleKind::PauseMessagesToApp, String::new(), String::new()),
    }
}

pub struct RulesPage {
    pub kind: RuleKind,
    pub name: Entity<TextInput>,
    pub url: Entity<TextInput>,
    pub method: Entity<TextInput>,
    pub first: Entity<TextInput>,
    pub second: Entity<TextInput>,
    /// The second field for a rule that changes a body, over several lines.
    pub body: Entity<TextInput>,
    /// The kind, for the fields' placeholders to say what they want.
    shown: std::rc::Rc<std::cell::Cell<RuleKind>>,
    /// The rule the form is changing, or none for a new one.
    pub editing: Option<usize>,
    pub problem: Option<String>,
    /// What the last import or export did, and whether it failed.
    pub note: Option<(String, bool)>,
}

impl RulesPage {
    fn new(cx: &mut Context<Traffic>) -> Self {
        let field = |hint: fn() -> String, cx: &mut Context<Traffic>| {
            cx.new(|cx| TextInput::said(hint, cx))
        };
        let shown = std::rc::Rc::new(std::cell::Cell::new(RuleKind::Block));
        let hint = |second: bool, cx: &mut Context<Traffic>| {
            let shown = std::rc::Rc::clone(&shown);
            cx.new(move |cx| {
                TextInput::said(
                    move || {
                        let (first, other) = shown.get().fields();
                        if second { other } else { first }.unwrap_or_default()
                    },
                    cx,
                )
            })
        };
        let (first, second) = (hint(false, cx), hint(true, cx));
        let body = {
            let shown = std::rc::Rc::clone(&shown);
            cx.new(move |cx| {
                TextInput::said(move || shown.get().fields().1.unwrap_or_default(), cx)
                    .multiline(3, 12)
            })
        };
        Self {
            kind: RuleKind::Block,
            name: field(|| tr!("Name"), cx),
            url: field(|| tr!("URL, like api.myapp.dev/v1/*"), cx),
            method: field(|| tr!("Method, or empty for all"), cx),
            first,
            second,
            body,
            shown,
            editing: None,
            problem: None,
            note: None,
        }
    }
}

fn set(input: &Entity<TextInput>, text: &str, cx: &mut Context<Traffic>) {
    input.update(cx, |input, cx| {
        input.set_text(text, text.len()..text.len(), cx)
    });
}

impl Traffic {
    fn rules_form(&mut self, cx: &mut Context<Self>) -> &mut RulesPage {
        if self.rules_page.is_none() {
            self.rules_page = Some(RulesPage::new(cx));
        }
        self.rules_page.as_mut().expect("just made")
    }

    fn traffic_rules_path(&self) -> PathBuf {
        self.data_dir.join("traffic-rules.toml")
    }

    pub fn traffic_rules(&self) -> TrafficRules {
        self.engine
            .traffic_rules
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Reads the rules' file again if something else has changed it since it
    /// was last read or written here. True if it did.
    pub(crate) fn read_rules_again(&mut self) -> bool {
        let path = self.traffic_rules_path();
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if modified == self.rules_seen {
            return false;
        }
        self.rules_seen = modified;
        // A file half written, or one that cannot be read, leaves them as they are.
        let Ok(rules) = TrafficRules::load(&path) else {
            return false;
        };
        let mut held = self
            .engine
            .traffic_rules
            .write()
            .unwrap_or_else(|e| e.into_inner());
        if *held == rules {
            return false;
        }
        *held = rules;
        true
    }

    fn change_traffic_rules(
        &mut self,
        change: impl FnOnce(&mut TrafficRules),
        cx: &mut Context<Self>,
    ) {
        // On what the file says now, so as not to undo what another changed.
        self.read_rules_again();
        let rules = {
            let mut rules = self
                .engine
                .traffic_rules
                .write()
                .unwrap_or_else(|e| e.into_inner());
            change(&mut rules);
            rules.clone()
        };
        if let Err(e) = rules.save(&self.traffic_rules_path()) {
            self.rules_form(cx).problem = Some(format!("{e:#}"));
        }
        self.rules_seen = std::fs::metadata(self.traffic_rules_path())
            .and_then(|m| m.modified())
            .ok();
        cx.notify();
    }

    fn rules_note(&mut self, note: String, failed: bool, cx: &mut Context<Self>) {
        self.rules_form(cx).note = Some((note, failed));
        cx.notify();
    }

    /// Asks where, then writes every rule there, for someone else to import.
    pub fn ask_export_rules(&mut self, cx: &mut Context<Self>) {
        let chosen =
            cx.prompt_for_new_path(&crate::files::save_dir(), Some("tracewing-rules.toml"));
        cx.spawn(async move |this, cx| {
            let _ = match chosen.await {
                Ok(Ok(Some(path))) => this.update(cx, |this, cx| this.export_rules_to(&path, cx)),
                Ok(Err(e)) => this.update(cx, |this, cx| {
                    this.rules_note(tr!("No file picker: {e}", e = format!("{e:#}")), true, cx)
                }),
                _ => return,
            };
        })
        .detach();
    }

    pub fn export_rules_to(&mut self, path: &Path, cx: &mut Context<Self>) {
        let rules = self.traffic_rules().rules;
        let written = export_set(&rules)
            .and_then(|text| std::fs::write(path, text).map_err(anyhow::Error::from));
        match written {
            Ok(()) => self.rules_note(
                tr_n!(
                    rules.len(),
                    "{n} rule written to {path}",
                    "{n} rules written to {path}",
                    path = path.display().to_string()
                ),
                false,
                cx,
            ),
            Err(e) => self.rules_note(
                tr!("The rules were not written: {e}", e = format!("{e:#}")),
                true,
                cx,
            ),
        }
    }

    /// Asks for a rules file, then adds its rules after those here.
    pub fn ask_import_rules(&mut self, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(tr!("Import").into()),
        });
        cx.spawn(async move |this, cx| {
            let _ = match chosen.await {
                Ok(Ok(Some(mut paths))) if !paths.is_empty() => {
                    let path = paths.swap_remove(0);
                    this.update(cx, |this, cx| this.import_rules_from(&path, cx))
                }
                Ok(Err(e)) => this.update(cx, |this, cx| {
                    this.rules_note(tr!("No file picker: {e}", e = format!("{e:#}")), true, cx)
                }),
                _ => return,
            };
        })
        .detach();
    }

    pub fn import_rules_from(&mut self, path: &Path, cx: &mut Context<Self>) {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => {
                let note = tr!(
                    "{path} could not be read: {e}",
                    path = path.display().to_string(),
                    e = e.to_string()
                );
                return self.rules_note(note, true, cx);
            }
        };
        let mut result = None;
        self.change_traffic_rules(|rules| result = Some(rules.import_set(&text)), cx);
        match result.expect("the change ran") {
            Ok(imported) => {
                let failed = imported.added == 0 && !imported.refused.is_empty();
                self.rules_note(import_note(&imported), failed, cx);
            }
            Err(e) => self.rules_note(
                tr!("Nothing was imported: {e}", e = format!("{e:#}")),
                true,
                cx,
            ),
        }
    }

    /// Fills in the form to make a rule for a URL seen in the list.
    pub fn start_rule_for(&mut self, url: &str, cx: &mut Context<Self>) {
        let pattern = url
            .split_once("://")
            .map_or(url, |(_, rest)| rest)
            .to_owned();
        let pattern = pattern.split('?').next().unwrap_or("").to_owned();
        let page = self.rules_form(cx);
        page.editing = None;
        page.problem = None;
        let (name, url_field, first, second) = (
            page.name.clone(),
            page.url.clone(),
            page.first.clone(),
            page.second.clone(),
        );
        set(&name, "", cx);
        set(&url_field, &pattern, cx);
        set(&first, "", cx);
        set(&second, "", cx);
        self.set_screen(crate::traffic::Screen::Rules, cx);
    }

    pub fn set_rule_kind(&mut self, kind: RuleKind, cx: &mut Context<Self>) {
        let page = self.rules_form(cx);
        page.kind = kind;
        page.shown.set(kind);
        cx.notify();
    }

    #[doc(hidden)]
    pub fn data_dir_for_tests(&self) -> &std::path::Path {
        &self.data_dir
    }

    /// Types into the form. For tests.
    #[doc(hidden)]
    pub fn fill_rule(
        &mut self,
        name: &str,
        url: &str,
        first: &str,
        second: &str,
        cx: &mut Context<Self>,
    ) {
        let page = self.rules_form(cx);
        let inputs = [
            (page.name.clone(), name),
            (page.url.clone(), url),
            (page.first.clone(), first),
            (page.second.clone(), second),
            (page.body.clone(), second),
        ];
        for (input, text) in inputs {
            set(&input, text, cx);
        }
    }

    /// Adds the form's rule, or changes the one being edited.
    pub fn save_rule(&mut self, cx: &mut Context<Self>) {
        let page = self.rules_form(cx);
        let read = |input: &Entity<TextInput>| input.read(cx).text().to_owned();
        let (name, url, method, first) = (
            read(&page.name),
            read(&page.url),
            read(&page.method),
            read(&page.first),
        );
        let second = read(if page.kind.takes_a_body() {
            &page.body
        } else {
            &page.second
        });
        let (kind, editing) = (page.kind, page.editing);
        let made = kind.action(&first, &second).and_then(|action| {
            let rule = TrafficRule {
                name: name.trim().to_owned(),
                enabled: true,
                url: url.trim().to_owned(),
                method: method.trim().to_owned(),
                action,
            };
            rule.check().map_err(|e| format!("{e:#}"))?;
            Ok(rule)
        });
        match made {
            Err(problem) => self.rules_form(cx).problem = Some(problem),
            Ok(rule) => {
                self.change_traffic_rules(
                    |rules| match editing.and_then(|at| rules.rules.get_mut(at)) {
                        Some(old) => {
                            let enabled = old.enabled;
                            *old = rule;
                            old.enabled = enabled;
                        }
                        None => rules.rules.push(rule),
                    },
                    cx,
                );
                self.clear_rule_form(cx);
            }
        }
        cx.notify();
    }

    pub fn clear_rule_form(&mut self, cx: &mut Context<Self>) {
        let page = self.rules_form(cx);
        page.editing = None;
        page.problem = None;
        let fields = [
            page.name.clone(),
            page.url.clone(),
            page.method.clone(),
            page.first.clone(),
            page.second.clone(),
            page.body.clone(),
        ];
        for input in fields {
            set(&input, "", cx);
        }
        cx.notify();
    }

    pub fn edit_rule(&mut self, at: usize, cx: &mut Context<Self>) {
        let Some(rule) = self.traffic_rules().rules.get(at).cloned() else {
            return;
        };
        let (kind, first, second) = fields_of(&rule.action);
        let page = self.rules_form(cx);
        page.kind = kind;
        page.shown.set(kind);
        page.editing = Some(at);
        page.problem = None;
        let other = if kind.takes_a_body() {
            page.body.clone()
        } else {
            page.second.clone()
        };
        let inputs = [
            (page.name.clone(), rule.name),
            (page.url.clone(), rule.url),
            (page.method.clone(), rule.method),
            (page.first.clone(), first),
            (other, second),
        ];
        for (input, text) in inputs {
            set(&input, &text, cx);
        }
        cx.notify();
    }

    pub fn toggle_rule(&mut self, at: usize, cx: &mut Context<Self>) {
        self.change_traffic_rules(
            |rules| {
                if let Some(rule) = rules.rules.get_mut(at) {
                    rule.enabled = !rule.enabled;
                }
            },
            cx,
        );
    }

    pub fn delete_rule(&mut self, at: usize, cx: &mut Context<Self>) {
        self.change_traffic_rules(
            |rules| {
                if at < rules.rules.len() {
                    rules.rules.remove(at);
                }
            },
            cx,
        );
        if self.rules_form(cx).editing.is_some() {
            self.clear_rule_form(cx);
        }
    }

    pub(crate) fn render_rules(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        self.rules_form(cx);
        let network = self.render_network(cx);
        let rules = self.traffic_rules();
        let page = self.rules_page.as_ref().expect("just made");
        let note = page.note.clone();
        let small = |id: (&'static str, u64), label: String| -> Stateful<Div> {
            div()
                .id(id)
                .flex_shrink_0()
                .h(px(28.))
                .px(px(12.))
                .rounded(px(14.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(px(12.))
                .font_weight(FontWeight::BOLD)
                .bg(rgb(c.chip))
                .text_color(rgb(c.on_chip))
                .child(label)
        };
        let list = rules.rules.iter().enumerate().map(|(at, rule)| {
            let key = at as u64;
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(12.))
                .px(px(14.))
                .py(px(10.))
                .rounded(px(18.))
                .bg(rgb(c.surface))
                .child(
                    div()
                        .id(("rule-on", key))
                        .w(px(44.))
                        .h(px(24.))
                        .p(px(3.))
                        .flex_shrink_0()
                        .rounded(px(12.))
                        .flex()
                        .cursor_pointer()
                        .reachable()
                        .when(rule.enabled, |d| d.bg(rgb(c.accent)).justify_end())
                        .when(!rule.enabled, |d| d.bg(rgb(c.input)))
                        .child(div().size(px(18.)).rounded(px(9.)).bg(rgb(if rule.enabled {
                            c.on_accent
                        } else {
                            c.muted
                        })))
                        .on_click(cx.listener(move |this, _, _, cx| this.toggle_rule(at, cx))),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(260.))
                        .flex()
                        .flex_col()
                        .child(div().font_weight(FontWeight::BOLD).child(rule.name.clone()))
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(c.muted))
                                .font_family(crate::MONO_FONT)
                                .font_features(gpui::FontFeatures::disable_ligatures())
                                .child(format!(
                                    "{}{}",
                                    if rule.method.is_empty() {
                                        String::new()
                                    } else {
                                        format!("{} ", rule.method.to_ascii_uppercase())
                                    },
                                    rule.url
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(c.muted))
                                .child(action_text(&rule.action)),
                        ),
                )
                .child(
                    small(("rule-edit", key), tr!("Edit"))
                        .on_click(cx.listener(move |this, _, _, cx| this.edit_rule(at, cx))),
                )
                .child(
                    small(("rule-delete", key), tr!("Delete"))
                        .on_click(cx.listener(move |this, _, _, cx| this.delete_rule(at, cx))),
                )
        });
        let field = |input: &Entity<TextInput>, mono: bool| {
            div()
                .flex_1()
                .min_w(px(220.))
                .h(px(36.))
                .px(px(12.))
                .rounded(px(12.))
                .flex()
                .items_center()
                .bg(rgb(c.input))
                .when(mono, |d| {
                    d.font_family(crate::MONO_FONT)
                        .font_features(gpui::FontFeatures::disable_ligatures())
                })
                .child(input.clone())
        };
        let (first_hint, second_hint) = page.kind.fields();
        let no_fields = first_hint.is_none();
        let current = page.kind;
        let kinds = RuleKind::ALL.into_iter().map(|kind| {
            let on = kind == current;
            div()
                .id(("rule-kind", kind as u64))
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
                .child(kind.label())
                .on_click(cx.listener(move |this, _, _, cx| this.set_rule_kind(kind, cx)))
        });
        let form = div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .p(px(14.))
            .rounded(px(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(if page.editing.is_some() {
                tr!("Change this rule")
            } else {
                tr!("A new rule")
            }))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(6.))
                    .children(kinds),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(8.))
                    .child(field(&page.name, false))
                    .child(field(&page.url, true))
                    .child(div().w(px(220.)).flex().child(field(&page.method, true))),
            )
            .when_some(first_hint, |d, first_hint| {
                d.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(c.muted))
                        .child(first_hint),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap(px(8.))
                        .child(field(&page.first, true))
                        .when(second_hint.is_some() && !current.takes_a_body(), |d| {
                            d.child(field(&page.second, true))
                        }),
                )
                .when(current.takes_a_body(), |d| {
                    d.child(
                        div()
                            .w_full()
                            .px(px(12.))
                            .py(px(8.))
                            .rounded(px(12.))
                            .bg(rgb(c.input))
                            .font_family(crate::MONO_FONT)
                            .font_features(gpui::FontFeatures::disable_ligatures())
                            .child(page.body.clone()),
                    )
                })
            })
            .when(no_fields, |d| {
                d.child(div().text_size(px(12.)).text_color(rgb(c.muted)).child(tr!(
                    "It waits under Paused until you send it on, changed or not."
                )))
            })
            .children(
                page.problem
                    .clone()
                    .map(|problem| div().text_color(rgb(c.error)).child(problem)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.))
                    .child(
                        div()
                            .id("rule-save")
                            .h(px(36.))
                            .px(px(16.))
                            .rounded(px(18.))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .reachable()
                            .font_weight(FontWeight::BOLD)
                            .bg(rgb(c.accent))
                            .text_color(rgb(c.on_accent))
                            .child(if page.editing.is_some() {
                                tr!("Save")
                            } else {
                                tr!("Add")
                            })
                            .on_click(cx.listener(|this, _, _, cx| this.save_rule(cx))),
                    )
                    .when(page.editing.is_some(), |d| {
                        d.child(
                            div()
                                .id("rule-cancel")
                                .h(px(36.))
                                .px(px(16.))
                                .rounded(px(18.))
                                .flex()
                                .items_center()
                                .cursor_pointer()
                                .reachable()
                                .font_weight(FontWeight::BOLD)
                                .bg(rgb(c.chip))
                                .text_color(rgb(c.on_chip))
                                .child(tr!("Cancel"))
                                .on_click(cx.listener(|this, _, _, cx| this.clear_rule_form(cx))),
                        )
                    }),
            );
        div()
            .id("rules")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .pb(px(4.))
                    .child(
                        div()
                            .text_size(px(20.))
                            .font_weight(FontWeight::BOLD)
                            .child(tr!("Rules")),
                    )
                    .child(div().text_color(rgb(c.muted)).child(tr!(
                        "They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent."
                    ))),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        small(("import-rules", 0), tr!("Import rules…"))
                            .on_click(cx.listener(|this, _, _, cx| this.ask_import_rules(cx))),
                    )
                    .when(!rules.rules.is_empty(), |d| {
                        d.child(
                            small(("export-rules", 0), tr!("Export rules…"))
                                .on_click(cx.listener(|this, _, _, cx| this.ask_export_rules(cx))),
                        )
                    })
                    .children(note.map(|(note, failed)| {
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(if failed { c.error } else { c.muted }))
                            .child(note)
                    })),
            )
            .child(network)
            .child(form)
            .children(list)
    }
}

/// What an import did, in a line.
fn import_note(imported: &Imported) -> String {
    let mut parts = vec![tr_n!(imported.added, "{n} rule added", "{n} rules added")];
    if imported.already > 0 {
        parts.push(tr_n!(
            imported.already,
            "{n} here already",
            "{n} here already"
        ));
    }
    if !imported.refused.is_empty() {
        let why: Vec<String> = imported
            .refused
            .iter()
            .map(|(name, why)| format!("{name} ({why})"))
            .collect();
        parts.push(tr!("left out: {why}", why = why.join("; ")));
    }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_import_says_what_it_did() {
        let imported = Imported {
            added: 2,
            already: 1,
            refused: vec![("Mocks".to_owned(), "needs a full path".to_owned())],
        };
        assert_eq!(
            import_note(&imported),
            "2 rules added, 1 here already, left out: Mocks (needs a full path)"
        );
        assert_eq!(
            import_note(&Imported {
                added: 1,
                ..Imported::default()
            }),
            "1 rule added"
        );
    }

    #[test]
    fn the_form_says_what_is_wrong_with_a_status() {
        assert_eq!(
            RuleKind::Block.action("403", "").unwrap(),
            Action::Block { status: 403 }
        );
        assert!(
            RuleKind::Block
                .action("nope", "")
                .unwrap_err()
                .contains("not a status")
        );
    }

    #[test]
    fn every_kind_goes_to_the_form_and_back() {
        for (kind, first, second) in [
            (RuleKind::Block, "451", ""),
            (RuleKind::MapLocal, "/tmp/mock", ""),
            (RuleKind::MapRemote, "http://localhost:3000", ""),
            (RuleKind::RequestHeader, "x-a", "1"),
            (RuleKind::ResponseHeader, "x-b", "2"),
            (RuleKind::PauseRequest, "", ""),
            (RuleKind::PauseResponse, "", ""),
            (RuleKind::PauseMessagesToServer, "", ""),
            (RuleKind::PauseMessagesToApp, "", ""),
        ] {
            let action = kind.action(first, second).unwrap();
            assert_eq!(
                fields_of(&action),
                (kind, first.to_owned(), second.to_owned())
            );
        }
    }
}
