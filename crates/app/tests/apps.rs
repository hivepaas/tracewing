//! The program behind each flow: the list says it, the sidebar counts it and
//! chooses it, and the inspector names it. The test sends some flows itself and
//! has curl send others.
//!
//! `cargo test --release -p tracewing --features headless --test apps`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, Entity, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::PathBuf,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    input::TextInput,
    inspector::Part,
    traffic::{Screen, Traffic},
};
use tracewing_dev::benchkit;
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

/// Sends a GET from the compose screen, which is this program.
fn send(cx: &mut HeadlessAppContext, window: WindowHandle<Traffic>, url: &str) {
    window
        .update(cx, |view, _, cx| {
            view.new_compose(cx);
            let page = view.compose_page.as_ref().unwrap();
            for (input, text) in [(page.method.clone(), "GET"), (page.url.clone(), url)] {
                type_in(&input, text, cx);
            }
            view.send_composed(cx);
        })
        .unwrap();
    wait_for(cx, window, "the answer", |view| {
        view.compose_page
            .as_ref()
            .is_some_and(|page| page.answer.is_some() || page.problem.is_some())
    });
    window
        .update(cx, |view, _, _| {
            let page = view.compose_page.as_ref().unwrap();
            assert_eq!(page.problem, None);
        })
        .unwrap();
}

#[test]
fn each_flow_says_which_program_sent_it() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-apps-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let origin_ca = dir.join("origin-ca.pem");
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let origin = listener.local_addr().unwrap().port();
    let ca_out = origin_ca.clone();
    runtime.spawn(async move { benchkit::serve_origin(listener, &ca_out).await });
    while !origin_ca.exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_190..20_200,
        extra_roots: vec![origin_ca],
        key_store: Default::default(),
        find_apps: true,
    })
    .unwrap();
    while !dir.join("ca.pem").exists() {
        std::thread::sleep(Duration::from_millis(5));
    }

    let text = Arc::new(gpui_wgpu::CosmicTextSystem::new(tracewing::FONT));
    let mut cx = HeadlessAppContext::with_platform(text, Arc::new(tracewing::Assets), || {
        gpui_platform::current_headless_renderer()
    });
    cx.update(tracewing::load_fonts);
    let data_dir = dir.clone();
    let window: WindowHandle<Traffic> = cx
        .open_window(size(px(1440.), px(900.)), move |window, cx| {
            cx.new(|cx| Traffic::new(engine, &data_dir, window, cx))
        })
        .unwrap();
    let proxy = window
        .update(&mut cx, |view, _, _| view.proxy_addr())
        .unwrap();

    send(
        &mut cx,
        window,
        &format!("https://localhost:{origin}/json?size=20"),
    );
    for size in [5, 6] {
        let out = Command::new("curl")
            .args(["-sS", "-o", "/dev/null", "--proxy"])
            .arg(format!("http://{proxy}"))
            .arg("--cacert")
            .arg(dir.join("ca.pem"))
            .arg(format!("https://localhost:{origin}/json?size={size}"))
            .output()
            .expect("curl");
        assert!(out.status.success(), "{out:?}");
    }

    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "three flows", |view| view.count() == 3);
    let me = std::env::current_exe().unwrap();
    let me = me.file_stem().unwrap().to_str().unwrap().to_owned();
    let expected = me.clone();
    // Said with what was actually counted: on a machine where a socket's
    // owner cannot be looked up, "waiting for both programs counted" alone
    // says nothing about why.
    let counts_now = |cx: &mut HeadlessAppContext| -> Vec<(String, u64)> {
        window
            .update(cx, |view, _, cx| {
                view.look(cx);
                view.app_counts()
                    .iter()
                    .map(|(app, n)| (app.to_string(), *n))
                    .collect()
            })
            .unwrap()
    };
    let want = vec![("curl".to_owned(), 2), (expected.clone(), 1)];
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut counts = counts_now(&mut cx);
    while counts != want {
        cx.run_until_parked();
        assert!(
            Instant::now() < deadline,
            "waiting for both programs counted: wanted {want:?}, and the list says {counts:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
        counts = counts_now(&mut cx);
    }
    shot(&mut cx, window.into(), "54-apps");

    // Chosen in the sidebar, then typed in the box.
    window
        .update(&mut cx, |view, _, cx| view.set_app(Some("curl".into()), cx))
        .unwrap();
    wait_for(&mut cx, window, "curl's flows", |view| {
        !view.is_filtering() && view.count() == 2
    });
    window
        .update(&mut cx, |view, _, cx| {
            view.set_app(None, cx);
            view.set_filter_text("app:cur", cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "curl's flows again", |view| {
        !view.is_filtering() && view.count() == 2
    });
    window
        .update(&mut cx, |view, _, cx| view.set_filter_text("", cx))
        .unwrap();
    wait_for(&mut cx, window, "all flows", |view| view.count() == 3);

    // The inspector names the program, and where it is.
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            let detail = view.inspector.detail.as_ref().unwrap();
            assert_eq!(detail.app.as_deref(), Some(me.as_str()));
            assert_eq!(detail.app_pid, Some(std::process::id()));
            assert!(detail.app_path.as_deref().unwrap().ends_with(&me));
            view.set_part(Part::Connection, cx);
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "55-client");
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(1, cx);
            let detail = view.inspector.detail.as_ref().unwrap();
            assert_eq!(detail.app.as_deref(), Some("curl"));
            assert_ne!(detail.app_pid, Some(std::process::id()));
        })
        .unwrap();

    std::fs::remove_dir_all(&dir).ok();
}
