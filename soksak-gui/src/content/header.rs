use gpui::prelude::*;
use gpui::*;
use rust_i18n::t;

use crate::content::job_list::JobList;

pub struct Header {
    job_list: Entity<JobList>,
}

impl Header {
    pub fn new(app: &mut App, job_list: Entity<JobList>) -> Entity<Self> {
        app.new(|_| Self { job_list })
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
                        // Add File Button
                        div()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x333333))
                            .text_sm()
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
                            .child(div().text_color(rgb(0xaaaaaa)).child("+")),
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
                            .cursor_pointer()
                            .child("Profile: Interview")
                            .child(
                                div().text_xs().text_color(rgb(0xaaaaaa)).child("v"), // Simple chevron representation
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
