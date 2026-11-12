//! Layouts kept under a name, as "Phone" and "API", to switch between: where
//! the panels are and how large, which columns show. The toolbar's layouts
//! button keeps the one in use under a name and goes back to one.

use crate::text_size::ui;
use crate::{
    colors::colors,
    layout::Layout,
    traffic::Traffic,
    widgets::{Reachable, eyebrow, icon},
};
use gpui::{
    Anchor, Context, FontWeight, IntoElement, KeyDownEvent, SharedString, Window, anchored,
    deferred, div, prelude::*, px, rgb,
};
use tracewing_i18n::tr;

impl Traffic {
    pub fn set_layouts_menu(&mut self, open: bool, cx: &mut Context<Self>) {
        self.layouts_menu = open;
        cx.notify();
    }

    pub fn layouts_menu_open(&self) -> bool {
        self.layouts_menu
    }

    pub fn layouts(&self) -> &Layout {
        &self.layout
    }

    /// Keeps the panels as they are now under the name typed.
    pub fn keep_layout(&mut self, cx: &mut Context<Self>) {
        let name = self.layout_name.read(cx).text().to_owned();
        if !self.layout.keep_as(&name) {
            return;
        }
        self.layout_name
            .update(cx, |input, cx| input.set_text("", 0..0, cx));
        self.save_layout();
        cx.notify();
    }

    /// Puts the panels as the layout of that name has them.
    pub fn use_layout(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(arrangement) = self.layout.named_arrangement(name) else {
            return;
        };
        // What else changes with them, first.
        self.set_sides(arrangement.sides, cx);
        let was_detached = self.inspector_detached();
        self.layout.arrange(&arrangement);
        if let Some(sort) = self.filter.sort
            && !self.layout.columns.is_shown(sort.column)
        {
            self.set_sort(None, cx);
        }
        match (was_detached, arrangement.detached) {
            (false, true) => {
                self.layout.detached = false;
                self.detach_inspector(cx);
            }
            (true, false) => {
                self.layout.detached = true;
                self.dock_inspector(cx);
            }
            _ => {}
        }
        self.layouts_menu = false;
        self.save_layout();
        cx.notify();
    }

    pub fn forget_layout(&mut self, name: &str, cx: &mut Context<Self>) {
        self.layout.forget(name);
        self.save_layout();
        cx.notify();
    }

    /// The toolbar's button, and the menu it opens.
    pub(crate) fn render_layouts_button(
        &self,
        button: impl Fn(&'static str, &'static str, bool) -> gpui::Stateful<gpui::Div>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open = self.layouts_menu;
        div()
            .relative()
            .child(
                button("layouts", "layouts", open)
                    .on_click(cx.listener(move |this, _, _, cx| this.set_layouts_menu(!open, cx))),
            )
            .when(open, |d| {
                d.child(
                    div().absolute().top(ui(40.)).left_0().child(deferred(
                        anchored()
                            .anchor(Anchor::TopLeft)
                            .snap_to_window_with_margin(px(8.))
                            .child(self.render_layouts_menu(cx)),
                    )),
                )
            })
    }

    fn render_layouts_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let current = self.layout.current_name().map(str::to_owned);
        let rows = self.layout.named.iter().enumerate().map(|(i, named)| {
            let name = named.name.clone();
            let on = current.as_deref() == Some(name.as_str());
            let (used, forgotten) = (name.clone(), name.clone());
            div()
                .id(SharedString::from(format!("layout-{i}")))
                .h(ui(30.))
                .pl(ui(10.))
                .pr(ui(4.))
                .rounded(ui(8.))
                .flex()
                .flex_row()
                .items_center()
                .gap(ui(8.))
                .cursor_pointer()
                .reachable()
                .hover(|d| d.bg(rgb(c.tonal)))
                .child(
                    div()
                        .w(ui(14.))
                        .flex_shrink_0()
                        .children(on.then(|| icon("check", 14., c.text))),
                )
                .child(div().flex_1().min_w_0().truncate().child(name))
                .child(
                    div()
                        .id(SharedString::from(format!("forget-layout-{i}")))
                        .size(ui(24.))
                        .rounded(ui(6.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .reachable()
                        .hover(|d| d.bg(rgb(c.surface)))
                        .child(icon("close", 12., c.muted))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.forget_layout(&forgotten, cx)
                        })),
                )
                .on_click(cx.listener(move |this, _, _, cx| this.use_layout(&used, cx)))
        });
        let empty = self.layout.named.is_empty();
        div()
            .id("layouts-list")
            .occlude()
            .w(ui(260.))
            .p(ui(6.))
            .rounded(ui(14.))
            .border_1()
            .border_color(rgb(c.line))
            .bg(rgb(c.surface))
            .shadow_lg()
            .flex()
            .flex_col()
            .text_size(ui(13.))
            .font_weight(FontWeight::NORMAL)
            .text_color(rgb(c.text))
            .child(eyebrow(tr!("Layouts")).px(ui(10.)).py(ui(4.)))
            .when(empty, |d| {
                d.child(
                    div()
                        .px(ui(10.))
                        .py(ui(4.))
                        .text_color(rgb(c.muted))
                        .child(tr!(
                            "Keep the panels as they are now under a name, to come back to them."
                        )),
                )
            })
            .children(rows)
            .child(div().h(ui(1.)).my(ui(4.)).bg(rgb(c.line)))
            .child(
                div()
                    .p(ui(4.))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(ui(6.))
                    .child(
                        div()
                            .id("layout-name")
                            .flex_1()
                            .min_w_0()
                            .h(ui(28.))
                            .px(ui(10.))
                            .rounded(ui(10.))
                            .flex()
                            .items_center()
                            .bg(rgb(c.input))
                            .font_family(crate::FONT)
                            .child(self.layout_name.clone())
                            .on_key_down(cx.listener(
                                |this, event: &KeyDownEvent, _: &mut Window, cx| {
                                    if event.keystroke.key == "enter" {
                                        cx.stop_propagation();
                                        this.keep_layout(cx);
                                    }
                                },
                            )),
                    )
                    .child(
                        div()
                            .id("keep-layout")
                            .h(ui(28.))
                            .px(ui(12.))
                            .rounded(ui(14.))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .reachable()
                            .bg(rgb(c.chip))
                            .text_color(rgb(c.on_chip))
                            .text_size(ui(12.))
                            .font_weight(FontWeight::BOLD)
                            .child(tr!("Keep"))
                            .on_click(cx.listener(|this, _, _, cx| this.keep_layout(cx))),
                    ),
            )
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.set_layouts_menu(false, cx)))
    }
}
