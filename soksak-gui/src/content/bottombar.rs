use gpui::{prelude::*, *};
use rust_i18n::t;

pub struct BottomBar {
    is_running: bool,
}

impl BottomBar {
    pub fn new(app: &mut App) -> Entity<Self> {
        app.new(|_| Self { is_running: false })
    }

    fn toggle_running(&mut self, _cx: &mut Context<Self>) {
        self.is_running = !self.is_running;
    }
}

impl Render for BottomBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let button_text = if self.is_running {
            t!("btn.cancel")
        } else {
            t!("btn.start")
        };

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
                    .child(SharedString::from(t!("progress.ready")))
                    .child("0%"),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .items_center()
                    .child(div().flex_1().h_2().rounded_full().bg(rgb(0x333333)).child(
                        div().h_full().w(px(0.0)).rounded_full().bg(rgb(0x0066ff)), // Blue
                    ))
                    .child(
                        // Start/Cancel Button
                        div()
                            .id("start_btn")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x333333))
                            .text_sm()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_running(cx)))
                            .child(SharedString::from(button_text)),
                    ),
            )
    }
}
