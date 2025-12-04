use gpui::*;
use rust_i18n::t;

pub struct Header;

impl Header {
    pub fn new(app: &mut App) -> Entity<Self> {
        app.new(|_| Self)
    }
}

impl Render for Header {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
            )
    }
}
