//! The keyboard: what each key does, and the list the window shows of them.
//! `mod` is ⌘ on macOS and Ctrl elsewhere, as gpui's `secondary` is.

use crate::{
    colors::{colors, with_alpha},
    traffic::{Screen, Traffic},
    widgets::eyebrow,
};
use gpui::{Context, FontWeight, IntoElement, div, prelude::*, px, rgb, rgba};
use tracewing_i18n::tr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Filter,
    Place,
    Sidebar,
    Inspector,
    Up,
    Down,
    First,
    Last,
    PageUp,
    PageDown,
    Request,
    Response,
    SendAgain,
    EditAndSend,
    CopyCurl,
    Record,
    Clear,
    Go(Screen),
    Shortcuts,
    Close,
}

/// How far a page goes in the list.
pub const PAGE: i64 = 20;

/// The screens `mod`+1 to `mod`+7 go to, in the sidebar's order.
pub const SCREENS: [Screen; 7] = [
    Screen::Setup,
    Screen::Traffic,
    Screen::Rules,
    Screen::Paused,
    Screen::Compose,
    Screen::Sessions,
    Screen::Settings,
];

/// What a key does. `typing` is true while a text field has the keyboard:
/// then keys without `mod` are the field's.
pub fn action_of(key: &str, secondary: bool, shift: bool, typing: bool) -> Option<Action> {
    use Action::*;
    if secondary {
        if let Some(n) = key
            .parse::<usize>()
            .ok()
            .filter(|n| (1..=SCREENS.len()).contains(n))
        {
            return (!shift).then_some(Go(SCREENS[n - 1]));
        }
        return match (key, shift) {
            ("f", false) => Some(Filter),
            ("\\", false) => Some(Place),
            ("b", false) => Some(Sidebar),
            ("i", false) => Some(Inspector),
            ("[", false) => Some(Request),
            ("]", false) => Some(Response),
            ("enter", false) => Some(SendAgain),
            ("e", false) => Some(EditAndSend),
            ("c", true) => Some(CopyCurl),
            ("r", false) => Some(Record),
            ("k", false) => Some(Clear),
            ("/", _) => Some(Shortcuts),
            _ => None,
        };
    }
    if key == "escape" {
        return Some(Close);
    }
    if typing {
        return None;
    }
    match key {
        "up" => Some(Up),
        "down" => Some(Down),
        "home" => Some(First),
        "end" => Some(Last),
        "pageup" => Some(PageUp),
        "pagedown" => Some(PageDown),
        "?" => Some(Shortcuts),
        "/" if shift => Some(Shortcuts),
        _ => None,
    }
}

/// The keys as this OS writes them: `mod+shift+c` is ⌘⇧C on macOS, and
/// Ctrl+Shift+C elsewhere.
pub fn label(keys: &str) -> String {
    let mac = cfg!(target_os = "macos");
    let parts: Vec<String> = keys
        .split('+')
        .map(|part| {
            match part {
                "mod" if mac => "⌘",
                "mod" => "Ctrl",
                "shift" if mac => "⇧",
                "shift" => "Shift",
                "enter" if mac => "↩",
                "enter" => "Enter",
                "up" => "↑",
                "down" => "↓",
                "home" => "Home",
                "end" => "End",
                "pageup" => "Page Up",
                "pagedown" => "Page Down",
                "escape" => "Esc",
                other => return other.to_uppercase(),
            }
            .to_owned()
        })
        .collect();
    parts.join(if mac { "" } else { "+" })
}

