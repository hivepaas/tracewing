//! Drives the real window with no display, with real traffic through the real
//! proxy, and saves a picture of each state to `target/shots/`.
//!
//! `cargo test --release -p tracewing --features headless --test screen -- --nocapture`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    bodyview::BodyView,
    colors,
    detail::Side,
    engine::{Engine, EngineOptions},
    inspector::{BodyState, Part},
    layout::{Appearance, Place},
    traffic::Traffic,
};
use tracewing_dev::benchkit;
use tracewing_store::settings::StorageSettings;

fn shots() -> PathBuf {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/shots");
    std::fs::create_dir_all(&out).expect("cannot make target/shots");
    out
}

fn save(cx: &mut HeadlessAppContext, window: gpui::AnyWindowHandle, name: &str) {
    let image = cx
        .capture_screenshot(window)
        .expect("cannot draw the window");
    image
        .save(shots().join(format!("{name}.png")))
        .expect("cannot save the picture");
}

/// Looks at the session until `done` says so, or gives up.
fn wait_for(
    cx: &mut HeadlessAppContext,
    window: WindowHandle<Traffic>,
    what: &str,
    done: impl Fn(&Traffic, &gpui::App) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let ok = window
            .update(cx, |view, _, cx| {
                view.look(cx);
                done(view, cx)
            })
            .expect("window");
        cx.run_until_parked();
        if ok {
            return;
        }
        assert!(Instant::now() < deadline, "waiting for {what}");
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn send_traffic(proxy: String, ca: PathBuf, origin: String) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let client = benchkit::client_through_proxy(&proxy, &ca, false).unwrap();
        let get = |path: String| {
            let client = client.clone();
            async move {
                client
                    .get(path)
                    .send()
                    .await
                    .unwrap()
                    .bytes()
                    .await
                    .unwrap();
            }
        };
        for i in 0..150 {
            get(format!("{origin}/json?size=4000&enc=gzip&page={i}")).await;
            get(format!("{origin}/json?size=800&enc=br&user={i}")).await;
        }
        get(format!("{origin}/json?size=300000&enc=zstd&big=1")).await;
        get(format!("{origin}/json?size=2000&enc=identity&last=1")).await;
        let posted = client
            .post(format!("{origin}/json?size=100&enc=identity&posted=1"))
            .header("content-type", "application/json")
            .body(r#"{"order":"đặt hàng","qty":2}"#)
            .send()
            .await
            .unwrap();
        posted.bytes().await.unwrap();
    });
}

