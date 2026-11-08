//! The program on this machine that sent a flow. The proxy finds it while the
//! flow goes on, and the writer takes the answer as it writes the row.

use std::{
    sync::{Condvar, Mutex},
    time::Instant,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct App {
    pub pid: u32,
    /// What to call it: the bundle of a macOS app, or the program's file.
    pub name: String,
    /// The program's file, when it could be read.
    pub path: Option<String>,
}

/// An answer that comes later. Many flows over one connection share it.
#[derive(Debug, Default)]
pub struct AppAnswer {
    found: Mutex<Option<Option<App>>>,
    ready: Condvar,
}

impl AppAnswer {
    pub fn give(&self, app: Option<App>) {
        let mut found = self.found.lock().unwrap_or_else(|e| e.into_inner());
        if found.is_none() {
            *found = Some(app);
            self.ready.notify_all();
        }
    }

    /// The answer, if it came.
    pub fn now(&self) -> Option<Option<App>> {
        self.found.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Waits for the answer until `deadline`, and if it never comes, there is none.
    pub fn wait_until(&self, deadline: Instant) -> Option<App> {
        let mut found = self.found.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(app) = &*found {
                return app.clone();
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return None;
            }
            found = self
                .ready
                .wait_timeout(found, left)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }
}

/// What to call the program at `path`: the outermost `.app` it is inside, as macOS
/// apps run helpers deep in their bundles, or else its file without `.exe`.
pub fn name_of(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
    if let Some(bundle) = parts.iter().find_map(|p| p.strip_suffix(".app")) {
        return Some(bundle.to_owned());
    }
    let file = parts.last()?;
    let file = file
        .strip_suffix(".exe")
        .or_else(|| file.strip_suffix(".EXE"))
        .unwrap_or(file);
    Some(file.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::Arc, time::Duration};

    #[test]
    fn programs_go_by_their_apps_or_files() {
        assert_eq!(name_of("/usr/bin/curl").as_deref(), Some("curl"));
        assert_eq!(
            name_of(
                "/Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/\
                 Helpers/Google Chrome Helper.app/Contents/MacOS/Google Chrome Helper"
            )
            .as_deref(),
            Some("Google Chrome")
        );
        assert_eq!(
            name_of(r"C:\Program Files\Mozilla Firefox\firefox.exe").as_deref(),
            Some("firefox")
        );
        assert_eq!(name_of(""), None);
    }

    #[test]
    fn an_answer_is_waited_for_and_kept() {
        let answer = Arc::new(AppAnswer::default());
        assert_eq!(answer.now(), None);
        let giver = Arc::clone(&answer);
        let app = App {
            pid: 7,
            name: "curl".into(),
            path: None,
        };
        let given = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            giver.give(Some(given));
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        assert_eq!(answer.wait_until(deadline), Some(app.clone()));
        // The first answer stays.
        answer.give(None);
        assert_eq!(answer.now(), Some(Some(app)));
    }

    #[test]
    fn no_answer_is_none_at_the_deadline() {
        let answer = AppAnswer::default();
        assert_eq!(answer.wait_until(Instant::now()), None);
    }
}
