//! The inspector in a window of its own: it shows the flow chosen in the main
//! window, takes keys pressed in it, and goes back when its window closes.
//!
//! `cargo test --release -p tracewing --features headless --test inspector_window`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, Entity, HeadlessAppContext, Keystroke, WindowHandle, px, size};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    input::TextInput,
    inspector::Part,
    layout::Place,
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
fn the_inspector_goes_to_a_window_of_its_own_and_back() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir =
        std::env::temp_dir().join(format!("tracewing-inspector-window-{}", std::process::id()));
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
        ports: 20_310..20_320,
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
    get(&mut cx, main, &format!("http://localhost:{port}/csv"));
    main.update(&mut cx, |view, _, cx| {
        view.set_screen(Screen::Traffic, cx);
    })
    .unwrap();
    wait_for(&mut cx, main, "two flows", |view| view.count() == 2);
    main.update(&mut cx, |view, _, cx| view.select_place(0, cx))
        .unwrap();

    // Out to its own window.
    main.update(&mut cx, |view, _, cx| view.detach_inspector(cx))
        .unwrap();
    cx.run_until_parked();
    let inspector = main
        .update(&mut cx, |view, _, _| {
            assert!(view.inspector_detached());
            view.inspector_window()
        })
        .unwrap()
        .expect("its window");
    shot(&mut cx, main.into(), "97-list-alone");
    shot(&mut cx, inspector, "98-inspector-window");

    // The flow chosen in the main window is the one it shows.
    let first = main
        .update(&mut cx, |view, _, cx| {
            let first = view.inspector.detail.as_ref().unwrap().id;
            view.select_place(1, cx);
            first
        })
        .unwrap();
    cx.run_until_parked();
    main.update(&mut cx, |view, _, _| {
        assert_ne!(view.inspector.detail.as_ref().unwrap().id, first);
    })
    .unwrap();

    // A key pressed in its window is the inspector's.
    let (focus, part) = main
        .update(&mut cx, |view, _, cx| {
            (gpui::Focusable::focus_handle(view, cx), view.inspector.part)
        })
        .unwrap();
    cx.update_window(inspector, |_, window, cx| {
        window.focus(&focus, cx);
        window.dispatch_keystroke(Keystroke::parse("]").unwrap(), cx);
    })
    .unwrap();
    cx.run_until_parked();
    main.update(&mut cx, |view, _, _| {
        assert_ne!(view.inspector.part, part, "the next part");
        assert_ne!(part, Part::Messages);
    })
    .unwrap();

    // Closed, it goes back, and the app goes on.
    cx.update_window(inspector, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
    main.update(&mut cx, |view, _, _| {
        assert!(!view.inspector_detached(), "back in the main window");
        assert!(view.inspector_window().is_none());
    })
    .unwrap();

    // Out again, then put below: its window closes.
    main.update(&mut cx, |view, _, cx| view.detach_inspector(cx))
        .unwrap();
    cx.run_until_parked();
    main.update(&mut cx, |view, _, cx| {
        assert!(view.inspector_detached());
        view.set_place(Place::Below, cx);
    })
    .unwrap();
    cx.run_until_parked();
    main.update(&mut cx, |view, _, _| {
        assert!(!view.inspector_detached());
        assert_eq!(view.inspector_place(), Place::Below);
    })
    .unwrap();

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}
