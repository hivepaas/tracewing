//! Settings' CA signed by the company: a request (CSR) to give IT, what IT
//! sends back taken in, and back to tracewing's own. Each from the next start.

use crate::{
    colors::colors,
    traffic::Traffic,
    widgets::{Reachable, eyebrow},
};
use gpui::{Context, FontWeight, PathPromptOptions, div, prelude::*, px, rgb};
use std::path::{Path, PathBuf};
use tracewing_i18n::tr;
use tracewing_proxy::company_ca;

impl Traffic {
    fn company_note(&mut self, note: String, failed: bool, cx: &mut Context<Self>) {
        self.company_ca_note = Some((note, failed));
        cx.notify();
    }

    /// Asks where, then writes the request there.
    pub fn ask_save_company_request(&mut self, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_new_path(&crate::files::save_dir(), Some("tracewing-ca.csr"));
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = chosen.await {
                let _ = this.update(cx, |this, cx| this.save_company_request(&path, cx));
            }
        })
        .detach();
    }

    pub fn save_company_request(&mut self, path: &Path, cx: &mut Context<Self>) {
        let written = company_ca::request(&self.data_dir, self.engine.key_store)
            .and_then(|csr| std::fs::write(path, csr).map_err(anyhow::Error::from));
        match written {
            Ok(()) => self.company_note(
                tr!(
                    "The request is in {path}. Give it to your IT team to sign as an intermediate CA, then take in what they send back.",
                    path = path.display().to_string()
                ),
                false,
                cx,
            ),
            Err(e) => self.company_note(format!("{e:#}"), true, cx),
        }
    }

    /// Asks for the file IT sent back, then takes it in.
    pub fn ask_take_company_cert(&mut self, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(tr!("Take in").into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(mut paths))) = chosen.await
                && let Some(path) = paths.pop()
            {
                let _ = this.update(cx, |this, cx| this.take_company_cert(&path, cx));
            }
        })
        .detach();
    }

    pub fn take_company_cert(&mut self, path: &PathBuf, cx: &mut Context<Self>) {
        let taken = std::fs::read_to_string(path)
            .map_err(anyhow::Error::from)
            .and_then(|pem| company_ca::use_signed(&self.data_dir, &pem, self.engine.key_store));
        match taken {
            Ok(info) => self.company_note(
                tr!(
                    "Signed by {issuer}: restart tracewing to use it. Nothing needs installing on this company's machines.",
                    issuer = info.issuer
                ),
                false,
                cx,
            ),
            Err(e) => self.company_note(
                tr!("It cannot be used: {why}", why = format!("{e:#}")),
                true,
                cx,
            ),
        }
    }

    pub fn back_to_own_ca(&mut self, cx: &mut Context<Self>) {
        match company_ca::use_own(&self.data_dir) {
            Ok(_) => self.company_note(
                tr!("Back to tracewing's own certificate: restart tracewing to use it."),
                false,
                cx,
            ),
            Err(e) => self.company_note(format!("{e:#}"), true, cx),
        }
    }

    pub(crate) fn render_company_ca(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let info = company_ca::info(&self.data_dir).ok();
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
        let now = info.as_ref().map(|info| {
            if info.company {
                tr!(
                    "Now: signed by {issuer}, until {until}.",
                    issuer = info.issuer.as_str(),
                    until = info.not_after.as_str()
                )
            } else {
                tr!(
                    "Now: tracewing's own, which signs itself, until {until}.",
                    until = info.not_after.as_str()
                )
            }
        });
        let company = info.as_ref().is_some_and(|info| info.company);
        let can_go_back = info.as_ref().is_some_and(|info| info.can_go_back);
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(14.))
            .rounded(px(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Certificate signed by your company")).px(px(6.)))
            .child(div().px(px(6.)).text_size(px(12.)).text_color(rgb(c.muted)).child(tr!(
                "Where IT lets no new root certificate in: tracewing makes a request for its CA's key, your IT team signs it as an intermediate CA under the company's root, and tracewing uses that instead. The company's computers trust it already, with nothing to install."
            )))
            .children(now.map(|now| div().px(px(6.)).child(now)))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(8.))
                    .child(
                        button("company-request", tr!("Make the request (CSR)…")).on_click(
                            cx.listener(|this, _, _, cx| this.ask_save_company_request(cx)),
                        ),
                    )
                    .child(
                        button("company-take", tr!("Take in the signed certificate…")).on_click(
                            cx.listener(|this, _, _, cx| this.ask_take_company_cert(cx)),
                        ),
                    )
                    .when(company && can_go_back, |d| {
                        d.child(button("company-back", tr!("Back to tracewing's own")).on_click(
                            cx.listener(|this, _, _, cx| this.back_to_own_ca(cx)),
                        ))
                    }),
            )
            .children(self.company_ca_note.clone().map(|(note, failed)| {
                div()
                    .px(px(6.))
                    .text_color(rgb(if failed { c.error } else { c.text }))
                    .child(note)
            }))
    }
}
