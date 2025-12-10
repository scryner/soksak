use gpui::{prelude::*, *};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Sender},
};
use std::time::Duration;

use crate::content::{
    bottombar::{BottomBar, BottomBarEvent},
    header::{Header, HeaderEvent},
    job::Status,
    job_list::JobList,
};
use crate::profile::ProfileManager;
use crate::progress::{GuiProgress, ProgressEvent};

mod bottombar;
mod header;
mod job;
pub mod job_list;
pub mod process_task;

pub struct Content {
    header: Entity<Header>,
    bottombar: Entity<BottomBar>,
    job_list: Entity<JobList>,
    profile_manager: Entity<ProfileManager>,
    profile_menu_open: bool,
    menu_position: Option<Point<Pixels>>,
    progress_tx: Sender<ProgressEvent>,
    cancel_flag: Arc<AtomicBool>,
    cancel_tx: Option<tokio::sync::oneshot::Sender<()>>,
    is_processing: bool,

    progress_total: Option<u64>,
    progress_current: u64,
}

impl Content {
    pub const SUPPORTED_VIDEO_EXTENSIONS: [&str; 2] = ["mp4", "avi"];

    pub fn new(
        app: &mut App,
        job_list: Entity<JobList>,
        profile_manager: Entity<ProfileManager>,
    ) -> Entity<Self> {
        app.new(|inner| {
            let header = Header::new(inner, job_list.clone(), profile_manager.clone());
            inner.subscribe(&header, Self::on_header_event).detach();
            let bottombar = BottomBar::new(inner);
            inner
                .subscribe(&bottombar, Self::on_bottombar_event)
                .detach();

            let (tx, rx) = mpsc::channel();
            let cancel_flag = Arc::new(AtomicBool::new(false));

            // Spawn monitoring task
            inner
                .spawn(|this: gpui::WeakEntity<Content>, cx: &mut gpui::AsyncApp| {
                    let cx = cx.clone();
                    async move {
                        loop {
                            let mut received = false;
                            while let Ok(msg) = rx.try_recv() {
                                received = true;
                                // Check if entity still exists
                                if cx
                                    .update(|cx| {
                                        this.update(cx, |content, cx| {
                                            content.handle_progress(msg, cx)
                                        })
                                    })
                                    .is_err()
                                {
                                    return; // Entity dropped
                                }
                            }

                            if !received {
                                cx.background_executor()
                                    .timer(Duration::from_millis(16))
                                    .await;
                            }
                        }
                    }
                })
                .detach();

            Self {
                header,
                bottombar,
                job_list,
                profile_manager,
                profile_menu_open: false,
                menu_position: None,
                progress_tx: tx,
                cancel_flag,
                cancel_tx: None,
                is_processing: false,

                progress_total: None,
                progress_current: 0,
            }
        })
    }

