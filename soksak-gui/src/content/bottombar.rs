use gpui::{prelude::*, *};
use rust_i18n::t;

pub enum BottomBarEvent {
    Start,
    Cancel,
}

pub struct BottomBar {
    is_running: bool,
    progress: f32,
    message: SharedString,
    progress_label: Option<String>,
}

impl EventEmitter<BottomBarEvent> for BottomBar {}

impl BottomBar {
    pub fn new(app: &mut App) -> Entity<Self> {
        app.new(|_| Self {
            is_running: false,
            progress: 0.0,
            message: SharedString::from(t!("progress.ready")),
            progress_label: None,
        })
    }

    fn toggle_running(&mut self, cx: &mut Context<Self>) {
        if self.is_running {
            cx.emit(BottomBarEvent::Cancel);
        } else {
            cx.emit(BottomBarEvent::Start);
        }
    }

    pub fn set_running(&mut self, running: bool, cx: &mut Context<Self>) {
        self.is_running = running;
        cx.notify();
    }

    pub fn set_progress(&mut self, progress: f32, cx: &mut Context<Self>) {
        self.progress = progress;
        self.progress_label = None;
        cx.notify();
    }

    pub fn set_progress_detailed(&mut self, current: u64, total: u64, cx: &mut Context<Self>) {
        self.progress = current as f32 / total as f32;
        self.progress_label = Some(format!(
            "{}% ({}/{})",
            (self.progress * 100.0) as u32,
            current,
            total
        ));
        cx.notify();
    }

    pub fn set_message(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.message = message.into();
        cx.notify();
    }
}

impl Render for BottomBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let button_text = if self.is_running {
            t!("btn.cancel")
        } else {
            t!("btn.start")
        };

        let progress_text = if let Some(label) = &self.progress_label {
            label.clone()
        } else {
            format!("{}%", (self.progress * 100.0) as u32)
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
                    .text_color(rgb(0xcccccc))
                    .child(self.message.clone())
                    .child(progress_text),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .items_center()
                    .child(
                        div().flex_1().h_2().rounded_full().bg(rgb(0x333333)).child(
                            div()
                                .h_full()
                                .w(DefiniteLength::Fraction(self.progress))
                                .rounded_full()
                                .bg(rgb(0x0066ff)), // Blue
                        ),
                    )
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
