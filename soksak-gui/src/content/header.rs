use gpui::InteractiveElement;
use gpui::prelude::*;
use gpui::*;
use rust_i18n::t;

use crate::content::job_list::JobList;
use crate::icon::Icon;
use crate::profile::{ProfileEvent, ProfileManager};

pub enum HeaderEvent {
    ToggleProfileMenu(Point<Pixels>),
}

pub struct Header {
    job_list: Entity<JobList>,
    profile_manager: Entity<ProfileManager>,
}

impl EventEmitter<HeaderEvent> for Header {}

impl Header {
    pub fn new(
        app: &mut App,
        job_list: Entity<JobList>,
        profile_manager: Entity<ProfileManager>,
    ) -> Entity<Self> {
        app.new(|cx| {
            cx.subscribe(&profile_manager, Self::on_profile_event)
                .detach();
            Self {
                job_list,
                profile_manager,
            }
        })
    }

    fn on_profile_event(
        &mut self,
        _manager: Entity<ProfileManager>,
        _event: &ProfileEvent,
        cx: &mut Context<Self>,
    ) {
        cx.notify();
    }

    fn toggle_profile_menu(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        cx.emit(HeaderEvent::ToggleProfileMenu(position));
    }
}

impl Render for Header {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let job_list = self.job_list.read(cx);
        let title = match job_list.filter {
            crate::content::job_list::Filter::All => t!("list.all"),
            crate::content::job_list::Filter::Processing => t!("list.inprogress"),
            crate::content::job_list::Filter::Queued => t!("list.queued"),
            crate::content::job_list::Filter::Completed => t!("list.completed"),
        };

        let profile_manager = self.profile_manager.read(cx);
        let current_profile = profile_manager
            .get_current_profile()
            .cloned()
            .unwrap_or_else(|| "Default".to_string());
        // We don't need `profiles` here anymore, parent needs it.

        // Header
        div()
            .h(px(56.0))
            .flex()
            .items_center()
            .justify_between()
            .px_6()
            .border_b_1()
            .border_color(rgb(0x333333))
            // .z_index(100) eliminated
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .child(SharedString::from(title)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        // Profile Dropdown Trigger
                        div().relative().child(
                            div()
                                .px_3()
                                .py_1()
                                .rounded_md()
                                .bg(rgb(0x333333))
                                .text_sm()
                                .flex()
                                .items_center()
                                .gap_1()
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                        this.toggle_profile_menu(event.position, cx);
                                    }),
                                )
                                .child(current_profile)
                                .child(Icon::ChevronDown.render().text_color(rgb(0xaaaaaa))),
                        ),
                        // Removed Rendered Menu
                    )
                    .child(
                        // Add File Button
                        div()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x333333))
                            .text_sm()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            // Hover effect could be added here later
                            .id("add_file_btn")
                            .on_click(cx.listener(|this, _event, _window, cx| {
                                let job_list = this.job_list.clone();
                                let profile_manager = this.profile_manager.read(cx);
                                let current_profile = profile_manager
                                    .get_current_profile()
                                    .cloned()
                                    .unwrap_or_else(|| "Default".to_string());

                                cx.spawn(|_, cx: &mut AsyncApp| {
                                    let mut cx: AsyncApp = cx.clone();
                                    async move {
                                        let file = rfd::AsyncFileDialog::new()
                                            .add_filter(
                                                "Video",
                                                &crate::content::Content::SUPPORTED_VIDEO_EXTENSIONS,
                                            )
                                            .pick_file()
                                            .await;

                                        if let Some(file) = file {
                                            let path = file.path().to_path_buf();
                                            // cx is owned AsyncApp here?
                                            job_list
                                                .update(&mut cx, move |job_list, cx| {
                                                    job_list.add_job(cx, path, current_profile);
                                                })
                                                .ok();
                                        }
                                    }
                                })
                                .detach();
                            }))
                            .child(
                                svg()
                                    .path("icons/plus.svg")
                                    .size_4()
                                    .text_color(rgb(0xaaaaaa)),
                            ),
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
                                // Settings Icon
                                svg()
                                    .path("icons/settings.svg")
                                    .size_4()
                                    .text_color(rgb(0xaaaaaa)),
                            ),
                    ),
            )
    }
}