    fn on_header_event(
        &mut self,
        _header: Entity<Header>,
        event: &HeaderEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            HeaderEvent::ToggleProfileMenu(position) => {
                self.profile_menu_open = !self.profile_menu_open;
                self.menu_position = Some(*position);
                cx.notify();
            }
        }
    }

    fn on_bottombar_event(
        &mut self,
        _bottombar: Entity<BottomBar>,
        event: &BottomBarEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            BottomBarEvent::Start => self.start_processing(cx),
            BottomBarEvent::Cancel => self.cancel_processing(cx),
        }
    }

    fn handle_progress(&mut self, event: ProgressEvent, cx: &mut Context<Self>) {
        if !self.is_processing {
            return;
        }

        match event {
            ProgressEvent::Inc(delta) => {
                self.progress_current += delta;
                if let Some(total) = self.progress_total {
                    self.bottombar.update(cx, |bar, cx| {
                        bar.set_progress_detailed(self.progress_current, total, cx);
                    });
                }
            }
            ProgressEvent::SetPosition(pos) => {
                self.progress_current = pos;
                if let Some(total) = self.progress_total {
                    self.bottombar.update(cx, |bar, cx| {
                        bar.set_progress_detailed(pos, total, cx);
                    });
                } else {
                    self.bottombar
                        .update(cx, |bar, cx| bar.set_progress(pos as f32 / 100.0, cx));
                }
            }
            ProgressEvent::SetMessage(msg) => {
                self.bottombar
                    .update(cx, |bar, cx| bar.set_message(msg, cx));
            }
            ProgressEvent::Finish => {
                self.progress_total = None;
                self.bottombar
                    .update(cx, |bar, cx| bar.set_progress(1.0, cx));
            }
            ProgressEvent::FinishWithMessage(msg) => {
                self.progress_total = None;
                self.bottombar.update(cx, |bar, cx| {
                    bar.set_message(msg, cx);
                    bar.set_progress(1.0, cx);
                });
            }
            ProgressEvent::SetLength(len) => {
                self.progress_total = Some(len);
                self.bottombar.update(cx, |bar, cx| {
                    bar.set_progress_detailed(self.progress_current, len, cx);
                });
            }
        }
    }

    fn start_processing(&mut self, cx: &mut Context<Self>) {
        if self.is_processing {
            return;
        }

        self.is_processing = true;
        self.progress_total = None;
        self.progress_current = 0;
        self.cancel_flag.store(false, Ordering::SeqCst);
        self.bottombar
            .update(cx, |bar, cx| bar.set_running(true, cx));

        self.process_next_job(cx);
    }

    fn cancel_processing(&mut self, cx: &mut Context<Self>) {
        self.cancel_flag.store(true, Ordering::SeqCst);
        self.is_processing = false;

        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.send(());
        }

        // Find current processing job and mark as Canceling
        self.job_list.update(cx, |list, cx| {
            list.mark_processing_as_canceling(cx);
        });

        self.bottombar.update(cx, |bar, cx| {
            bar.set_running(false, cx);
            bar.set_message(rust_i18n::t!("progress.ready"), cx);
            bar.set_progress(0.0, cx);
        });
    }

    fn process_next_job(&mut self, cx: &mut Context<Self>) {
        if self.cancel_flag.load(Ordering::SeqCst) {
            self.is_processing = false;
            self.bottombar.update(cx, |bar, cx| {
                bar.set_running(false, cx);
                bar.set_message(rust_i18n::t!("progress.ready"), cx);
                bar.set_progress(0.0, cx);
            });
            return;
        }

        let job_opt = self.job_list.read(cx).get_next_queued_job(cx);

        if let Some(job) = job_opt {
            self.job_list.update(cx, |list, cx| {
                list.mark_job_status(job.clone(), Status::Processing, cx);
            });

            let path_str = job.read(cx).get_path();
            let path = std::path::PathBuf::from(path_str);
            let profile = job.read(cx).profile.to_string();

            let progress_tx = self.progress_tx.clone();
            let gui_progress = GuiProgress::new(progress_tx);

            let (tx, rx) = tokio::sync::oneshot::channel();
            self.cancel_tx = Some(tx);

            // Spawn background work
            cx.spawn(|this: gpui::WeakEntity<Content>, cx: &mut gpui::AsyncApp| {
                let cx = cx.clone();
                async move {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            crate::content::process_task::run_job(path, profile, gui_progress, rx)
                                .await
                        })
                        .await;

                    cx.update(|cx| {
                        this.update(cx, move |content, cx| {
                            // Check cancellation first, regardless of result
                            if content.cancel_flag.load(Ordering::SeqCst) {
                                content.job_list.update(cx, |list, cx| {
                                    list.mark_job_status(job, Status::Canceled, cx);
                                });
                            } else {
                                match result {
                                    Ok(_) => {
                                        content.job_list.update(cx, |list, cx| {
                                            list.mark_job_status(job, Status::Completed, cx);
                                        });
                                    }
                                    Err(e) => {
                                        eprintln!("Job failed: {:?}", e);
                                        content.job_list.update(cx, |list, cx| {
                                            list.mark_job_status(job, Status::Failed, cx);
                                        });
                                    }
                                }
                            }
                            content.process_next_job(cx);
                        })
                        .ok();
                    })
                    .ok();
                }
            })
            .detach();
        } else {
            self.is_processing = false;
            self.bottombar.update(cx, |bar, cx| {
                bar.set_running(false, cx);
                bar.set_message(rust_i18n::t!("progress.ready"), cx);
                bar.set_progress(0.0, cx);
            });
        }
    }

    fn close_profile_menu(&mut self, cx: &mut Context<Self>) {
        self.profile_menu_open = false;
        cx.notify();
    }

    fn select_profile(&mut self, profile: String, cx: &mut Context<Self>) {
        self.profile_manager.update(cx, |manager, cx| {
            manager.select_profile(profile, cx);
        });
        self.close_profile_menu(cx);
    }
}

impl Render for Content {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let profile_manager = self.profile_manager.read(cx);
        let current_profile = profile_manager
            .get_current_profile()
            .cloned()
            .unwrap_or_else(|| "Default".to_string());
        let profiles = profile_manager.get_profiles().clone();

        div()
            .relative()
            .flex_1()
            .h_full()
            .bg(rgb(0x1e1e1e))
            .flex()
            .flex_col()
            .child(self.header.clone())
            .child(
                div()
                    .flex_1()
                    .size_full()
                    .child(self.job_list.clone())
                    .on_drop(cx.listener(|this, dropped: &ExternalPaths, _, cx| {
                        let paths = dropped.paths();
                        let profile_manager = this.profile_manager.read(cx);
                        let current_profile = profile_manager
                            .get_current_profile()
                            .cloned()
                            .unwrap_or_else(|| "Default".to_string());

                        this.job_list.update(cx, |job_list, cx| {
                            for path in paths {
                                if let Some(ext) = path.extension() {
                                    if let Some(ext_str) = ext.to_str() {
                                        let ext_lower = ext_str.to_lowercase();
                                        if Self::SUPPORTED_VIDEO_EXTENSIONS
                                            .contains(&ext_lower.as_str())
                                        {
                                            job_list.add_job(
                                                cx,
                                                path.clone(),
                                                current_profile.clone(),
                                            );
                                        }
                                    }
                                }
                            }
                        });
                    })),
            )
            .child(self.bottombar.clone())
            .when(self.profile_menu_open, |parent| {
                if let Some(pos) = self.menu_position {
                    let sidebar_width = px(256.0);
                    let menu_left = pos.x - sidebar_width - px(150.0);

                    parent
                        .child(div().absolute().size_full().top_0().left_0().on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.close_profile_menu(cx);
                            }),
                        ))
                        .child(
                            div()
                                .absolute()
                                .top(pos.y - px(10.0))
                                .left(menu_left)
                                .w_40()
                                .bg(rgb(0x252526))
                                .border_1()
                                .border_color(rgb(0x333333))
                                .rounded_md()
                                .shadow_md()
                                .children(profiles.into_iter().map(|profile| {
                                    let is_selected = profile == current_profile;
                                    div()
                                        .px_3()
                                        .py_1()
                                        .text_sm()
                                        .cursor_pointer()
                                        .hover(|s| s.bg(rgb(0x37373d)))
                                        .when(is_selected, |s| s.text_color(rgb(0x4a9eff)))
                                        .child(profile.clone())
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, _, _, cx| {
                                                this.select_profile(profile.clone(), cx);
                                            }),
                                        )
                                })),
                        )
                } else {
                    parent
                }
            })
    }
}
