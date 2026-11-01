//! The rules screen: what changes traffic on its way. They apply from the next
//! request, and a flow says which rules touched it.

use crate::{colors::colors, input::TextInput, traffic::Traffic, widgets::eyebrow};
use gpui::{AppContext as _, Context, Div, Entity, FontWeight, Stateful, div, prelude::*, px, rgb};
use std::path::PathBuf;
use tracewing_proxy::traffic_rules::{Action, TrafficRule, TrafficRules};

/// What a rule does, as the form asks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleKind {
    Block,
    MapLocal,
    MapRemote,
    RequestHeader,
    ResponseHeader,
}

impl RuleKind {
    pub const ALL: [Self; 5] = [
        Self::Block,
        Self::MapLocal,
        Self::MapRemote,
        Self::RequestHeader,
        Self::ResponseHeader,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Block => "Block",
            Self::MapLocal => "Answer from a file",
            Self::MapRemote => "Send to another host",
            Self::RequestHeader => "Set a request header",
            Self::ResponseHeader => "Set a response header",
        }
    }

    /// What the form's fields ask for: the first, and a second if it needs one.
    fn fields(self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::Block => ("Status to answer with, like 403", None),
            Self::MapLocal => ("Full path of a file or folder", None),
            Self::MapRemote => ("Where to, like http://localhost:3000", None),
            Self::RequestHeader | Self::ResponseHeader => ("Header's name", Some("Its value")),
        }
    }

    fn action(self, first: &str, second: &str) -> Result<Action, String> {
        let first = first.trim();
        Ok(match self {
            Self::Block => Action::Block {
                status: first
                    .parse()
                    .map_err(|_| format!("\"{first}\" is not a status"))?,
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
        })
    }
}

/// What a rule does, in words, for the list.
fn action_text(action: &Action) -> String {
    match action {
        Action::Block { status } => format!("Block with {status}"),
        Action::MapLocal { path } => format!("Answer from {}", path.display()),
        Action::MapRemote { to } => format!("Send to {to}"),
        Action::SetRequestHeader { header, value } => format!("Request: {header}: {value}"),
        Action::SetResponseHeader { header, value } => format!("Response: {header}: {value}"),
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
    }
}

pub struct RulesPage {
    pub kind: RuleKind,
    pub name: Entity<TextInput>,
    pub url: Entity<TextInput>,
    pub method: Entity<TextInput>,
    pub first: Entity<TextInput>,
    pub second: Entity<TextInput>,
    /// The rule the form is changing, or none for a new one.
    pub editing: Option<usize>,
    pub problem: Option<String>,
}

impl RulesPage {
    fn new(cx: &mut Context<Traffic>) -> Self {
        let field =
            |hint: &'static str, cx: &mut Context<Traffic>| cx.new(|cx| TextInput::new(hint, cx));
        Self {
            kind: RuleKind::Block,
            name: field("Name", cx),
            url: field("URL, like api.myapp.dev/v1/*", cx),
            method: field("Method, or empty for all", cx),
            first: field("", cx),
            second: field("", cx),
            editing: None,
            problem: None,
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

    fn change_traffic_rules(
        &mut self,
        change: impl FnOnce(&mut TrafficRules),
        cx: &mut Context<Self>,
    ) {
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
        cx.notify();
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
        self.rules_form(cx).kind = kind;
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
        ];
        for (input, text) in inputs {
            set(&input, text, cx);
        }
    }

    /// Adds the form's rule, or changes the one being edited.
    pub fn save_rule(&mut self, cx: &mut Context<Self>) {
        let page = self.rules_form(cx);
        let read = |input: &Entity<TextInput>| input.read(cx).text().to_owned();
        let (name, url, method, first, second) = (
            read(&page.name),
            read(&page.url),
            read(&page.method),
            read(&page.first),
            read(&page.second),
        );
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
        page.editing = Some(at);
        page.problem = None;
        let inputs = [
            (page.name.clone(), rule.name),
            (page.url.clone(), rule.url),
            (page.method.clone(), rule.method),
            (page.first.clone(), first),
            (page.second.clone(), second),
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
        let rules = self.traffic_rules();
        let page = self.rules_page.as_ref().expect("just made");
        let small = |id: (&'static str, u64), label: &'static str| -> Stateful<Div> {
            div()
                .id(id)
                .flex_shrink_0()
                .h(px(28.))
                .px(px(12.))
                .rounded(px(14.))
                .flex()
                .items_center()
                .cursor_pointer()
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
                    small(("rule-edit", key), "Edit")
                        .on_click(cx.listener(move |this, _, _, cx| this.edit_rule(at, cx))),
                )
                .child(
                    small(("rule-delete", key), "Delete")
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
                .when(mono, |d| d.font_family(crate::MONO_FONT))
                .child(input.clone())
        };
        let (first_hint, second_hint) = page.kind.fields();
        let current = page.kind;
        let kinds = RuleKind::ALL.into_iter().map(|kind| {
            let on = kind == current;
            div()
                .id(kind.label())
                .h(px(30.))
                .px(px(12.))
                .rounded(px(15.))
                .flex()
                .items_center()
                .cursor_pointer()
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
                "Change this rule"
            } else {
                "A new rule"
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
            .child(
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
                    .when_some(second_hint, |d, _| d.child(field(&page.second, true))),
            )
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
                            .font_weight(FontWeight::BOLD)
                            .bg(rgb(c.accent))
                            .text_color(rgb(c.on_accent))
                            .child(if page.editing.is_some() {
                                "Save"
                            } else {
                                "Add"
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
                                .font_weight(FontWeight::BOLD)
                                .bg(rgb(c.chip))
                                .text_color(rgb(c.on_chip))
                                .child("Cancel")
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
                            .child("Rules"),
                    )
                    .child(div().text_color(rgb(c.muted)).child(
                        "They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every header rule that fits applies.",
                    )),
            )
            .child(form)
            .children(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        ] {
            let action = kind.action(first, second).unwrap();
            assert_eq!(
                fields_of(&action),
                (kind, first.to_owned(), second.to_owned())
            );
        }
    }
}
