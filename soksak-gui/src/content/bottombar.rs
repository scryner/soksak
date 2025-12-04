use gpui::*;

pub struct BottomBar;

impl BottomBar {
    pub fn new(app: &mut App) -> Entity<Self> {
        app.new(|_| Self)
    }
}

impl Render for BottomBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
            )
    }
}
