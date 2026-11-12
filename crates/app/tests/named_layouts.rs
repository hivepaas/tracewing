//! Named layouts: the panels kept under a name, and put back as they were,
//! the inspector's own window with them.
//!
//! `cargo test --release -p tracewing --features headless --test named_layouts`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, Entity, HeadlessAppContext, Keystroke, WindowHandle, px, size};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    columns::Column,
    engine::{Engine, EngineOptions},
    input::TextInput,
    layout::Place,
    shortcuts::Action,
    traffic::{Screen, Traffic},
};
use tracewing_store::settings::StorageSettings;

fn wait_for(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    what: &str,
    done: impl Fn(&Traffic) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let ok = window
            .update(cx, |view, _, cx| {
                view.look(cx);
                done(view)
            })
            .unwrap();
        cx.run_until_parked();
        if ok {
            return;
        }
        assert!(Instant::now() < deadline, "waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn shot(cx: &mut HeadlessAppContext, window: gpui::AnyWindowHandle, name: &str) {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/shots");
    std::fs::create_dir_all(&out).unwrap();
    cx.capture_screenshot(window)
        .unwrap()
        .save(out.join(format!("{name}.png")))
        .unwrap();
}

fn type_in(input: &Entity<TextInput>, text: &str, cx: &mut gpui::Context<Traffic>) {
    input.update(cx, |input, cx| input.set_text(text, 0..0, cx));
}

fn get(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, url: &str) {
    window
        .update(cx, |view, _, cx| {
            view.new_compose(cx);
            let page = view.compose_page.as_ref().unwrap();
            type_in(&page.url.clone(), url, cx);
            view.send_composed(cx);
        })
        .unwrap();
    wait_for(cx, window, "the answer", |view| {
        view.compose_page
            .as_ref()
            .is_some_and(|page| page.answer.is_some() || page.problem.is_some())
    });
}

#[test]
fn a_layout_kept_under_a_name_comes_back() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-named-layouts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    runtime.spawn(tracewing_dev::playground::serve(listener));
    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_320..20_330,
        extra_roots: Vec::new(),
        key_store: Default::default(),
        find_apps: false,
    })
    .unwrap();
    let text = Arc::new(gpui_wgpu::CosmicTextSystem::new(tracewing::FONT));
    let mut cx = HeadlessAppContext::with_platform(text, Arc::new(tracewing::Assets), || {
        gpui_platform::current_headless_renderer()
    });
    cx.update(tracewing::load_fonts);
    let data_dir = dir.clone();
    let main: WindowHandle<Traffic> = cx
        .open_window(size(px(1440.), px(900.)), move |window, cx| {
            cx.new(|cx| Traffic::new(engine, &data_dir, window, cx))
        })
        .unwrap();

    get(&mut cx, main, &format!("http://localhost:{port}/json"));
    main.update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, main, "the flow", |view| view.count() == 1);
    main.update(&mut cx, |view, _, cx| view.select_place(0, cx))
        .unwrap();

    // As it starts: kept as "API".
    main.update(&mut cx, |view, _, cx| {
        type_in(&view.layout_name.clone(), "API", cx);
        view.keep_layout(cx);
        assert_eq!(view.layouts().current_name(), Some("API"));
        assert_eq!(view.layout_name.read(cx).text(), "", "the field is emptied");
    })
    .unwrap();

    // Below, no sidebar, no Size column, the inspector in its own window:
    // kept as "Phone", with Enter in the field.
    main.update(&mut cx, |view, window, cx| {
        view.set_place(Place::Below, cx);
        view.do_key(Action::Sidebar, window, cx);
        view.toggle_column(Column::Size, cx);
        view.detach_inspector(cx);
    })
    .unwrap();
    cx.run_until_parked();
    let field = main
        .update(&mut cx, |view, _, cx| {
            assert!(view.inspector_detached());
            assert_eq!(view.layouts().current_name(), None);
            view.set_layouts_menu(true, cx);
            type_in(&view.layout_name.clone(), "Phone", cx);
            gpui::Focusable::focus_handle(view.layout_name.read(cx), cx)
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(main.into(), |_, window, cx| {
        window.focus(&field, cx);
        window.dispatch_keystroke(Keystroke::parse("enter").unwrap(), cx);
    })
    .unwrap();
    cx.run_until_parked();
    main.update(&mut cx, |view, _, _| {
        assert_eq!(view.layouts().current_name(), Some("Phone"));
        assert_eq!(view.layouts().named.len(), 2);
    })
    .unwrap();

    // Back to "API": beside, all there, the inspector's window closed.
    main.update(&mut cx, |view, _, cx| view.use_layout("API", cx))
        .unwrap();
    cx.run_until_parked();
    main.update(&mut cx, |view, _, cx| {
        let layout = view.layouts();
        assert_eq!(layout.place, Place::Beside);
        assert!(layout.sidebar_shown);
        assert!(layout.columns.is_shown(Column::Size));
        assert!(!view.inspector_detached());
        assert!(view.inspector_window().is_none());
        assert!(!view.layouts_menu_open(), "the menu closes");
        view.set_layouts_menu(true, cx);
    })
    .unwrap();
    cx.run_until_parked();
    shot(&mut cx, main.into(), "99-layouts-menu");

    // And to "Phone", whatever the case: its window opens again.
    main.update(&mut cx, |view, _, cx| view.use_layout("phone", cx))
        .unwrap();
    cx.run_until_parked();
    main.update(&mut cx, |view, _, _| {
        let layout = view.layouts();
        assert_eq!(layout.place, Place::Below);
        assert!(!layout.sidebar_shown);
        assert!(!layout.columns.is_shown(Column::Size));
        assert!(view.inspector_detached());
        assert_eq!(layout.current_name(), Some("Phone"));
    })
    .unwrap();

    // Escape closes the menu; forgotten, a layout is gone.
    main.update(&mut cx, |view, window, cx| {
        view.set_layouts_menu(true, cx);
        assert!(view.do_key(Action::Close, window, cx));
        assert!(!view.layouts_menu_open());
        view.forget_layout("api", cx);
        assert_eq!(view.layouts().named.len(), 1);
    })
    .unwrap();

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}
