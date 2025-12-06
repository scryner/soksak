use gpui::InteractiveElement;
use gpui::prelude::*;
use gpui::*;
use rust_i18n::t;

pub enum JobEvent {
    MenuOpen(Point<Pixels>),
}

pub struct Job {
    pub(crate) name: SharedString,
    pub(crate) profile: SharedString,
    pub(crate) status: Status,
    pub(crate) selected: bool,
}

impl EventEmitter<JobEvent> for Job {}

impl Job {
    pub fn new(app: &mut App, name: &str, profile: &str, status: Status) -> Entity<Self> {
        let name: SharedString = name.to_string().into();
        let profile: SharedString = profile.to_string().into();

        app.new(|_| Self {
            name,
            profile,
            status,
            selected: false,
        })
    }

    pub fn set_selected(&mut self, selected: bool, cx: &mut Context<Self>) {
        self.selected = selected;
        cx.notify();
    }

    pub fn toggle_menu(&mut self, event: &ClickEvent, cx: &mut Context<Self>) {
        cx.emit(JobEvent::MenuOpen(event.position()));
    }
}

impl Render for Job {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg_color = if self.selected {
            rgb(0x28324a)
        } else {
            rgb(0x252526)
        };

        div()
            .p_4()
            .rounded_lg()
            .bg(bg_color)
            .when(!self.selected, |this| {
                this.hover(|style| style.bg(rgb(0x37373d)))
            })
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
                            .flex_shrink_0()
                            .rounded_md()
                            .bg(rgb(0x6b7280)) // Gray 500
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                crate::icon::Icon::FileVideo
                                    .render()
                                    .text_color(rgb(0xffffff)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(div().font_weight(FontWeight::BOLD).child(self.name.clone()))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x888888))
                                    .child(self.profile.clone()),
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
                            .child(SharedString::from(self.status)),
                    )
                    .child(
                        div().child(
                            div()
                                .id("menu-button")
                                .cursor_pointer()
                                .child(
                                    crate::icon::Icon::EllipsisVertical
                                        .render()
                                        .text_color(rgb(0xaaaaaa)),
                                )
                                .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                                    this.toggle_menu(event, cx)
                                })),
                        ),
                    ),
            )
    }
}

#[derive(Copy, Clone)]
pub enum Status {
    Queued,
    Processing,
    Completed,
    Failed,
}

impl From<Status> for SharedString {
    fn from(value: Status) -> Self {
        match value {
            Status::Queued => SharedString::from(t!("list.queued")),
            Status::Processing => SharedString::from(t!("list.processing")),
            Status::Completed => SharedString::from(t!("list.completed")),
            Status::Failed => SharedString::from(t!("list.failed")),
        }
    }
}
