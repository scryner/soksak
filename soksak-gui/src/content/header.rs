use gpui::InteractiveElement;
use gpui::prelude::*;
use gpui::*;
use rust_i18n::t;

use crate::content::job_list::JobList;
use crate::icon::Icon;
use crate::profile::{ProfileEvent, ProfileManager};

pub struct Header {
    job_list: Entity<JobList>,
    profile_manager: Entity<ProfileManager>,
    is_profile_menu_open: bool,
}

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
                is_profile_menu_open: false,
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

    fn toggle_profile_menu(&mut self, cx: &mut Context<Self>) {
        self.is_profile_menu_open = !self.is_profile_menu_open;
        cx.notify();
    }

    fn select_profile(&mut self, profile: String, cx: &mut Context<Self>) {
        self.profile_manager.update(cx, |manager, cx| {
            manager.select_profile(profile, cx);
        });
        self.is_profile_menu_open = false;
        cx.notify();
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
        let profiles = profile_manager.get_profiles().clone();

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
                    .child(SharedString::from(title)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        // Profile Dropdown
                        div()
                            .relative()
                            .child(
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
                                        cx.listener(|this, _, _, cx| {
                                            this.toggle_profile_menu(cx);
                                        }),
                                    )
                                    .child(format!("Profile: {}", current_profile))
                                    .child(Icon::ChevronDown.render().text_color(rgb(0xaaaaaa))),
                            )
                            .when(self.is_profile_menu_open, |parent| {
                                parent.child(
                                    div()
                                        .absolute()
                                        .top(px(30.0))
                                        .right(px(0.0))
                                        .w_40()
                                        .bg(rgb(0x252526))
                                        .border_1()
                                        .border_color(rgb(0x333333))
                                        .rounded_md()
                                        .shadow_md()
                                        // Ensure it's on top by order
                                        .children(profiles.into_iter().map(|profile| {
                                            let is_selected = profile == current_profile;
                                            div()
                                                .px_3()
                                                .py_1()
                                                .text_sm()
                                                .cursor_pointer()
                                                .hover(|s| s.bg(rgb(0x37373d)))
                                                .when(is_selected, |s| s.text_color(rgb(0x4a9eff))) // Highlight selected
                                                .child(profile.clone())
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(move |this, _, _, cx| {
                                                        this.select_profile(profile.clone(), cx);
                                                    }),
                                                )
                                        })),
                                )
                            }),
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
                                cx.spawn(|_, cx: &mut AsyncApp| {
                                    let mut cx: AsyncApp = cx.clone();
                                    async move {
                                        let file = rfd::AsyncFileDialog::new()
                                            .add_filter("Video", &["mp4", "avi"])
                                            .pick_file()
                                            .await;

                                        if let Some(file) = file {
                                            let path = file.path().to_path_buf();
                                            // cx is owned AsyncApp here?
                                            job_list
                                                .update(&mut cx, |job_list, cx| {
                                                    job_list.add_job(cx, path);
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
