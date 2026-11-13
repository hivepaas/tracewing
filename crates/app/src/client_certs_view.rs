//! Settings' client certificates: for servers that ask the client for one,
//! a PEM certificate and its key for the hosts a pattern names. Each applies
//! to the next connection; the files stay where they are.

use crate::text_size::ui;
use crate::{
    colors::colors,
    input::TextInput,
    traffic::Traffic,
    widgets::{Reachable, eyebrow},
};
use gpui::{AppContext as _, Context, Entity, FontWeight, PathPromptOptions, div, prelude::*, rgb};
use std::path::PathBuf;
use tracewing_i18n::tr;
use tracewing_proxy::client_certs::{ClientCert, ClientCertSettings};

/// A certificate being added: its host typed, its files chosen.
pub struct ClientCertForm {
    pub host: Entity<TextInput>,
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    pub problem: Option<String>,
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

impl Traffic {
    fn client_cert_form(&mut self, cx: &mut Context<Self>) -> &mut ClientCertForm {
        self.client_cert_form.get_or_insert_with(|| ClientCertForm {
            host: cx.new(|cx| TextInput::said(|| tr!("api.example.com, or *.example.com"), cx)),
            cert: None,
            key: None,
            problem: None,
        })
    }

    /// The certificates set, as `client-certs.toml` has them.
    pub fn client_cert_settings(&self) -> ClientCertSettings {
        ClientCertSettings::load(&self.data_dir).unwrap_or_default()
    }

