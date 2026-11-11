//! The `.proto` files chosen in Settings, which name the fields of Protobuf
//! and gRPC bodies.

use crate::{
    colors::colors,
    detail::{Side, host_and_path},
    input::{Changed, TextInput},
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

/// How many messages the list to read a body as shows at once.
const MESSAGES_SHOWN: usize = 200;

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

    /// Asks the server whose address is written for its files by gRPC
    /// reflection, keeps them, and reads them with the rest.
    pub fn fetch_protos(&mut self, cx: &mut Context<Self>) {
        let Some(input) = &self.protos_server else {
            return;
        };
        if self.protos_fetching {
            return;
        }
        let address = input.read(cx).text().to_owned();
        self.protos_fetching = true;
        self.protos_fetched = None;
        let protos = Arc::clone(&self.protos);
        let (proxy, ca) = (self.engine.addr, self.data_dir.join("ca.pem"));
        let dir = self.data_dir.join("reflection");
        cx.spawn(async move |this, cx| {
            let done = cx
                .background_spawn(async move {
                    let fetched = crate::reflection::fetch(&address, proxy, &ca, &dir)
                        .map_err(|e| format!("{e:#}"))?;
                    let schema = protos
                        .add([fetched.path.clone()])
                        .map_err(|e| tr!("The list was not saved: {e}", e = e.to_string()))?;
                    Ok::<_, String>((fetched, schema))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.protos_fetching = false;
                this.protos_fetched = Some(match done {
                    Ok((fetched, schema)) => {
                        this.protos_read = Some(schema);
                        this.load_body(cx);
                        Ok(tr_n!(
                            fetched.services.len(),
                            "{count} service: {names}",
                            "{count} services: {names}",
                            count = fetched.services.len(),
                            names = fetched.services.join(", ")
                        ))
                    }
                    Err(problem) => Err(problem),
                });
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
        let server = self
            .protos_server
            .get_or_insert_with(|| {
                cx.new(|cx| {
                    TextInput::said(
                        || tr!("https://api.example.com or http://localhost:50051"),
                        cx,
                    )
                })
            })
            .clone();
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
        let fetching = self.protos_fetching;
        let fetched = self.protos_fetched.clone();
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
            .child(
                div()
                    .px(px(6.))
                    .pt(px(4.))
                    .text_size(px(12.))
                    .text_color(rgb(c.muted))
                    .child(tr!("Or ask a server that offers gRPC reflection for its files. https:// goes through tracewing; http:// is gRPC with no TLS, straight to the server.")),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h(px(32.))
                            .px(px(12.))
                            .rounded(px(16.))
                            .flex()
                            .items_center()
                            .bg(rgb(c.input))
                            .font_family(crate::MONO_FONT)
                            .font_features(gpui::FontFeatures::disable_ligatures())
                            .text_size(px(12.))
                            .on_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, _, cx| {
                                if ev.keystroke.key == "enter" {
                                    this.fetch_protos(cx);
                                    cx.stop_propagation();
                                }
                            }))
                            .child(server),
                    )
                    .child(if fetching {
                        button("protos-fetch", tr!("Asking…"))
                    } else {
                        button("protos-fetch", tr!("Fetch"))
                            .on_click(cx.listener(|this, _, _, cx| this.fetch_protos(cx)))
                    }),
            )
            .children(fetched.map(|fetched| {
                let (text, color) = match fetched {
                    Ok(text) => (text, c.ok),
                    Err(text) => (text, c.error),
                };
                div()
                    .px(px(6.))
                    .text_size(px(12.))
                    .text_color(rgb(color))
                    .child(text)
            }))
    }

    /// The host, path and side of the body shown, if it is Protobuf that
    /// can be read as a message: there are `.proto` files to choose from.
    fn read_as_target(&self) -> Option<(String, String, Side)> {
        use crate::inspector::{BodyState, Part};
        use tracewing_store::binjson;
        let detail = self.inspector.detail.as_ref()?;
        if self.inspector.part != Part::Body
            || !matches!(self.inspector.body, BodyState::Shown(..))
            || self.protos.sources().is_empty()
        {
            return None;
        }
        let side = self.inspector.side;
        let content_type = match side {
            Side::Request => detail.req_content_type.as_deref(),
            Side::Response => detail.content_type.as_deref(),
        };
        binjson::format_of(content_type).filter(|f| f.numbered())?;
        let (host, path) = host_and_path(&detail.url);
        Some((host.to_owned(), path.to_owned(), side))
    }

    /// Opens or closes the list of messages to read the body as.
    pub fn toggle_read_as(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        if self.inspector.read_as.take().is_some() {
            cx.notify();
            return;
        }
        let input = cx.new(|cx| TextInput::said(|| tr!("Find a message"), cx));
        cx.subscribe(&input, |_, _, _: &Changed, cx| cx.notify())
            .detach();
        gpui::Focusable::focus_handle(input.read(cx), cx).focus(window, cx);
        self.inspector.read_as = Some(input);
        cx.notify();
    }

    /// Reads the body shown as `message`, and every body that side of its
    /// path sends after; with `None`, as the `.proto` files say by
    /// themselves.
    pub fn read_body_as(&mut self, message: Option<String>, cx: &mut Context<Self>) {
        let Some((host, path, side)) = self.read_as_target() else {
            return;
        };
        if let Err(e) = self
            .protos
            .choose(&host, &path, side == Side::Request, message.as_deref())
        {
            self.protos_problem = Some(tr!("The list was not saved: {e}", e = e.to_string()));
        }
        self.load_body(cx);
        cx.notify();
    }

    pub(crate) fn render_read_as_button(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Stateful<gpui::Div>> {
        let c = colors();
        let (host, path, side) = self.read_as_target()?;
        let label = match self.protos.chosen(&host, &path, side == Side::Request) {
            Some(name) => tr!("Read as {message}", message = short_name(&name).to_owned()),
            None => tr!("Read as…"),
        };
        let open = self.inspector.read_as.is_some();
        Some(
            div()
                .id("read-as")
                .h(px(28.))
                .px(px(10.))
                .rounded(px(14.))
                .flex()
                .items_center()
                .cursor_pointer()
                .reachable()
                .when(open, |d| d.bg(rgb(c.accent)).text_color(rgb(c.on_accent)))
                .when(!open, |d| d.bg(rgb(c.chip)).text_color(rgb(c.on_chip)))
                .text_size(px(12.))
                .font_weight(FontWeight::BOLD)
                .child(label)
                .on_click(cx.listener(|this, _, window, cx| this.toggle_read_as(window, cx))),
        )
    }

    /// The list of messages, narrowed by what is typed, in place of the body.
    pub(crate) fn render_read_as(&self, cx: &mut Context<Self>) -> Option<gpui::Div> {
        let c = colors();
        let input = self.inspector.read_as.clone()?;
        let (host, path, side) = self.read_as_target()?;
        let chosen = self.protos.chosen(&host, &path, side == Side::Request);
        let wanted = input.read(cx).text().to_lowercase();
        let names = self.protos.last_schema().message_names();
        let matching: Vec<String> = names
            .into_iter()
            .filter(|name| wanted.is_empty() || name.to_lowercase().contains(&wanted))
            .collect();
        let more = matching.len().saturating_sub(MESSAGES_SHOWN);
        let line = |id: ElementId, label: SharedString, on: bool| {
            div()
                .id(id)
                .px(px(10.))
                .py(px(5.))
                .rounded(px(8.))
                .cursor_pointer()
                .reachable()
                .font_family(crate::MONO_FONT)
                .font_features(gpui::FontFeatures::disable_ligatures())
                .text_size(px(12.))
                .when(on, |d| d.bg(rgb(c.tonal)).text_color(rgb(c.on_tonal)))
                .hover(|d| d.bg(rgb(c.chip)))
                .child(label)
        };
        let where_ = match side {
            Side::Request => tr!(
                "Requests to {path} on {host} are read as the message chosen.",
                path = path.split('?').next().unwrap_or(&path).to_owned(),
                host = host.clone()
            ),
            Side::Response => tr!(
                "Answers from {path} on {host} are read as the message chosen.",
                path = path.split('?').next().unwrap_or(&path).to_owned(),
                host = host.clone()
            ),
        };
        Some(
            div()
                .size_full()
                .flex()
                .flex_col()
                .gap(px(8.))
                .p(px(12.))
                .on_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, _, cx| {
                    if ev.keystroke.key == "escape" {
                        this.inspector.read_as = None;
                        cx.stop_propagation();
                        cx.notify();
                    }
                }))
                .child(
                    div()
                        .h(px(30.))
                        .px(px(10.))
                        .rounded(px(15.))
                        .flex()
                        .items_center()
                        .bg(rgb(c.input))
                        .text_size(px(12.5))
                        .child(div().flex_1().min_w_0().child(input)),
                )
                .child(
                    div()
                        .px(px(4.))
                        .text_size(px(12.))
                        .text_color(rgb(c.muted))
                        .child(where_),
                )
                .child(
                    div()
                        .id("read-as-list")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap(px(1.))
                        .child(
                            line(
                                "read-as-found".into(),
                                tr!("Automatic (gRPC path or messageType)").into(),
                                chosen.is_none(),
                            )
                            .font_family(crate::FONT)
                            .on_click(cx.listener(|this, _, _, cx| this.read_body_as(None, cx))),
                        )
                        .children(matching.into_iter().take(MESSAGES_SHOWN).enumerate().map(
                            |(i, name)| {
                                let on = chosen.as_deref() == Some(name.as_str());
                                let picked = name.clone();
                                line(
                                    ElementId::NamedInteger("read-as-message".into(), i as u64),
                                    name.into(),
                                    on,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.read_body_as(Some(picked.clone()), cx)
                                    },
                                ))
                            },
                        ))
                        .when(more > 0, |d| {
                            d.child(
                                div()
                                    .px(px(10.))
                                    .py(px(5.))
                                    .text_size(px(12.))
                                    .text_color(rgb(c.muted))
                                    .child(tr_n!(
                                        more,
                                        "and {count} more: type to narrow them",
                                        "and {count} more: type to narrow them",
                                        count = more
                                    )),
                            )
                        }),
                ),
        )
    }
}

/// `Order` of `shop.v1.Order`: the button is small.
fn short_name(full: &str) -> &str {
    full.rsplit('.').next().unwrap_or(full)
}
