use gpui::*;
use rust_i18n::t;

pub struct Sidebar;

impl Sidebar {
    pub fn new(app: &mut App) -> Entity<Self> {
        app.new(|_| Self)
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
                                crate::icon::Icon::List
                                    .render()
                                    .size_5()
                                    .text_color(rgb(0xffffff)),
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
                        crate::icon::Icon::ProgressActivity
                            .render()
                            .size_5()
                            .text_color(rgb(0xaaaaaa)),
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
                        crate::icon::Icon::PauseCircle
                            .render()
                            .size_5()
                            .text_color(rgb(0xaaaaaa)),
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
                        crate::icon::Icon::CheckCircle
                            .render()
                            .size_5()
                            .text_color(rgb(0xaaaaaa)),
                    )
                    .child(SharedString::from(t!("list.completed"))),
            )
    }
}
