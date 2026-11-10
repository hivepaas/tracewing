//! Mouse and keys, as a hand would use them, for the tests that select text.

#![allow(dead_code)]

use gpui::{
    HeadlessAppContext, Keystroke, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, PlatformInput, Point, WindowHandle,
};
use tracewing::traffic::Traffic;

pub fn mouse(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, event: PlatformInput) {
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(event, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

/// A press at `from` and a drag to `to`, as a hand would select.
pub fn drag(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    from: Point<Pixels>,
    to: Point<Pixels>,
) {
    mouse(
        cx,
        window,
        PlatformInput::MouseMove(MouseMoveEvent {
            position: from,
            pressed_button: None,
            modifiers: Modifiers::default(),
        }),
    );
    mouse(
        cx,
        window,
        PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Left,
            position: from,
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        }),
    );
    mouse(
        cx,
        window,
        PlatformInput::MouseMove(MouseMoveEvent {
            position: to,
            pressed_button: Some(MouseButton::Left),
            modifiers: Modifiers::default(),
        }),
    );
    mouse(
        cx,
        window,
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position: to,
            modifiers: Modifiers::default(),
            click_count: 1,
        }),
    );
}

pub fn right_click(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, at: Point<Pixels>) {
    mouse(
        cx,
        window,
        PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Right,
            position: at,
            modifiers: Modifiers::default(),
            click_count: 1,
            first_mouse: false,
        }),
    );
    mouse(
        cx,
        window,
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Right,
            position: at,
            modifiers: Modifiers::default(),
            click_count: 1,
        }),
    );
}

pub fn key(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, keys: &str) {
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_keystroke(Keystroke::parse(keys).unwrap(), cx);
    })
    .unwrap();
    cx.run_until_parked();
}

pub fn clipboard(cx: &mut HeadlessAppContext) -> String {
    cx.update(|cx| cx.read_from_clipboard())
        .and_then(|item| item.text())
        .expect("copied")
}
