//! HAR files in and out: the flows in the list, or a whole session, written for
//! other tools; a HAR from elsewhere opened as a session of its own.

use crate::traffic::{Screen, Traffic};
use gpui::{AppContext as _, Context, PathPromptOptions};
use std::path::{Path, PathBuf};
use tracewing_store::har::{self, ExportOptions};

#[derive(Default)]
pub struct HarState {
    /// Leave out the values of credential headers, and the cookies.
    pub without_secrets: bool,
    pub busy: bool,
    /// What the last import or export did, and whether it failed.
    pub note: Option<(String, bool)>,
}

/// Where a file is first offered to go: Downloads, if there is one.
fn save_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let downloads = home.join("Downloads");
    if downloads.is_dir() { downloads } else { home }
}

impl Traffic {
    fn har(&mut self) -> &mut HarState {
        &mut self.sessions_page.har
    }

    pub fn set_har_without_secrets(&mut self, on: bool, cx: &mut Context<Self>) {
        self.har().without_secrets = on;
        cx.notify();
    }

    /// Asks where, then writes the flows the list shows.
    pub fn ask_export_shown(&mut self, cx: &mut Context<Self>) {
        let session = self.session_dir().to_owned();
        let ids = self.shown_ids();
        let name = format!("tracewing-{}.har", file_stem(&session));
        self.ask_export(session, Some(ids), name, cx);
    }

    /// Asks where, then writes every flow of `session`.
    pub fn ask_export_session(&mut self, session: PathBuf, cx: &mut Context<Self>) {
        let name = format!("tracewing-{}.har", file_stem(&session));
        self.ask_export(session, None, name, cx);
    }

    fn ask_export(
        &mut self,
        session: PathBuf,
        ids: Option<Vec<i64>>,
        name: String,
        cx: &mut Context<Self>,
    ) {
        if self.har().busy {
            return;
        }
        let chosen = cx.prompt_for_new_path(&save_dir(), Some(&name));
        cx.spawn(async move |this, cx| {
            let path = match chosen.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(e)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.har().note = Some((format!("No file picker: {e:#}"), true));
                        cx.notify();
                    });
                    return;
                }
            };
            let _ = this.update(cx, |this, cx| this.export_har(session, ids, path, cx));
        })
        .detach();
    }

    /// Writes the flows `ids` of `session`, all of them if `None`, to `path`.
    pub fn export_har(
        &mut self,
        session: PathBuf,
        ids: Option<Vec<i64>>,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let options = ExportOptions {
            without_secrets: self.har().without_secrets,
        };
        self.har().busy = true;
        self.har().note = Some(("Writing the HAR…".to_owned(), false));
        cx.notify();
        cx.spawn(async move |this, cx| {
            let target = path.clone();
            let result = cx
                .background_spawn(async move {
                    let file = std::fs::File::create(&target)?;
                    let mut out = std::io::BufWriter::new(file);
                    har::export(&session, ids.as_deref(), options, &mut out)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let state = this.har();
                state.busy = false;
                state.note = Some(match result {
                    Ok(1) => (format!("1 flow written to {}", path.display()), false),
                    Ok(n) => (format!("{n} flows written to {}", path.display()), false),
                    Err(e) => (format!("The HAR was not written: {e:#}"), true),
                });
                cx.notify();
            });
        })
        .detach();
    }

    /// Asks for a HAR, then opens it as a session.
    pub fn ask_import_har(&mut self, cx: &mut Context<Self>) {
        if self.har().busy {
            return;
        }
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open".into()),
        });
        cx.spawn(async move |this, cx| {
            let path = match chosen.await {
                Ok(Ok(Some(mut paths))) if !paths.is_empty() => paths.swap_remove(0),
                Ok(Err(e)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.har().note = Some((format!("No file picker: {e:#}"), true));
                        cx.notify();
                    });
                    return;
                }
                _ => return,
            };
            let _ = this.update(cx, |this, cx| this.import_har(path, cx));
        })
        .detach();
    }

    /// Reads the HAR at `path` into a new session named after it, and opens it.
    pub fn import_har(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let sessions = self.sessions_dir();
        let session = sessions.join(crate::engine::next_session_name(&sessions));
        let name = path
            .file_stem()
            .map_or_else(|| "HAR".to_owned(), |s| s.to_string_lossy().into_owned());
        let settings = self.engine.settings.clone();
        self.har().busy = true;
        self.har().note = Some((format!("Reading {}…", path.display()), false));
        cx.notify();
        cx.spawn(async move |this, cx| {
            let target = session.clone();
            let result = cx
                .background_spawn(async move {
                    let file = std::fs::File::open(&path)?;
                    let imported = har::import(file, &target, &name, &settings);
                    if imported.is_err() {
                        // Half a session is no use to anyone.
                        let _ = std::fs::remove_dir_all(&target);
                    }
                    imported
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.har().busy = false;
                match result {
                    Ok(imported) => {
                        let skipped = match imported.skipped {
                            0 => String::new(),
                            n => format!(", {n} entries left out"),
                        };
                        this.har().note =
                            Some((format!("{} flows read{skipped}", imported.flows), false));
                        this.open_session(&session, cx);
                        this.set_screen(Screen::Traffic, cx);
                        this.refresh_sessions(cx);
                    }
                    Err(e) => {
                        this.har().note = Some((format!("The HAR was not read: {e:#}"), true));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn is_har_busy(&self) -> bool {
        self.sessions_page.har.busy
    }
}

fn file_stem(session: &Path) -> String {
    session.file_name().map_or_else(
        || "session".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    )
}
