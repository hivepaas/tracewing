//! The inspector in a window of its own, to put on a second display beside
//! the list. Its state stays with [`Traffic`]: this window only draws it, and
//! hands it the keys pressed here. Closed, the inspector goes back beside or
//! below the list.

use crate::text_size::ui;
use crate::{chrome, colors::colors, layout::WindowState, placement, traffic::Traffic};
use gpui::{
    AnyWindowHandle, AppContext as _, Bounds, Context, Entity, IntoElement, Render, Subscription,
    TitlebarOptions, Window, WindowBounds, WindowDecorations, WindowOptions, div, point,
    prelude::*, px, rgb, size,
};

/// Its size the first time, beside the main window.
const FIRST_SIZE: (f32, f32) = (640., 820.);

pub struct InspectorWindow {
    traffic: Entity<Traffic>,
    _redraw: Subscription,
    _bounds: Subscription,
}

impl InspectorWindow {
    fn new(traffic: Entity<Traffic>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Drawn again whenever what it shows is.
        let redraw = cx.observe(&traffic, |_, _, cx| cx.notify());
        // Where it is, for the next time it opens.
        let bounds = cx.observe_window_bounds(window, |this, window, cx| {
            let state = placement::state(window.window_bounds());
            this.traffic.update(cx, |traffic, _| {
                traffic.layout.inspector_window = Some(state);
                traffic.save_layout();
            });
        });
        Self {
            traffic,
            _redraw: redraw,
            _bounds: bounds,
        }
    }
}

impl Render for InspectorWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = colors();
        let traffic = self.traffic.clone();
        let size = traffic.read(cx).text_size();
        window.set_rem_size(crate::text_size::rem_for(size));
        let (inspector, focus, appearance) = traffic.update(cx, |traffic, cx| {
            (
                traffic.render_inspector(cx).into_any_element(),
                traffic.focus.clone(),
                traffic.layout.appearance,
            )
        });
        let keys = traffic.clone();
        let body = div()
            .track_focus(&focus)
            .key_context("Traffic")
            .on_key_down(move |event, window, cx| {
                keys.update(cx, |traffic, cx| traffic.on_key(event, window, cx));
            })
            .flex_1()
            .min_h_0()
            .w_full()
            // Close under the title bar, as the main window's toolbar is.
            .pt(ui(2.))
            .px(ui(10.))
            .pb(ui(10.))
            .flex()
            .text_color(rgb(c.text))
            .font_family(crate::ui_font())
            .text_size(ui(13.))
            .child(inspector);
        let switch = chrome::appearance_switch(appearance).on_click(move |_, window, cx| {
            traffic.update(cx, |traffic, cx| {
                let next = traffic.layout.appearance.next();
                traffic.set_appearance(next, window, cx);
            });
        });
        chrome::window_chrome(body, rgb(c.page).into(), switch, window, cx)
    }
}

impl Traffic {
    /// Whether the inspector is in a window of its own.
    pub fn inspector_detached(&self) -> bool {
        self.layout.detached && self.inspector_window.is_some()
    }

    /// Moves the inspector to a window of its own, or brings that window to
    /// the front if it is open already.
    pub fn detach_inspector(&mut self, cx: &mut Context<Self>) {
        if let Some(handle) = self.inspector_window {
            if handle
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
            {
                return;
            }
            self.inspector_window = None;
        }
        // Opened once this update is over: its first frame reads `Traffic`.
        let traffic = cx.entity();
        cx.defer(move |cx| Self::open_inspector_window(traffic, cx));
    }

    fn open_inspector_window(traffic: Entity<Traffic>, cx: &mut gpui::App) {
        if traffic.read(cx).inspector_window.is_some() {
            return;
        }
        let bounds = traffic.read(cx).inspector_window_bounds(cx);
        let client_side = cfg!(any(target_os = "linux", target_os = "freebsd"));
        let shown = traffic.clone();
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(bounds),
                window_min_size: Some(size(px(crate::layout::INSPECTOR_MIN), px(320.))),
                window_decorations: client_side.then_some(WindowDecorations::Client),
                titlebar: (!client_side).then(|| TitlebarOptions {
                    title: Some("tracewing inspector".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(chrome::TRAFFIC_LIGHTS),
                }),
                app_id: Some("tracewing".into()),
                focus: true,
                ..Default::default()
            },
            move |window, cx| cx.new(|cx| InspectorWindow::new(shown, window, cx)),
        );
        traffic.update(cx, |this, cx| {
            match opened {
                Ok(handle) => {
                    // However it closes, the inspector goes back where it was;
                    // unless the app is quitting, when it opens here again next.
                    let (id, main) = (handle.window_id(), this.main_window);
                    let me = cx.entity().downgrade();
                    this.inspector_closed_watch = Some(cx.on_window_closed(move |cx, closed| {
                        if closed != id {
                            return;
                        }
                        let quitting = !cx.windows().iter().any(|w| Some(w.window_id()) == main);
                        let _ = me.update(cx, |traffic, cx| {
                            traffic.inspector_window_gone(quitting, cx)
                        });
                    }));
                    this.inspector_window = Some(handle.into());
                    this.layout.detached = true;
                    this.layout.inspector_shown = true;
                }
                Err(e) => {
                    eprintln!("tracewing: cannot open the inspector's window: {e:#}");
                    this.layout.detached = false;
                }
            }
            this.save_layout();
            cx.notify();
        });
    }

    /// Puts the inspector back in the main window, closing its own.
    pub fn dock_inspector(&mut self, cx: &mut Context<Self>) {
        // Its closing is not heard of again: this is it.
        self.inspector_closed_watch = None;
        if let Some(handle) = self.inspector_window.take() {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        }
        self.inspector_window_gone(false, cx);
    }

    /// Its window has closed, however it was.
    fn inspector_window_gone(&mut self, quitting: bool, cx: &mut Context<Self>) {
        self.inspector_window = None;
        self.inspector_closed_watch = None;
        if !quitting {
            self.layout.detached = false;
            self.save_layout();
        }
        cx.notify();
    }

    /// Where it was last, or beside the main window.
    fn inspector_window_bounds(&self, cx: &gpui::App) -> WindowBounds {
        if let Some(WindowState {
            x,
            y,
            width,
            height,
            ..
        }) = self.layout.inspector_window
        {
            return WindowBounds::Windowed(Bounds {
                origin: point(px(x), px(y)),
                size: size(px(width), px(height)),
            });
        }
        let first = size(px(FIRST_SIZE.0), px(FIRST_SIZE.1));
        WindowBounds::Windowed(match self.layout.window {
            Some(main) => Bounds {
                origin: point(px(main.x + main.width + 16.), px(main.y)),
                size: first,
            },
            None => Bounds::centered(None, first, cx),
        })
    }

    /// Opens the inspector's window again at start, if it was left open.
    pub fn reopen_inspector_window(&mut self, cx: &mut Context<Self>) {
        if self.layout.detached && self.inspector_window.is_none() {
            self.detach_inspector(cx);
        }
    }

    pub fn inspector_window(&self) -> Option<AnyWindowHandle> {
        self.inspector_window
    }

    /// Where the inspector goes in the main window.
    pub fn inspector_place(&self) -> crate::layout::Place {
        self.layout.place
    }
}
