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
use tracewing_proxy::{crypto_provider, default_data_dir};
use tracewing_system as system;

/// `tracewing mcp`, which an AI tool starts to read the recorded traffic. And
/// what the system crate runs this program for: `watchdog --data-dir D` when the
/// app has gone, `sysproxy recover --data-dir D` at the next login, and on
/// Windows `block-quic --parent P --name N --data-dir D`, as administrator.
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
        [_, command, ..] if tracewing::cli::is_command(command) => Some(tracewing::cli::run(args)),
        [_, mcp] if mcp == "mcp" => Some(tracewing::mcp::serve(&default_data_dir())),
        [_, mcp, flag, value] if mcp == "mcp" => {
            Some(tracewing::mcp::serve(&data_dir(flag, value)?))
        }
        [_, block, parent_flag, parent, name_flag, name, flag, value]
            if block == "block-quic" && parent_flag == "--parent" && name_flag == "--name" =>
        {
            let parent = parent.parse().ok()?;
            Some(system::quic_helper(parent, name, &data_dir(flag, value)?))
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
    let (options, settings_problem) = EngineOptions::from_data_dir(&data_dir, PORTS);
    let engine = Engine::start(options)?;

    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            tracewing::load_fonts(cx);
            let layout = Layout::load(&data_dir.join("layout.toml"));
            // Before anything is said. Tests start no `main`, and speak English.
            tracewing_i18n::set_language(layout.language.as_deref());
            let saved = layout.window;
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
            let main = match opened {
                Ok(main) => main,
                Err(e) => {
                    eprintln!("tracewing: cannot open a window: {e:#}");
                    cx.quit();
                    return;
                }
            };
            // The inspector's window, if it was left open, comes back too.
            let _ = main.update(cx, |view, _, cx| view.reopen_inspector_window(cx));
            // Only the main window's closing ends the app.
            let main_id = main.window_id();
            cx.on_window_closed(move |cx, closed| {
                if closed == main_id {
                    cx.quit();
                }
            })
            .detach();
            cx.activate(true);
        });
    Ok(())
}
