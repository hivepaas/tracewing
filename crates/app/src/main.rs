//! `tracewing`: the window. The proxy starts with it and stops with it.

use anyhow::Result;
use gpui::{AppContext as _, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};
use tracewing::{
    Assets,
    engine::{Engine, EngineOptions, PORTS},
    traffic::Traffic,
};
use tracewing_proxy::{crypto_provider, default_data_dir};
use tracewing_store::settings::StorageSettings;

fn main() -> Result<()> {
    let _ = crypto_provider().install_default();
    let data_dir = default_data_dir();
    std::fs::create_dir_all(&data_dir)?;
    let engine = Engine::start(EngineOptions {
        data_dir: data_dir.clone(),
        settings: StorageSettings::default(),
        ports: PORTS,
        extra_roots: Vec::new(),
    })?;

    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx| {
            tracewing::load_fonts(cx);
            let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
            let opened = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("tracewing".into()),
                        ..Default::default()
                    }),
                    focus: true,
                    ..Default::default()
                },
                move |window, cx| cx.new(|cx| Traffic::new(engine, &data_dir, window, cx)),
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
