//! `tracewing`: the window. The proxy starts with it and stops with it.

use anyhow::Result;
use gpui::{AppContext as _, TitlebarOptions, WindowDecorations, WindowOptions, px, size};
use std::path::PathBuf;
use tracewing::{
    Assets, chrome,
    engine::{Engine, EngineOptions, PORTS},
    layout::Layout,
    placement,
    traffic::Traffic,
};
use tracewing_proxy::{
    crypto_provider, default_data_dir, rules::HostRules, traffic_rules::TrafficRules,
};
use tracewing_store::settings::StorageSettings;
use tracewing_system as system;

/// What the system crate runs this program for: `watchdog --data-dir D` when the
/// app has gone, and `sysproxy recover --data-dir D` at the next login.
fn helper_command(args: &[String]) -> Option<Result<()>> {
    let data_dir =
        |flag: &String, value: &String| (flag == "--data-dir").then(|| PathBuf::from(value));
    match args {
        [_, watchdog, flag, value] if watchdog == "watchdog" => {
            Some(system::watchdog(&data_dir(flag, value)?))
        }
        [_, sysproxy, recover, flag, value] if sysproxy == "sysproxy" && recover == "recover" => {
            Some(system::recover(&data_dir(flag, value)?).map(|_| ()))
        }
        _ => None,
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(result) = helper_command(&args) {
        return result;
    }
    let _ = crypto_provider().install_default();
    let data_dir = default_data_dir();
    std::fs::create_dir_all(&data_dir)?;
    // Left on by a crash or a power cut, with the login recovery not yet run.
    if let Err(e) = system::recover(&data_dir) {
        eprintln!("tracewing: cannot put the system proxy back: {e:#}");
    }
    let path = data_dir.join("settings.toml");
    let (settings, settings_problem) = if path.exists() {
        match StorageSettings::load(&path) {
            Ok(settings) => (settings, None),
            Err(e) => (StorageSettings::default(), Some(format!("{e:#}"))),
        }
    } else {
        (StorageSettings::default(), None)
    };
    let rules_path = data_dir.join("decrypt.toml");
    let (rules, rules_problem) = match HostRules::load(&rules_path) {
        Ok(rules) => (rules, None),
        Err(e) => (HostRules::default(), Some(format!("{e:#}"))),
    };
    let (traffic_rules, traffic_problem) =
        match TrafficRules::load(&data_dir.join("traffic-rules.toml")) {
            Ok(rules) => (rules, None),
            Err(e) => (TrafficRules::default(), Some(format!("{e:#}"))),
        };
    let settings_problem = settings_problem.or(rules_problem).or(traffic_problem);
    let engine = Engine::start(EngineOptions {
        data_dir: data_dir.clone(),
        settings,
        rules,
        traffic_rules,
        ports: PORTS,
        extra_roots: Vec::new(),
    })?;

    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            tracewing::load_fonts(cx);
            let saved = Layout::load(&data_dir.join("layout.toml")).window;
            let bounds = placement::initial(saved, cx);
            // GNOME's Wayland compositor draws no window frame, so Linux gets
            // client-side decorations. macOS and Windows keep the system's frame
            // but hide its title bar, and the app draws its own.
            let client_side = cfg!(any(target_os = "linux", target_os = "freebsd"));
            let opened = cx.open_window(
                WindowOptions {
                    window_bounds: Some(bounds),
                    window_min_size: Some(size(
                        px(placement::MIN_SIZE.0),
                        px(placement::MIN_SIZE.1),
                    )),
                    window_decorations: client_side.then_some(WindowDecorations::Client),
                    titlebar: (!client_side).then(|| TitlebarOptions {
                        title: Some("tracewing".into()),
                        appears_transparent: true,
                        traffic_light_position: Some(chrome::TRAFFIC_LIGHTS),
                    }),
                    app_id: Some("tracewing".into()),
                    focus: true,
                    ..Default::default()
                },
                move |window, cx| {
                    cx.new(|cx| {
                        let mut view = Traffic::new(engine, &data_dir, window, cx);
                        view.settings_problem = settings_problem;
                        view.open_first_screen();
                        view
                    })
                },
            );
            if let Err(e) = opened {
                eprintln!("tracewing: cannot open a window: {e:#}");
                cx.quit();
                return;
            }
            cx.on_window_closed(|cx, _| cx.quit()).detach();
            cx.activate(true);
        });
    Ok(())
}
