//! Other devices: the sidebar counts what each sent and chooses it, the box finds
//! it by address or name, and the inspector names it. This machine's address on
//! its network stands in for another device, with programs not looked for, so
//! nothing says what sent it.
//!
//! `cargo test --release -p tracewing --features headless --test devices`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, HeadlessAppContext, WindowHandle, px, size};
use std::{
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    devices::{Devices, on_the_network},
    engine::{Engine, EngineOptions},
    inspector::Part,
    setup_page::Guide,
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

fn curl(proxy: SocketAddr, ca: &Path, url: &str) {
    let out = Command::new("curl")
        .args(["-sS", "-o", "/dev/null", "--proxy"])
        .arg(format!("http://{proxy}"))
        .arg("--cacert")
        .arg(ca)
        .arg(url)
        .output()
        .expect("curl");
    assert!(out.status.success(), "{out:?}");
}

fn device_counts(view: &Traffic) -> Vec<(String, u64)> {
    view.device_counts()
        .iter()
        .map(|(ip, n)| (ip.to_string(), *n))
        .collect()
}

#[test]
fn other_devices_are_counted_found_and_named() {
    let Some(here) = on_the_network() else {
        eprintln!("skipped: this machine has no address on a network");
        return;
    };
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-devices-{}", std::process::id()));
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
            other_devices: true,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_200..20_210,
        extra_roots: vec![origin_ca],
        key_store: Default::default(),
        find_apps: false,
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
    let own = window
        .update(&mut cx, |view, _, _| view.proxy_addr())
        .unwrap();
    assert!(own.ip().is_loopback(), "this machine's own way in: {own}");

    // Two from the "device", one from this machine's loopback.
    let ca = dir.join("ca.pem");
    let lan = SocketAddr::new(here, own.port());
    for size in [5, 6] {
        curl(
            lan,
            &ca,
            &format!("https://localhost:{origin}/json?size={size}"),
        );
    }
    curl(own, &ca, &format!("https://localhost:{origin}/json?size=7"));

    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Traffic, cx))
        .unwrap();
    wait_for(&mut cx, window, "three flows", |view| view.count() == 3);
    let ip = match here {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => v6.to_canonical().to_string(),
    };
    let expected = ip.clone();
    wait_for(&mut cx, window, "the device counted", move |view| {
        device_counts(view) == [(expected.clone(), 2)]
    });
    shot(&mut cx, window.into(), "56-devices");

    // Chosen in the sidebar, then typed in the box by its address.
    window
        .update(&mut cx, |view, _, cx| view.set_device(Some(ip.clone()), cx))
        .unwrap();
    wait_for(&mut cx, window, "the device's flows", |view| {
        !view.is_filtering() && view.count() == 2
    });
    window
        .update(&mut cx, |view, _, cx| {
            view.set_device(None, cx);
            view.set_filter_text(&format!("device:{ip}"), cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the device's flows by address", |view| {
        !view.is_filtering() && view.count() == 2
    });

    // Named from one of its flows. The box keeps it, and finds it by name.
    window
        .update(&mut cx, |view, _, cx| view.set_filter_text("", cx))
        .unwrap();
    wait_for(&mut cx, window, "all flows", |view| view.count() == 3);
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(0, cx);
            assert!(view.inspector.detail.as_ref().unwrap().app.is_none());
            view.set_part(Part::Connection, cx);
            let input = view.device_name_input().clone();
            input.update(cx, |input, cx| input.set_text("Test phone", 0..0, cx));
        })
        .unwrap();
    cx.run_until_parked();
    window
        .update(&mut cx, |view, _, _| {
            assert_eq!(view.devices().name(&ip), Some("Test phone"));
        })
        .unwrap();
    assert_eq!(
        Devices::load(&dir.join("devices.toml")).name(&ip),
        Some("Test phone"),
        "kept for next time"
    );
    shot(&mut cx, window.into(), "57-device-name");
    window
        .update(&mut cx, |view, _, cx| {
            view.set_filter_text("device:phone", cx)
        })
        .unwrap();
    wait_for(&mut cx, window, "the device's flows by name", |view| {
        !view.is_filtering() && view.count() == 2
    });

    // This machine's own flow has no device, so nothing to name.
    window
        .update(&mut cx, |view, _, cx| view.set_filter_text("", cx))
        .unwrap();
    wait_for(&mut cx, window, "all flows again", |view| view.count() == 3);
    window
        .update(&mut cx, |view, _, cx| {
            view.select_place(2, cx);
            assert_eq!(view.device_name_input().read(cx).text(), "");
            let detail = view.inspector.detail.as_ref().unwrap();
            assert!(detail.client.starts_with("127.0.0.1:"), "{}", detail.client);
        })
        .unwrap();

    // The Set up page has a code for a phone to scan, and how to trust the
    // certificate on each kind.
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Setup, cx))
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "61-setup-phones");
    // Down to the devices and what to undo on them, which tracewing cannot.
    window
        .update(&mut cx, |view, _, _| {
            assert!(!view.device_counts().is_empty(), "a device went through");
        })
        .unwrap();
    let scroll = |cx: &mut HeadlessAppContext, by: f32| {
        cx.update_window(window.into(), |_, window, cx| {
            window.dispatch_event(
                gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position: gpui::point(px(900.), px(500.)),
                    delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-by))),
                    ..Default::default()
                }),
                cx,
            );
        })
        .unwrap();
        cx.run_until_parked();
    };
    scroll(&mut cx, 820.);
    shot(&mut cx, window.into(), "95-setup-devices-to-undo");

    // iOS simulators, as on a Mac: here an `xcrun` that answers as simctl does
    // and writes down what it was asked.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let asked = dir.join("asked");
        let fake = dir.join("xcrun");
        std::fs::write(
            &fake,
            format!(
                "#!/bin/sh\necho \"$@\" >> {asked}\n\
                 case \"$2\" in list) echo '{{\"devices\":{{\"com.apple.CoreSimulator.SimRuntime.iOS-18-2\":\
                 [{{\"udid\":\"A1\",\"name\":\"iPhone 16\",\"state\":\"Booted\"}},\
                 {{\"udid\":\"A2\",\"name\":\"iPad Air\",\"state\":\"Booted\"}}]}}}}' ;; esac\n",
                asked = asked.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        window
            .update(&mut cx, |view, _, cx| {
                view.system.ios.xcrun = Some(Some(fake.clone()));
                view.find_simulators(cx);
            })
            .unwrap();
        wait_for(&mut cx, window, "the simulators", |view| {
            view.system.ios.simulators.len() == 2
        });
        window
            .update(&mut cx, |view, _, cx| view.trust_simulator("A1".into(), cx))
            .unwrap();
        wait_for(&mut cx, window, "the certificate trusted", |view| {
            view.system.ios.trusted.contains("A1") && view.system.ios.working.is_none()
        });
        let said = std::fs::read_to_string(&asked).unwrap();
        assert!(
            said.ends_with(&format!(
                "simctl keychain A1 add-root-cert {}\n",
                dir.join("ca.pem").display()
            )),
            "{said}"
        );
        scroll(&mut cx, 500.);
        shot(&mut cx, window.into(), "106-setup-simulators");
        scroll(&mut cx, -500.);
    }
    scroll(&mut cx, 1400.);
    shot(&mut cx, window.into(), "96-setup-when-done");
    window
        .update(&mut cx, |view, _, cx| view.set_guide(Guide::Android, cx))
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "62-setup-android");

    std::fs::remove_dir_all(&dir).ok();
}
