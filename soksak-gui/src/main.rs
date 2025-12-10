use gpui::*;

use crate::socsak_app::SoksakApp;

pub mod assets;
mod content;
pub mod icon;
mod profile;
mod sidebar;
mod socsak_app;

rust_i18n::i18n!("assets");
actions!(soksak_gui, [Quit]);

pub mod progress;

fn main() {
    // Get system locale and set it as application locale
    let locale = sys_locale::get_locale().unwrap_or("en_US".to_string());
    rust_i18n::set_locale(&locale);

    // Start the application
    Application::new()
        .with_assets(crate::assets::Assets::new(
            if let Ok(exe_path) = std::env::current_exe() {
                let bundle_assets = exe_path.parent().unwrap().join("../Resources/assets");
                if bundle_assets.exists() {
                    bundle_assets
                } else {
                    std::path::PathBuf::from("soksak-gui/assets")
                }
            } else {
                std::path::PathBuf::from("soksak-gui/assets")
            },
        ))
        .run(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(1080.), px(720.)), cx);

            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, app| {
                    let profile_manager = crate::profile::ProfileManager::new(app);
                    app.new(|inner| SoksakApp::new(inner, profile_manager))
                },
            )
            .expect("failed to open window");

            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

            cx.activate(true);

            cx.on_window_closed(|cx| cx.quit()).detach();
        });
}
