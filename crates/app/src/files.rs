//! Files in and out. HAR: the flows in the list, or a whole session, written
//! for other tools. A HAR, a Fiddler `.saz` or a Charles `.chlsj` from elsewhere,
//! opened as a session of its own. And a flow's body, saved as it is, unpacked.

use crate::{
    detail::Side,
    traffic::{Screen, Traffic},
};
use gpui::{AppContext as _, Context, PathPromptOptions};
use std::path::{Path, PathBuf};
use tracewing_store::{
    har::{self, ExportOptions},
    import,
    session::BodyReader,
};

#[derive(Default)]
pub struct FileState {
    /// Leave out the values of credential headers, and the cookies.
    pub without_secrets: bool,
    pub busy: bool,
    /// What the last file written or read did, and whether it failed.
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
    fn files(&mut self) -> &mut FileState {
        &mut self.sessions_page.files
    }

    pub fn set_har_without_secrets(&mut self, on: bool, cx: &mut Context<Self>) {
        self.files().without_secrets = on;
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
        if self.files().busy {
            return;
        }
        let chosen = cx.prompt_for_new_path(&save_dir(), Some(&name));
        cx.spawn(async move |this, cx| {
            let path = match chosen.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(e)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.files().note = Some((format!("No file picker: {e:#}"), true));
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
            without_secrets: self.files().without_secrets,
        };
        self.files().busy = true;
        self.files().note = Some(("Writing the HAR…".to_owned(), false));
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
                let state = this.files();
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

    /// Asks for a file from another tool, then opens it as a session.
    pub fn ask_import(&mut self, cx: &mut Context<Self>) {
        if self.files().busy {
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
                        this.files().note = Some((format!("No file picker: {e:#}"), true));
                        cx.notify();
                    });
                    return;
                }
                _ => return,
            };
            let _ = this.update(cx, |this, cx| this.import_file(path, cx));
        })
        .detach();
    }

    /// Reads the HAR, `.saz` or `.chlsj` at `path` into a new session named after
    /// it, and opens it.
    pub fn import_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let sessions = self.sessions_dir();
        let session = sessions.join(crate::engine::next_session_name(&sessions));
        let name = path.file_stem().map_or_else(
            || "Imported".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        let settings = self.engine.settings.clone();
        self.files().busy = true;
        self.files().note = Some((format!("Reading {}…", path.display()), false));
        cx.notify();
        cx.spawn(async move |this, cx| {
            let target = session.clone();
            let result = cx
                .background_spawn(async move {
                    let imported = import::import_file(&path, &target, &name, &settings);
                    if imported.is_err() {
                        // Half a session is no use to anyone.
                        let _ = std::fs::remove_dir_all(&target);
                    }
                    imported
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.files().busy = false;
                match result {
                    Ok((format, imported)) => {
                        let skipped = match imported.skipped {
                            0 => String::new(),
                            n => format!(", {n} entries left out"),
                        };
                        this.files().note = Some((
                            format!(
                                "{} flows read from a {}{skipped}",
                                imported.flows,
                                format.name()
                            ),
                            false,
                        ));
                        this.open_session(&session, cx);
                        this.set_screen(Screen::Traffic, cx);
                        this.refresh_sessions(cx);
                    }
                    Err(e) => {
                        this.files().note = Some((format!("The file was not read: {e:#}"), true));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn is_busy_with_files(&self) -> bool {
        self.sessions_page.files.busy
    }

    /// Asks where, then saves the body on show in the inspector.
    pub fn ask_save_body(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = &self.inspector.detail else {
            return;
        };
        let side = self.inspector.side;
        if !detail.body(side).stored.exists() {
            return;
        }
        let content_type = match side {
            Side::Request => detail.req_content_type.clone(),
            Side::Response => detail.content_type.clone(),
        };
        let name = body_file_name(&detail.url, content_type.as_deref(), side);
        let chosen = cx.prompt_for_new_path(&save_dir(), Some(&name));
        cx.spawn(async move |this, cx| {
            let path = match chosen.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(e)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.files().note = Some((format!("No file picker: {e:#}"), true));
                        cx.notify();
                    });
                    return;
                }
            };
            let _ = this.update(cx, |this, cx| this.save_body(side, path, cx));
        })
        .detach();
    }

    /// Writes the body of `side` of the flow on show to `path`, unpacked.
    pub fn save_body(&mut self, side: Side, path: PathBuf, cx: &mut Context<Self>) {
        let Some(detail) = &self.inspector.detail else {
            return;
        };
        let stored = detail.body(side).stored.clone();
        let session = self.session_dir().to_owned();
        self.files().busy = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let target = path.clone();
            let result = cx
                .background_spawn(async move {
                    let file = std::fs::File::create(&target)?;
                    let mut out = std::io::BufWriter::new(file);
                    let size = BodyReader::new(&session)
                        .unpack_into(&stored, u64::MAX, &mut out)
                        .ok_or_else(|| anyhow::anyhow!("it is no longer kept"))?;
                    std::io::Write::flush(&mut out)?;
                    anyhow::Ok(size)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                let state = this.files();
                state.busy = false;
                state.note = Some(match result {
                    Ok(size) => (
                        format!(
                            "{} saved to {}",
                            crate::flows::size_text(size),
                            path.display()
                        ),
                        false,
                    ),
                    Err(e) => {
                        let _ = std::fs::remove_file(&path);
                        (format!("The body was not saved: {e:#}"), true)
                    }
                });
                cx.notify();
            });
        })
        .detach();
    }
}

/// A name to offer for a body: the URL's own file name if it has one, or
/// `response.json` and the like from its type.
pub fn body_file_name(url: &str, content_type: Option<&str>, side: Side) -> String {
    let path = url.split(['?', '#']).next().unwrap_or_default();
    let last = path
        .rsplit('/')
        .next()
        .filter(|_| path.matches('/').count() > 2)
        .unwrap_or_default();
    if last.contains('.') && !last.starts_with('.') {
        return tracewing_store::decode::percent_decode(last).replace(['/', '\\'], "_");
    }
    let essence = content_type
        .and_then(|t| t.split(';').next())
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let extension = match essence.as_str() {
        "application/json" | "application/problem+json" => "json",
        "text/html" => "html",
        "text/css" => "css",
        "text/plain" => "txt",
        "text/csv" => "csv",
        "text/javascript" | "application/javascript" => "js",
        "application/xml" | "text/xml" => "xml",
        "application/pdf" => "pdf",
        "application/zip" => "zip",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "image/avif" => "avif",
        "font/woff2" => "woff2",
        "application/wasm" => "wasm",
        e if e.ends_with("+json") => "json",
        _ => "bin",
    };
    let stem = match side {
        Side::Request => "request",
        Side::Response => "response",
    };
    format!("{stem}.{extension}")
}

fn file_stem(session: &Path) -> String {
    session.file_name().map_or_else(
        || "session".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_is_offered_a_fitting_name() {
        let name = |url, ct| body_file_name(url, ct, Side::Response);
        assert_eq!(
            name("https://a.test/img/logo%20big.png?v=2", None),
            "logo big.png"
        );
        assert_eq!(
            name(
                "https://a.test/api/items",
                Some("application/json; charset=utf-8")
            ),
            "response.json"
        );
        assert_eq!(name("https://a.test/", Some("text/html")), "response.html");
        assert_eq!(
            name("https://a.test", Some("application/vnd.api+json")),
            "response.json"
        );
        assert_eq!(
            body_file_name("https://a.test/upload", None, Side::Request),
            "request.bin"
        );
    }
}
