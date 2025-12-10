use gpui::*;

use crate::socsak_app::SoksakApp;

pub mod assets;
mod content;
pub mod icon;
mod profile;
mod sidebar;
mod socsak_app;

rust_i18n::i18n!("assets");
actions!(soksak_gui, [Quit, About]);

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
            cx.set_menus(vec![Menu {
                name: "Soksak".into(),
                items: vec![
                    MenuItem::action("About Soksak", About),
                    MenuItem::separator(),
                    MenuItem::action("Quit Soksak", Quit),
                ],
            }]);
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
                    app.new(|inner| {
                        inner.on_release(|_, cx| cx.quit()).detach();
                        SoksakApp::new(inner, profile_manager)
                    })
                },
            )
            .expect("failed to open window");

            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.on_action(|_: &About, cx| {
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                            None,
                            size(px(300.), px(200.)),
                            cx,
                        ))),
                        titlebar: Some(TitlebarOptions {
                            title: Some("About Soksak".into()),
                            appears_transparent: true,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    |_, cx| cx.new(|_| AboutView),
                )
                .ok();
            });
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

            cx.activate(true);
        });
}

struct AboutView;

impl Render for AboutView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .bg(rgb(0xffffff))
            .text_color(rgb(0x000000))
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .mb_2()
                    .child("Soksak"),
            )
            .child(div().text_sm().child("AI powered audio tool"))
            .child(
                div()
                    .text_xs()
                    .mt_4()
                    .text_color(rgb(0x888888))
                    .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
            )
    }
}