/// A group of shortcuts: its title, and each row's keys and what they do.
pub type Group = (String, Vec<(&'static str, String)>);

/// What the list of shortcuts shows, a group at a time, in the language in use.
pub fn list() -> Vec<Group> {
    vec![
        (
            tr!("Flows"),
            vec![
                ("up / down", tr!("Choose the flow above or below")),
                ("home / end", tr!("The first or the last flow")),
                ("pageup / pagedown", tr!("A page up or down")),
                ("mod+f", tr!("Filter")),
                ("mod+r", tr!("Pause or resume recording")),
                ("mod+k", tr!("Clear the list")),
            ],
        ),
        (
            tr!("The chosen flow"),
            vec![
                ("mod+[", tr!("Its request")),
                ("mod+]", tr!("Its response")),
                ("mod+enter", tr!("Send it again")),
                ("mod+e", tr!("Edit it and send it")),
                ("mod+shift+c", tr!("Copy it as cURL")),
            ],
        ),
        (
            tr!("The window"),
            vec![
                (
                    "mod+1 … mod+7",
                    tr!("Set up, Traffic, Rules, Paused, Compose, Sessions, Settings"),
                ),
                ("mod+b", tr!("Show or hide the sidebar")),
                ("mod+i", tr!("Show or hide the inspector")),
                ("mod+\\", tr!("The inspector beside the list or below it")),
                ("mod+/", tr!("These shortcuts")),
                ("escape", tr!("Close them")),
            ],
        ),
    ]
}

/// A row's keys as this OS writes them, alternatives kept apart.
pub fn row_label(keys: &str) -> String {
    keys.split(" / ")
        .map(|alt| alt.split(" … ").map(label).collect::<Vec<_>>().join(" … "))
        .collect::<Vec<_>>()
        .join(" / ")
}

impl Traffic {
    /// The list of shortcuts, over the window. A click outside it or Esc closes it.
    pub(crate) fn render_shortcuts(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let groups = list().into_iter().map(|(title, rows)| {
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(eyebrow(title))
                .children(rows.into_iter().map(|(keys, what)| {
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(14.))
                        .child(
                            div().w(px(170.)).flex_shrink_0().child(
                                div()
                                    .px(px(8.))
                                    .py(px(2.))
                                    .rounded(px(8.))
                                    .bg(rgb(c.chip))
                                    .text_color(rgb(c.on_chip))
                                    .font_family(crate::MONO_FONT)
                                    .font_features(gpui::FontFeatures::disable_ligatures())
                                    .text_size(px(12.))
                                    .child(row_label(keys)),
                            ),
                        )
                        .child(div().text_color(rgb(c.muted)).child(what))
                }))
        });
        div()
            .id("shortcuts-scrim")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(with_alpha(0x000000, 0x66)))
            .on_click(cx.listener(|this, _, _, cx| this.set_shortcuts(false, cx)))
            .child(
                div()
                    .id("shortcuts")
                    .w(px(620.))
                    .flex()
                    .flex_col()
                    .gap(px(18.))
                    .p(px(24.))
                    .rounded(px(22.))
                    .bg(rgb(c.surface))
                    .text_color(rgb(c.text))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(18.))
                                    .font_weight(FontWeight::BOLD)
                                    .child(tr!("Keyboard shortcuts")),
                            )
                            .child(
                                div()
                                    .id("shortcuts-close")
                                    .px(px(12.))
                                    .py(px(4.))
                                    .rounded(px(14.))
                                    .cursor_pointer()
                                    .bg(rgb(c.chip))
                                    .text_color(rgb(c.on_chip))
                                    .child(tr!("Close"))
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.set_shortcuts(false, cx)),
                                    ),
                            ),
                    )
                    .children(groups)
                    // A click on the list itself leaves it open.
                    .on_click(|_, _, cx| cx.stop_propagation()),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_do_what_the_list_says() {
        assert_eq!(action_of("f", true, false, true), Some(Action::Filter));
        assert_eq!(action_of("c", true, true, false), Some(Action::CopyCurl));
        // Plain mod+c stays the text field's copy.
        assert_eq!(action_of("c", true, false, false), None);
        assert_eq!(
            action_of("3", true, false, false),
            Some(Action::Go(Screen::Rules))
        );
        assert_eq!(action_of("8", true, false, false), None);
        assert_eq!(action_of("/", true, false, true), Some(Action::Shortcuts));
        assert_eq!(action_of("?", false, true, false), Some(Action::Shortcuts));
        assert_eq!(action_of("home", false, false, false), Some(Action::First));
        assert_eq!(action_of("escape", false, false, true), Some(Action::Close));
    }

    #[test]
    fn a_text_field_keeps_its_own_keys() {
        for key in ["up", "down", "home", "end", "pageup", "?"] {
            assert_eq!(action_of(key, false, false, true), None, "{key}");
        }
    }

    #[test]
    fn every_listed_key_does_something() {
        for (_, rows) in list() {
            for (keys, what) in rows {
                for alt in keys.split(" / ").flat_map(|k| k.split(" … ")) {
                    let parts: Vec<&str> = alt.split('+').collect();
                    let key = *parts.last().unwrap();
                    let secondary = parts.contains(&"mod");
                    let shift = parts.contains(&"shift");
                    assert!(
                        action_of(key, secondary, shift, false).is_some(),
                        "{alt}: {what}"
                    );
                }
            }
        }
    }

    #[test]
    fn labels_follow_the_os() {
        let label = row_label("mod+shift+c");
        if cfg!(target_os = "macos") {
            assert_eq!(label, "⌘⇧C");
        } else {
            assert_eq!(label, "Ctrl+Shift+C");
        }
        assert_eq!(row_label("up / down"), "↑ / ↓");
    }
}
