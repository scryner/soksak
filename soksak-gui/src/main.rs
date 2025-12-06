use gpui::*;

use crate::socsak_app::SoksakApp;

pub mod assets;
mod content;
pub mod icon;
mod sidebar;
mod socsak_app;

rust_i18n::i18n!("assets");
actions!(soksak_gui, [Quit]);

fn main() {
    // Get system locale and set it as application locale
    let locale = sys_locale::get_locale().unwrap_or("en_US".to_string());
    rust_i18n::set_locale(&locale);

    // Start the application
    Application::new()
        .with_assets(crate::assets::Assets::new(std::path::PathBuf::from(
            "soksak-gui/assets",
        )))
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
                |_, app| app.new(|inner| SoksakApp::new(inner)),
            )
            .expect("failed to open window");

            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

            cx.activate(true);
        });
}