    /// Asks for the certificate's file, or the key's.
    pub fn ask_for_client_cert_file(&mut self, key: bool, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(tr!("Choose").into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(mut paths))) = chosen.await
                && let Some(path) = paths.pop()
            {
                let _ = this.update(cx, |this, cx| this.set_client_cert_file(key, path, cx));
            }
        })
        .detach();
    }

    pub fn set_client_cert_file(&mut self, key: bool, path: PathBuf, cx: &mut Context<Self>) {
        let form = self.client_cert_form(cx);
        if key {
            form.key = Some(path);
        } else {
            form.cert = Some(path);
        }
        form.problem = None;
        cx.notify();
    }

    /// Adds what the form says, if its files can be read and go together.
    pub fn add_client_cert(&mut self, cx: &mut Context<Self>) {
        let form = self.client_cert_form(cx);
        let host = form.host.read(cx).text().trim().to_ascii_lowercase();
        let Some(cert) = form.cert.clone() else {
            form.problem = Some(tr!("Choose the certificate's file."));
            return cx.notify();
        };
        if host.is_empty() {
            form.problem = Some(tr!("Say which host it is for."));
            return cx.notify();
        }
        let added = ClientCert {
            host,
            cert,
            key: form.key.clone(),
        };
        if let Err(why) =
            tracewing_proxy::client_certs::read(&added, &tracewing_proxy::crypto_provider())
        {
            form.problem = Some(why);
            return cx.notify();
        }
        let mut settings = self.client_cert_settings();
        settings.certs.push(added);
        if self.apply_client_certs(&settings, cx) {
            let form = self.client_cert_form(cx);
            form.cert = None;
            form.key = None;
            form.problem = None;
            let host = form.host.clone();
            host.update(cx, |input, cx| input.set_text("", 0..0, cx));
        }
        cx.notify();
    }

    pub fn remove_client_cert(&mut self, at: usize, cx: &mut Context<Self>) {
        let mut settings = self.client_cert_settings();
        if at < settings.certs.len() {
            settings.certs.remove(at);
            self.apply_client_certs(&settings, cx);
        }
        cx.notify();
    }

    /// Saves them, and uses them from the next connection. False if they
    /// could not be saved, which the form says.
    fn apply_client_certs(
        &mut self,
        settings: &ClientCertSettings,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Err(e) = settings.save(&self.data_dir) {
            self.client_cert_form(cx).problem = Some(format!("{e:#}"));
            return false;
        }
        self.engine.client_certs.set(settings);
        true
    }

    pub(crate) fn render_client_certs(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        self.client_cert_form(cx);
        let settings = self.client_cert_settings();
        let problems = self.engine.client_certs.problems();
        let form = self.client_cert_form.as_ref().expect("just made");
        let button = |id: &'static str, label: String| {
            div()
                .id(id)
                .h(ui(32.))
                .px(ui(14.))
                .rounded(ui(16.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .text_size(ui(12.5))
                .font_weight(FontWeight::BOLD)
                .bg(rgb(c.chip))
                .text_color(rgb(c.on_chip))
                .child(label)
        };
        let rows = settings.certs.iter().enumerate().map(|(at, cert)| {
            let files = match &cert.key {
                Some(key) => format!("{} · {}", file_name(&cert.cert), file_name(key)),
                None => file_name(&cert.cert),
            };
            let problem = problems.get(at).cloned().flatten();
            div()
                .px(ui(6.))
                .flex()
                .flex_row()
                .items_center()
                .gap(ui(10.))
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
                                .gap(ui(10.))
                                .child(div().font_weight(FontWeight::BOLD).child(cert.host.clone()))
                                .child(
                                    div()
                                        .text_size(ui(12.))
                                        .text_color(rgb(c.muted))
                                        .child(files),
                                ),
                        )
                        .children(problem.map(|why| {
                            div()
                                .text_size(ui(12.))
                                .text_color(rgb(c.error))
                                .child(tr!("Not used: {why}", why = why))
                        })),
                )
                .child(
                    div()
                        .id(("remove-client-cert", at))
                        .h(ui(28.))
                        .px(ui(12.))
                        .rounded(ui(14.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .reachable()
                        .text_size(ui(12.))
                        .font_weight(FontWeight::BOLD)
                        .bg(rgb(c.chip))
                        .text_color(rgb(c.on_chip))
                        .child(tr!("Remove"))
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.remove_client_cert(at, cx)),
                        ),
                )
        });
        let chosen = |path: &Option<PathBuf>, none: String| {
            div()
                .text_size(ui(12.))
                .text_color(rgb(c.muted))
                .child(path.as_deref().map(file_name).unwrap_or(none))
        };
        div()
            .flex()
            .flex_col()
            .gap(ui(8.))
            .p(ui(14.))
            .rounded(ui(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Client certificates")).px(ui(6.)))
            .child(div().px(ui(6.)).text_size(ui(12.)).text_color(rgb(c.muted)).child(tr!(
                "For servers that ask the client for a certificate (mutual TLS): tracewing shows the one given for their host, in the app's place. A PEM certificate and its key without a passphrase; the files stay where they are. Each applies to the next connection."
            )))
            .children(rows)
            .when(settings.certs.is_empty(), |d| {
                d.child(div().px(ui(6.)).text_color(rgb(c.muted)).child(tr!("None yet.")))
            })
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(ui(8.))
                    .child(
                        div()
                            .id("client-cert-host")
                            .w(ui(340.))
                            .h(ui(32.))
                            .px(ui(10.))
                            .rounded(ui(12.))
                            .flex()
                            .items_center()
                            .bg(rgb(c.input))
                            .font_family(crate::mono_font())
                            .child(form.host.clone()),
                    )
                    .child(
                        button("client-cert-file", tr!("Certificate…")).on_click(cx.listener(
                            |this, _, _, cx| this.ask_for_client_cert_file(false, cx),
                        )),
                    )
                    .child(chosen(&form.cert, tr!("none chosen")))
                    .child(
                        button("client-cert-key", tr!("Key…")).on_click(cx.listener(
                            |this, _, _, cx| this.ask_for_client_cert_file(true, cx),
                        )),
                    )
                    .child(chosen(&form.key, tr!("in the certificate's file")))
                    .child(
                        button("client-cert-add", tr!("Add"))
                            .on_click(cx.listener(|this, _, _, cx| this.add_client_cert(cx))),
                    ),
            )
            .children(form.problem.clone().map(|why| {
                div()
                    .px(ui(6.))
                    .text_color(rgb(c.error))
                    .child(why)
            }))
    }
}
