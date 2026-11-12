//! Authorities put in the data directory's `trusted-servers` folder: the
//! proxy trusts their servers, and Settings says what the folder holds. And
//! the TLS secrets of those connections, written for Wireshark from Settings.
//!
//! `cargo test --release -p tracewing --features headless --test trusted`

#![cfg(all(feature = "headless", target_os = "linux"))]

use gpui::{AppContext as _, Entity, HeadlessAppContext, WindowHandle, px, size};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tracewing::{
    engine::{Engine, EngineOptions},
    input::TextInput,
    traffic::{Screen, Traffic},
    trusted,
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

#[test]
fn servers_of_a_trusted_authority_are_trusted() {
    let _ = tracewing_proxy::crypto_provider().install_default();
    let dir = std::env::temp_dir().join(format!("tracewing-trusted-ui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // The playground's authority in the folder, as `--trust` puts it, and a
    // file that is no certificate.
    let state = dir.join("playground");
    let ca = tracewing_dev::playground::authority(&state).unwrap();
    let folder = trusted::folder(&dir);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::copy(&ca, folder.join("tracewing-playground.pem")).unwrap();
    std::fs::write(folder.join("broken.pem"), "not a certificate").unwrap();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let serving = state.clone();
    runtime.spawn(async move { tracewing_dev::playground::serve_tls(listener, &serving).await });
    // And with mutual TLS, for a client certificate given in Settings.
    let mtls = runtime
        .block_on(tokio::net::TcpListener::bind("127.0.0.1:0"))
        .unwrap();
    let mtls_port = mtls.local_addr().unwrap().port();
    let serving = state.clone();
    runtime.spawn(async move { tracewing_dev::playground::serve_mtls(mtls, &serving).await });

    let found = trusted::read(&dir);
    let engine = Engine::start(EngineOptions {
        data_dir: dir.clone(),
        settings: StorageSettings {
            disk_reserve_mib: 1,
            ..StorageSettings::default()
        },
        rules: Default::default(),
        traffic_rules: Default::default(),
        ports: 20_280..20_290,
        extra_roots: found.paths(),
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

    // Through the proxy, which trusts the playground's server.
    window
        .update(&mut cx, |view, _, cx| {
            view.new_compose(cx);
            let page = view.compose_page.as_ref().unwrap();
            type_in(
                &page.url.clone(),
                &format!("https://localhost:{port}/json"),
                cx,
            );
            view.send_composed(cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the answer", |view| {
        view.compose_page
            .as_ref()
            .is_some_and(|page| page.answer.is_some() || page.problem.is_some())
    });
    window
        .update(&mut cx, |view, _, _| {
            let page = view.compose_page.as_ref().unwrap();
            assert_eq!(page.problem, None);
            assert_eq!(page.answer.as_ref().unwrap().status, 200);
        })
        .unwrap();

    // Settings says what the folder holds, and what was left out.
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Settings, cx);
            let names: Vec<&str> = view.trusted.good.iter().map(|a| a.name.as_str()).collect();
            assert_eq!(names, ["tracewing playground CA"]);
            let bad: Vec<&str> = view.trusted.bad.iter().map(|(f, _)| f.as_str()).collect();
            assert_eq!(bad, ["broken.pem"]);
            assert_eq!(view.trusted.paths(), view.engine.extra_roots, "as started");
        })
        .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "82-trusted-servers");

    // Another put in: a restart is needed, and Settings says so.
    std::fs::copy(&ca, folder.join("again.pem")).unwrap();
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Traffic, cx);
            view.set_screen(Screen::Settings, cx);
            assert_eq!(view.trusted.good.len(), 2);
            assert_ne!(view.trusted.paths(), view.engine.extra_roots);
        })
        .unwrap();

    // A client certificate, given in Settings for localhost: the server that
    // asks for one has it from then on.
    let (client_cert, client_key) = tracewing_dev::playground::client_certificate(&state).unwrap();
    let ask = |cx: &mut HeadlessAppContext| {
        window
            .update(cx, |view, _, cx| {
                view.new_compose(cx);
                let page = view.compose_page.as_ref().unwrap();
                type_in(
                    &page.url.clone(),
                    &format!("https://localhost:{mtls_port}/text"),
                    cx,
                );
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
                (
                    page.answer.as_ref().map(|answer| answer.status),
                    page.problem.clone(),
                )
            })
            .unwrap()
    };
    let (status, _) = ask(&mut cx);
    assert_ne!(status, Some(200), "none given yet");
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Settings, cx);
            // Nothing chosen: it says what is missing.
            view.add_client_cert(cx);
            let form = view.client_cert_form.as_ref().unwrap();
            assert!(form.problem.is_some());
            let host = form.host.clone();
            type_in(&host, "localhost", cx);
            // A key that is not this certificate's is refused.
            view.set_client_cert_file(false, client_cert.clone(), cx);
            view.set_client_cert_file(true, dir.join("ca.pem"), cx);
            view.add_client_cert(cx);
            assert!(view.client_cert_settings().certs.is_empty(), "not added");
            view.set_client_cert_file(true, client_key.clone(), cx);
            view.add_client_cert(cx);
            assert_eq!(view.client_cert_settings().certs.len(), 1);
            let form = view.client_cert_form.as_ref().unwrap();
            assert_eq!(form.problem, None);
            assert!(form.cert.is_none(), "the form is emptied");
        })
        .unwrap();
    cx.run_until_parked();
    let (status, problem) = ask(&mut cx);
    assert_eq!(status, Some(200), "{problem:?}");
    window
        .update(&mut cx, |view, _, cx| view.set_screen(Screen::Settings, cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position: gpui::point(px(900.), px(500.)),
                delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-900.))),
                ..Default::default()
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "103-client-certificates");
    window
        .update(&mut cx, |view, _, cx| {
            view.remove_client_cert(0, cx);
            assert!(view.client_cert_settings().certs.is_empty());
        })
        .unwrap();

    // A request for the company to sign, and what it sends back taken in: the
    // CA is the company's from the next start, and back to its own after.
    let csr = dir.join("tracewing-ca.csr");
    let signed = dir.join("signed.pem");
    window
        .update(&mut cx, |view, _, cx| {
            view.save_company_request(&csr, cx);
            assert!(!view.company_ca_note.as_ref().unwrap().1);
        })
        .unwrap();
    {
        use hudsucker::rcgen::{
            BasicConstraints, CertificateParams, CertificateSigningRequestParams, DnType, IsCa,
            Issuer, KeyPair, KeyUsagePurpose,
        };
        let key = KeyPair::generate().unwrap();
        let mut root = CertificateParams::default();
        root.distinguished_name
            .push(DnType::CommonName, "Example Corp Root");
        root.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        root.key_usages = vec![KeyUsagePurpose::KeyCertSign];
        let issuer = Issuer::new(root, key);
        let asked =
            CertificateSigningRequestParams::from_pem(&std::fs::read_to_string(&csr).unwrap())
                .unwrap();
        std::fs::write(&signed, asked.signed_by(&issuer).unwrap().pem()).unwrap();
    }
    window
        .update(&mut cx, |view, _, cx| {
            view.take_company_cert(&signed, cx);
            let (note, failed) = view.company_ca_note.clone().unwrap();
            assert!(!failed, "{note}");
            assert!(note.contains("Example Corp Root"), "{note}");
            // What cannot be used is refused, and says why.
            view.take_company_cert(&csr, cx);
            assert!(view.company_ca_note.as_ref().unwrap().1);
            view.take_company_cert(&signed, cx);
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position: gpui::point(px(900.), px(500.)),
                delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-150.))),
                ..Default::default()
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "105-company-ca");
    window
        .update(&mut cx, |view, _, cx| {
            view.back_to_own_ca(cx);
            let (note, failed) = view.company_ca_note.clone().unwrap();
            assert!(!failed, "{note}");
        })
        .unwrap();
    assert!(
        !tracewing_proxy::company_ca::info(&dir).unwrap().company,
        "its own again"
    );

    // TLS keys for Wireshark, switched on in Settings: the next request's
    // secrets are written to the data folder.
    let keys = dir.join(tracewing::settings_page::TLS_KEYS_FILE);
    window
        .update(&mut cx, |view, _, cx| {
            assert!(view.engine.tls_keys.path().is_none(), "off at start");
            view.switch_tls_keys(cx);
            assert_eq!(view.engine.tls_keys.path(), Some(keys.clone()));
            view.new_compose(cx);
            let page = view.compose_page.as_ref().unwrap();
            type_in(
                &page.url.clone(),
                &format!("https://localhost:{port}/xml"),
                cx,
            );
            view.send_composed(cx);
        })
        .unwrap();
    wait_for(&mut cx, window, "the answer", |view| {
        view.compose_page
            .as_ref()
            .is_some_and(|page| page.answer.is_some() || page.problem.is_some())
    });
    let written = std::fs::read_to_string(&keys).unwrap();
    assert!(written.contains("CLIENT_TRAFFIC_SECRET_0 "), "{written}");
    window
        .update(&mut cx, |view, _, cx| {
            view.set_screen(Screen::Settings, cx);
            view.switch_tls_keys(cx);
            assert!(view.engine.tls_keys.path().is_none());
            view.switch_tls_keys(cx);
        })
        .unwrap();
    cx.run_until_parked();
    // Down to it.
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position: gpui::point(px(900.), px(500.)),
                delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-900.))),
                ..Default::default()
            }),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    shot(&mut cx, window.into(), "100-tls-keys");

    drop(cx);
    let _ = std::fs::remove_dir_all(&dir);
}
