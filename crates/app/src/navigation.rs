//! Back and forward through the places the window has shown, as a browser goes:
//! the screen, the part of Settings open, and the session on show.

use std::path::PathBuf;

use gpui::Context;

use crate::layout::SettingsSection;
use crate::traffic::{Screen, Traffic};

/// A place the window can show again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub screen: Screen,
    /// The part open, on Settings.
    pub section: Option<SettingsSection>,
    /// The earlier session on show, or none for the one recording.
    pub session: Option<PathBuf>,
}

impl Traffic {
    /// Where the window is.
    pub fn here(&self) -> Location {
        Location {
            screen: self.screen,
            section: (self.screen == Screen::Settings).then_some(self.layout.settings_section),
            session: self.viewing.clone(),
        }
    }

    /// The user went where the window is now: kept, unless it is going back or
    /// forward itself.
    pub(crate) fn went(&mut self) {
        if self.going {
            return;
        }
        let here = self.here();
        self.history.went(here);
    }

    /// Whether `to` can still be shown: Compare only with flows to compare, an
    /// earlier session only while it is on disk.
    fn can_go(&self, to: &Location) -> bool {
        (to.screen != Screen::Compare || self.compare_page.is_some())
            && to.session.as_ref().is_none_or(|dir| dir.is_dir())
    }

    /// Goes back, or forward, to the nearest place that can be shown. False if
    /// there is none, or the list of shortcuts is over the window.
    pub fn go(&mut self, back: bool, cx: &mut Context<Self>) -> bool {
        if self.shortcuts_open {
            return false;
        }
        self.went();
        let Some((i, to)) = self.history.next(back, |to| self.can_go(to)) else {
            return false;
        };
        self.going = true;
        if to.session != self.viewing {
            match &to.session {
                Some(dir) => self.open_session(dir, cx),
                None => self.back_to_live(cx),
            }
        }
        if let Some(section) = to.section {
            self.show_settings_section(section, cx);
        }
        self.set_screen(to.screen, cx);
        self.going = false;
        let here = self.here();
        self.history.moved(i, here);
        cx.notify();
        true
    }
}
