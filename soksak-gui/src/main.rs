use gpui::*;
use rust_i18n::t;

actions!(soksak_gui, [Quit]);

struct Workspace {
    // We can add state here later
}

// ... existing code ...

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .bg(rgb(0x1e1e1e)) // Dark background
            .text_color(rgb(0xffffff))
            .key_context("Workspace")
            .child(
                // Left Panel
                div()
                    .w_64()
                    .h_full()
                    .flex()
                    .flex_col()
                    .bg(rgb(0x252526)) // Slightly lighter sidebar
                    .border_r_1()
                    .border_color(rgb(0x333333))
                    .child(
                        // Spacer for native traffic lights
                        div().h_10(),
                    )
                    .child(
                        // "All" Item (Active)
                        div()
                            .mx_2()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x37373d)) // Active background
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        // Icon placeholder
                                        div()
                                            .w_4()
                                            .h_4()
                                            .border_1()
                                            .rounded_full()
                                            .border_color(rgb(0xaaaaaa)),
                                    )
                                    .child(SharedString::from(t!("list.all"))),
                            )
                            .child(
                                div()
                                    .px_1()
                                    .rounded_sm()
                                    .bg(rgb(0x555555))
                                    .text_xs()
                                    .child("3"),
                            ),
                    )
                    .child(
                        // "Processing" Item
                        div()
                            .mx_2()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .flex()
                            .gap_2()
                            .items_center()
                            .text_color(rgb(0xaaaaaa))
                            .child(
                                // Icon placeholder
                                div()
                                    .w_4()
                                    .h_4()
                                    .border_1()
                                    .rounded_full()
                                    .border_color(rgb(0x666666)),
                            )
                            .child(SharedString::from(t!("list.inprogress"))),
                    )
                    .child(
                        // "Queued" Item
                        div()
                            .mx_2()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .flex()
                            .gap_2()
                            .items_center()
                            .text_color(rgb(0xaaaaaa))
                            .child(
                                // Icon placeholder
                                div()
                                    .w_4()
                                    .h_4()
                                    .border_1()
                                    .rounded_full()
                                    .border_color(rgb(0x666666)),
                            )
                            .child(SharedString::from(t!("list.queued"))),
                    )
                    .child(
                        // "Done" Item
                        div()
                            .mx_2()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .flex()
                            .gap_2()
                            .items_center()
                            .text_color(rgb(0xaaaaaa))
                            .child(
                                // Icon placeholder
                                div()
                                    .w_4()
                                    .h_4()
                                    .border_1()
                                    .rounded_full()
                                    .border_color(rgb(0x666666)),
                            )
                            .child(SharedString::from(t!("list.completed"))),
                    ),
            )
            .child(
                // Main Content Area
                div()
                    .flex_1()
                    .h_full()
                    .bg(rgb(0x1e1e1e))
                    .flex()
                    .flex_col()
                    .child(
                        // Header
                        div()
                            .h(px(56.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_6()
                            .border_b_1()
                            .border_color(rgb(0x333333))
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::BOLD)
                                    .child(SharedString::from(t!("list.all"))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(
                                        // Start Button
                                        div()
                                            .px_3()
                                            .py_1()
                                            .rounded_md()
                                            .bg(rgb(0x333333))
                                            .text_sm()
                                            .child(SharedString::from(t!("btn.start"))),
                                    )
                                    .child(
                                        // Profile Dropdown
                                        div()
                                            .px_3()
                                            .py_1()
                                            .rounded_md()
                                            .bg(rgb(0x333333))
                                            .text_sm()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(div().text_color(rgb(0xaaaaaa)).child("+"))
                                            .child("Profile: Interview"),
                                    )
                                    .child(
                                        // Settings Button
                                        div()
                                            .w_8()
                                            .h_8()
                                            .rounded_md()
                                            .bg(rgb(0x333333))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                // Gear Icon placeholder
                                                div()
                                                    .w_4()
                                                    .h_4()
                                                    .border_1()
                                                    .rounded_full()
                                                    .border_color(rgb(0xaaaaaa)),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        // List View
                        div()
                            .flex_1()
                            .p_6()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                // Item 1
                                div()
                                    .p_4()
                                    .rounded_lg()
                                    .bg(rgb(0x252526))
                                    .border_1()
                                    .border_color(rgb(0x333333))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .gap_4()
                                            .items_center()
                                            .child(
                                                // Icon
                                                div()
                                                    .w_10()
                                                    .h_10()
                                                    .rounded_md()
                                                    .bg(rgb(0x333333))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(
                                                        div()
                                                            .w_6()
                                                            .h_6()
                                                            .border_1()
                                                            .border_color(rgb(0xaaaaaa)),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .child(
                                                        div()
                                                            .font_weight(FontWeight::BOLD)
                                                            .child("Soksak App Demo.mp4"),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(rgb(0x888888))
                                                            .child("Profile: General"),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap_4()
                                            .items_center()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(0xaaaaaa))
                                                    .child(SharedString::from(t!("list.queued"))),
                                            )
                                            .child(
                                                div().w_1().h_4().bg(rgb(0xaaaaaa)), // Kebab menu placeholder
                                            ),
                                    ),
                            )
                            .child(
                                // Item 2
                                div()
                                    .p_4()
                                    .rounded_lg()
                                    .bg(rgb(0x1e1e1e)) // Transparent/Darker
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .gap_4()
                                            .items_center()
                                            .child(
                                                // Icon
                                                div()
                                                    .w_10()
                                                    .h_10()
                                                    .rounded_md()
                                                    .bg(rgb(0x333333))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(
                                                        div()
                                                            .w_6()
                                                            .h_6()
                                                            .border_1()
                                                            .border_color(rgb(0xaaaaaa)),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .child(
                                                        div()
                                                            .font_weight(FontWeight::BOLD)
                                                            .child("Product Interview.mov"),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(rgb(0x888888))
                                                            .child("Profile: Interview"),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap_4()
                                            .items_center()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(0xaaaaaa))
                                                    .child(SharedString::from(t!("list.queued"))),
                                            )
                                            .child(div().w_1().h_4().bg(rgb(0xaaaaaa))),
                                    ),
                            )
                            .child(
                                // Item 3
                                div()
                                    .p_4()
                                    .rounded_lg()
                                    .bg(rgb(0x1e1e1e))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .gap_4()
                                            .items_center()
                                            .child(
                                                // Icon
                                                div()
                                                    .w_10()
                                                    .h_10()
                                                    .rounded_md()
                                                    .bg(rgb(0x333333))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .child(
                                                        div()
                                                            .w_6()
                                                            .h_6()
                                                            .border_1()
                                                            .border_color(rgb(0xaaaaaa)),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .child(
                                                        div()
                                                            .font_weight(FontWeight::BOLD)
                                                            .child("Weekly Standup.mp4"),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(rgb(0x888888))
                                                            .child("Profile: General"),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap_4()
                                            .items_center()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(rgb(0xaaaaaa))
                                                    .child(SharedString::from(t!("list.queued"))),
                                            )
                                            .child(div().w_1().h_4().bg(rgb(0xaaaaaa))),
                                    ),
                            ),
                    )
                    .child(
                        // Bottom Progress Bar
                        div()
                            .h_20()
                            .bg(rgb(0x252526))
                            .border_t_1()
                            .border_color(rgb(0x333333))
                            .px_6()
                            .flex()
                            .flex_col()
                            .justify_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .text_sm()
                                    .text_color(rgb(0xcccccc))
                                    .child("Processing: Onboarding_flow_v3.mp4")
                                    .child("42%"),
                            )
                            .child(
                                div().h_2().w_full().rounded_full().bg(rgb(0x333333)).child(
                                    div()
                                        .h_full()
                                        .w_2_5() // 40% roughly
                                        .rounded_full()
                                        .bg(rgb(0x0066ff)), // Blue
                                ),
                            ),
                    ),
            )
    }
}

rust_i18n::i18n!("assets");

fn main() {
    // Get system locale and set it as application locale
    let locale = sys_locale::get_locale().unwrap_or("en_US".to_string());
    rust_i18n::set_locale(&locale);

    // Start the application
    Application::new().run(|cx: &mut App| {
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
            |_, cx| cx.new(|_| Workspace {}),
        )
        .expect("failed to open window");

        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

        cx.activate(true);
    });
}
