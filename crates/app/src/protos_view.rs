//! The `.proto` files chosen in Settings, which name the fields of Protobuf
//! and gRPC bodies.

use crate::{
    colors::colors,
    traffic::Traffic,
    widgets::{Reachable, eyebrow},
};
use gpui::{
    Context, ElementId, FontWeight, PathPromptOptions, SharedString, div, prelude::*, px, rgb,
};
use std::{path::PathBuf, sync::Arc};
use tracewing_i18n::{tr, tr_n};
use tracewing_store::schema::Schema;

/// How many problems a source shows; the rest are counted.
const PROBLEMS_SHOWN: usize = 5;

impl Traffic {
    /// Whether the `.proto` files are being read.
    pub fn protos_busy(&self) -> bool {
        self.protos_busy
    }

    /// Reads the sources again, off the UI thread, and shows what came of it.
    pub fn read_protos(&mut self, cx: &mut Context<Self>) {
        if self.protos_busy {
            return;
        }
        self.protos_busy = true;
        let protos = Arc::clone(&self.protos);
        cx.spawn(async move |this, cx| {
            let schema = cx.background_spawn(async move { protos.schema() }).await;
            let _ = this.update(cx, |this, cx| {
                this.protos_busy = false;
                this.protos_read = Some(schema);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Adds sources and reads them all again. What comes of it shows when
    /// they are read.
    pub fn add_protos(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.change_protos(cx, move |protos| protos.add(paths));
    }

    pub fn remove_proto(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.change_protos(cx, move |protos| protos.remove(&path));
    }

    fn change_protos(
        &mut self,
        cx: &mut Context<Self>,
        change: impl FnOnce(&tracewing_store::schema::Library) -> std::io::Result<Arc<Schema>>
        + Send
        + 'static,
    ) {
        self.protos_busy = true;
        let protos = Arc::clone(&self.protos);
        cx.spawn(async move |this, cx| {
            let done = cx.background_spawn(async move { change(&protos) }).await;
            let _ = this.update(cx, |this, cx| {
                this.protos_busy = false;
                match done {
                    Ok(schema) => {
                        this.protos_read = Some(schema);
                        this.protos_problem = None;
                        // Bodies shown may read differently now.
                        this.load_body(cx);
                    }
                    Err(e) => {
                        this.protos_problem =
                            Some(tr!("The list was not saved: {e}", e = e.to_string()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// Asks for folders, or for files, to add.
    pub fn ask_for_protos(&mut self, folders: bool, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: !folders,
            directories: folders,
            multiple: true,
            prompt: Some(tr!("Add").into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = chosen.await
                && !paths.is_empty()
            {
                let _ = this.update(cx, |this, cx| this.add_protos(paths, cx));
            }
        })
        .detach();
    }

    pub(crate) fn render_protos(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let c = colors();
        let sources = self.protos.sources();
        if self.protos_read.is_none() && !sources.is_empty() {
            self.read_protos(cx);
        }
        let read = self.protos_read.clone();
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
        let rows = sources.iter().enumerate().map(|(i, path)| {
            let report = read
                .as_ref()
                .and_then(|schema| schema.sources.iter().find(|s| &s.path == path));
            let summary: SharedString = match report {
                None => tr!("Reading…").into(),
                Some(report) => [
                    tr_n!(
                        report.files,
                        "{count} file",
                        "{count} files",
                        count = report.files
                    ),
                    tr_n!(
                        report.messages,
                        "{count} message",
                        "{count} messages",
                        count = report.messages
                    ),
                    tr_n!(
                        report.services,
                        "{count} service",
                        "{count} services",
                        count = report.services
                    ),
                ]
                .join(" · ")
                .into(),
            };
            let problems = report.map(|r| r.problems.clone()).unwrap_or_default();
            let more = problems.len().saturating_sub(PROBLEMS_SHOWN);
            let remove_path = path.clone();
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap(px(10.))
                .p(px(10.))
                .rounded(px(12.))
                .bg(rgb(c.input))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .font_family(crate::MONO_FONT)
                                .font_features(gpui::FontFeatures::disable_ligatures())
                                .text_size(px(12.))
                                .child(path.display().to_string()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(c.muted))
                                .child(summary),
                        )
                        .children(problems.into_iter().take(PROBLEMS_SHOWN).map(|problem| {
                            div()
                                .font_family(crate::MONO_FONT)
                                .font_features(gpui::FontFeatures::disable_ligatures())
                                .text_size(px(11.5))
                                .text_color(rgb(c.error))
                                .child(problem)
                        }))
                        .when(more > 0, |d| {
                            d.child(div().text_size(px(11.5)).text_color(rgb(c.error)).child(
                                tr_n!(
                                    more,
                                    "and {count} more problem",
                                    "and {count} more problems",
                                    count = more
                                ),
                            ))
                        }),
                )
                .child(
                    div()
                        .id(ElementId::NamedInteger("proto-remove".into(), i as u64))
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
                        .child(tr!("Remove"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.remove_proto(remove_path.clone(), cx)
                        })),
                )
        });
        let busy = self.protos_busy;
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(14.))
            .rounded(px(22.))
            .bg(rgb(c.surface))
            .child(eyebrow(tr!("Protobuf schemas")).px(px(6.)))
            .child(
                div()
                    .px(px(6.))
                    .text_size(px(12.))
                    .text_color(rgb(c.muted))
                    .child(tr!("Folders of .proto files, or descriptor sets made with protoc -o, to show Protobuf and gRPC bodies with their fields by name. A gRPC call finds its messages by its path; other Protobuf by the messageType its type names. They are read again when they change.")),
            )
            .children(rows)
            .children(self.protos_problem.clone().map(|problem| {
                div()
                    .px(px(6.))
                    .text_size(px(12.))
                    .text_color(rgb(c.error))
                    .child(problem)
            }))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        button("protos-add-folder", tr!("Add a folder…"))
                            .on_click(cx.listener(|this, _, _, cx| this.ask_for_protos(true, cx))),
                    )
                    .child(
                        button("protos-add-files", tr!("Add files…"))
                            .on_click(cx.listener(|this, _, _, cx| this.ask_for_protos(false, cx))),
                    )
                    .when(!sources.is_empty() && !busy, |d| {
                        d.child(button("protos-read", tr!("Read again")).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.protos.forget();
                                this.read_protos(cx);
                            },
                        )))
                    })
                    .when(busy, |d| {
                        d.child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(c.muted))
                                .child(tr!("Reading…")),
                        )
                    }),
            )
    }
}