#[test]
fn the_window_shows_real_traffic() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-screen-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // The origin, on a runtime of its own for the length of the test.
    let origin_ca = dir.join("origin-ca.pem");
    let origin_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let origin_port = {
        let listener = origin_runtime
            .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let ca_out = origin_ca.clone();
        origin_runtime.spawn(async move { benchkit::serve_origin(listener, &ca_out).await });
        port
    };
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
        ports: 19_190..19_200,
        extra_roots: vec![origin_ca],
    })
    .unwrap();
    let proxy = format!("http://{}", engine.addr);
    let proxy_for_later = proxy.clone();
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
        .expect("cannot open a window");
    cx.run_until_parked();
    save(&mut cx, window.into(), "1-empty");

    let (ca, origin) = (
        dir.join("ca.pem"),
        format!("https://localhost:{origin_port}"),
    );
    let sender = std::thread::spawn(move || send_traffic(proxy, ca, origin));
    wait_for(&mut cx, window, "the flows", |view, _| view.count() >= 303);
    sender.join().unwrap();
    wait_for(&mut cx, window, "the last flow", |view, _| {
        view.count() == 303
    });
    save(&mut cx, window.into(), "2-traffic");

    // The newest flow: a POST, with its request in the inspector.
    window
        .update(&mut cx, |view, _, cx| {
            let last = view.count() - 1;
            view.select_place(last, cx);
            view.set_side(Side::Request, cx);
        })
        .unwrap();
    let opened = |view: &Traffic, _: &gpui::App| matches!(view.inspector.body, BodyState::Shown(_));
    wait_for(&mut cx, window, "the request body", opened);
    window
        .update(&mut cx, |view, _, cx| {
            let detail = view.inspector.detail.as_ref().expect("a flow is chosen");
            assert_eq!(detail.method, "POST");
            let headers = detail.headers(Side::Request).expect("headers were kept");
            assert!(headers.iter().any(|(n, _)| n == "content-type"));
            let BodyState::Shown(body) = &view.inspector.body else {
                panic!()
            };
            let body: &BodyView = body.read(cx);
            assert_eq!(body.body().size(), 32, "the request as it was sent");
        })
        .unwrap();
    save(&mut cx, window.into(), "3-request");

    // A body sent compressed comes out as text.
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            view.set_side(Side::Response, cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the response body", opened);
    window
        .update(&mut cx, |view, _, cx| {
            let BodyState::Shown(body) = &view.inspector.body else {
                panic!()
            };
            assert!(body.read(cx).body().size() >= 4000, "unpacked");
        })
        .unwrap();
    save(&mut cx, window.into(), "4-response");

    window
        .update(&mut cx, |view, _, cx| view.set_part(Part::Headers, cx))
        .unwrap();
    cx.run_until_parked();
    save(&mut cx, window.into(), "5-headers");

    // Filters, on a worker thread.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_part(Part::Body, cx);
            view.set_filter_text("big=1", cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the filter", |view, _| {
        !view.is_filtering() && view.count() == 1
    });
    save(&mut cx, window.into(), "6-filter");
    window
        .update(&mut cx, |view, _, cx| {
            view.set_filter_text("method:post", cx)
        })
        .unwrap();
    wait_for(&mut cx, window, "the method", |view, _| {
        !view.is_filtering() && view.count() == 1
    });
    window
        .update(&mut cx, |view, _, cx| view.set_filter_text("", cx))
        .unwrap();
    wait_for(&mut cx, window, "no filter", |view, _| view.count() == 303);

    // Rows that cannot be read must not make the list ask for all of them: that
    // once took 51 GB, at a million rows.
    window
        .update(&mut cx, |view, _, cx| view.lose_the_session(cx))
        .unwrap();
    cx.run_until_parked();
    window
        .update(&mut cx, |view, _, _| {
            assert!(view.places_asked() < 100, "{}", view.places_asked());
        })
        .unwrap();
    wait_for(&mut cx, window, "the session again", |view, _| {
        view.count() == 303
    });

    // Moved below the list, and it stays there next time.
    window
        .update(&mut cx, |view, _, cx| view.set_place(Place::Below, cx))
        .unwrap();
    cx.run_until_parked();
    // A second frame, as the list asks for when its size changes.
    window.update(&mut cx, |_, _, cx| cx.notify()).unwrap();
    cx.run_until_parked();
    save(&mut cx, window.into(), "7-below");
    let layout = std::fs::read_to_string(Path::new(&dir).join("layout.toml")).unwrap();
    assert!(layout.contains("place = \"below\""), "{layout}");

    // The other look, kept for next time, then back for the pictures after it.
    window
        .update(&mut cx, |view, window, cx| {
            view.set_appearance(Appearance::Light, window, cx)
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(colors::colors().page, 0xf4f6fb);
    save(&mut cx, window.into(), "7-below-light");
    let layout = std::fs::read_to_string(Path::new(&dir).join("layout.toml")).unwrap();
    assert!(layout.contains("appearance = \"light\""), "{layout}");
    window
        .update(&mut cx, |view, window, cx| {
            view.set_appearance(Appearance::Dark, window, cx)
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(colors::colors().page, 0x141517);

    window
        .update(&mut cx, |view, _, cx| view.begin_inspector_drag(cx))
        .unwrap();
    cx.run_until_parked();
    save(&mut cx, window.into(), "8-dragging");
    window
        .update(&mut cx, |view, _, cx| {
            view.cancel_drag(cx);
            view.set_place(Place::Beside, cx);
        })
        .unwrap();

    // A browser reaching the test page shows on Set up without anything else moving.
    let proxy_ca = dir.join("ca.pem");
    let client = benchkit::client_through_proxy(&proxy_for_later, &proxy_ca, false).unwrap();
    origin_runtime
        .block_on(async {
            client
                .get(format!("https://{}/", tracewing_proxy::capture::CHECK_HOST))
                .header("user-agent", "Firefox/144.0")
                .send()
                .await?
                .bytes()
                .await
        })
        .unwrap();
    wait_for(&mut cx, window, "the visit", |view, _| {
        view.check_visit.as_ref().is_some_and(|v| v.secure)
    });

    // A host left alone: a row that says so, and the way back.
    window
        .update(&mut cx, |view, _, cx| {
            view.change_rules(|rules| rules.leave_alone("localhost"), cx)
        })
        .unwrap();
    let origin_url = format!("https://localhost:{origin_port}/json?size=100");
    let trusts_origin =
        benchkit::client_through_proxy(&proxy_for_later, &dir.join("origin-ca.pem"), false)
            .unwrap();
    origin_runtime
        .block_on(async { trusts_origin.get(origin_url).send().await?.bytes().await })
        .unwrap();
    wait_for(&mut cx, window, "the tunnel", |view, _| view.count() == 304);
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(303, cx);
            let detail = view.inspector.detail.as_ref().expect("a flow is chosen");
            assert_eq!(detail.kind, "tunnel");
            assert_eq!(detail.method, "CONNECT");
        })
        .unwrap();
    wait_for(&mut cx, window, "the note", |view, _| {
        matches!(view.inspector.body, BodyState::Note(_))
    });
    save(&mut cx, window.into(), "15-tunnel");

    drop(cx);
    origin_runtime.shutdown_background();
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A million flows through the real proxy, and the list scrolled while more keep
/// coming. Headless, so this times the CPU's part of a frame: layout, rows from the
/// session, and paint, but not drawing the pixels.
///
/// `MILLION=1000000 cargo test --release -p tracewing --features headless --test screen a_million -- --nocapture`
#[test]
fn a_million_flows_while_traffic_keeps_coming() {
    let Some(flows) = std::env::var("MILLION").ok() else {
        eprintln!("skipped: set MILLION to how many flows");
        return;
    };
    let flows: u64 = flows.parse().expect("a number");
    let memory = |phase: &str| {
        let status = std::fs::read_to_string("/proc/self/status").unwrap();
        let rss = status
            .lines()
            .find_map(|l| l.strip_prefix("VmRSS:"))
            .unwrap_or("?")
            .trim()
            .to_owned();
        println!("[{phase}] rss {rss}");
    };
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../target/million-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let origin_ca = dir.join("origin-ca.pem");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let origin_port = listener.local_addr().unwrap().port();
    let ca_out = origin_ca.clone();
    runtime.spawn(async move { benchkit::serve_origin(listener, &ca_out).await });
    while !origin_ca.exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings::default(),
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 19_290..19_300,
        extra_roots: vec![origin_ca],
    })
    .unwrap();
    let proxy = format!("http://{}", engine.addr);
    while !dir.join("ca.pem").exists() {
        std::thread::sleep(Duration::from_millis(5));
    }
    let load = |requests: u64| benchkit::LoadOptions {
        proxy: proxy.clone(),
        url: format!("https://localhost:{origin_port}/json?size=500&enc=gzip"),
        proxy_ca: dir.join("ca.pem"),
        requests,
        concurrency: 64,
        http1_only: true,
        watch_pid: None,
    };

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
        .expect("cannot open a window");
    cx.run_until_parked();

    memory("window open");
    let report = runtime.block_on(benchkit::run_load(load(flows))).unwrap();
    report.print();
    memory("traffic sent");
    wait_for(&mut cx, window, "the flows", move |view, _| {
        view.count() as u64 >= flows
    });
    memory("list up to date");

    let timed =
        |cx: &mut HeadlessAppContext, name: &str, frames: u32, y: &dyn Fn(u32, f32) -> f32| {
            let mut times = Vec::new();
            for frame in 0..frames {
                let started = Instant::now();
                window
                    .update(cx, |view, _, cx| {
                        view.look(cx);
                        let max = view.max_scroll();
                        view.set_scroll_y(y(frame, max));
                        cx.notify();
                    })
                    .unwrap();
                times.push(started.elapsed());
            }
            times.sort_unstable();
            let at = |p: f64| times[((times.len() - 1) as f64 * p) as usize].as_secs_f64() * 1000.;
            println!(
                "{name}: p50 {:.2} ms, p95 {:.2} ms, p99 {:.2} ms, max {:.2} ms",
                at(0.5),
                at(0.95),
                at(0.99),
                at(1.0)
            );
        };
    let jump = |frame: u32, max: f32| {
        let unit = (frame.wrapping_mul(2_654_435_761) >> 8) as f32 / (1u32 << 24) as f32;
        unit * max
    };
    println!("no traffic:");
    timed(&mut cx, "  scrolling", 600, &|frame, _| frame as f32 * 40.);
    memory("after 600 frames");
    timed(&mut cx, "  jumping", 300, &jump);

    // Now with a flood coming in at the same time.
    let flood = load(flows);
    let flooding = runtime.spawn(benchkit::run_load(flood));
    std::thread::sleep(Duration::from_millis(300));
    memory("after scrolling and jumping");
    println!("with traffic:");
    timed(&mut cx, "  scrolling", 600, &|frame, _| frame as f32 * 40.);
    memory("after 600 frames");
    timed(&mut cx, "  jumping", 300, &jump);
    memory("after scrolling with traffic");
    window
        .update(&mut cx, |view, _, cx| {
            view.set_filter_text("status:200 trace", cx)
        })
        .unwrap();
    let started = Instant::now();
    wait_for(&mut cx, window, "a filter", |view, _| !view.is_filtering());
    println!(
        "  a filter over it all: {:.0} ms",
        started.elapsed().as_secs_f64() * 1000.
    );
    runtime.block_on(flooding).unwrap().unwrap().print();
    save(&mut cx, window.into(), "9-million");

    drop(cx);
    runtime.shutdown_background();
    std::fs::remove_dir_all(&dir).unwrap();
}
