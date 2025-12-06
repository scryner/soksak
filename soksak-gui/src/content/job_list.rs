use gpui::InteractiveElement;
use gpui::prelude::*;
use gpui::*;
use std::path::PathBuf;

use crate::content::job::{Job, JobEvent, Status};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Filter {
    All,
    Processing,
    Queued,
    Completed,
}

pub struct JobList {
    jobs: Vec<Entity<Job>>,
    pub(crate) filter: Filter,
    selected_index: Option<usize>,
    active_menu: Option<(Entity<Job>, Point<Pixels>)>,
}

impl JobList {
    pub fn new(app: &mut App) -> Entity<Self> {
        // app.new(|_| Self { jobs: Vec::new() })
        app.new(|cx| {
            let jobs = vec![
                Job::new(cx, "A.mp4", "Profile 1", Status::Queued),
                Job::new(cx, "B.mp4", "Profile 2", Status::Queued),
                Job::new(cx, "C.mp4", "Profile 3", Status::Queued),
            ];

            for job in &jobs {
                cx.subscribe(job, Self::on_job_event).detach();
            }

            Self {
                jobs,
                filter: Filter::All,
                selected_index: None,
                active_menu: None,
            }
        })
    }

    pub fn add_job(&mut self, cx: &mut Context<Self>, path: PathBuf) {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let job = Job::new(cx, &name, "Default", Status::Queued);
        cx.subscribe(&job, Self::on_job_event).detach();

        self.jobs.push(job);
        cx.notify();
    }

    pub fn close_menu(&mut self, cx: &mut Context<Self>) {
        self.active_menu = None;
        cx.notify();
    }

    pub fn delete_job(&mut self, job: Entity<Job>, cx: &mut Context<Self>) {
        self.jobs.retain(|j| j != &job);
        self.active_menu = None;
        cx.notify();
    }

    pub fn select_job(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected_index = Some(index);
        for (i, job) in self.jobs.iter().enumerate() {
            job.update(cx, |job, cx| {
                job.set_selected(i == index, cx);
            });
        }
        cx.notify();
    }

    pub fn set_filter(&mut self, filter: Filter, cx: &mut Context<Self>) {
        self.filter = filter;
        cx.notify();
    }

    pub fn counts(&self, cx: &App) -> (usize, usize, usize, usize) {
        let mut all = 0;
        let mut processing = 0;
        let mut queued = 0;
        let mut completed = 0;

        for job in &self.jobs {
            all += 1;
            let status = job.read(cx).status;
            match status {
                Status::Processing => processing += 1,
                Status::Queued => queued += 1,
                Status::Completed => completed += 1,
                Status::Failed => (), // Or count separately if needed
            }
        }

        (all, processing, queued, completed)
    }

    fn on_job_event(&mut self, job: Entity<Job>, event: &JobEvent, cx: &mut Context<Self>) {
        match event {
            JobEvent::MenuOpen(pos) => {
                self.active_menu = Some((job, *pos));
                cx.notify();
            }
        }
    }
}

impl Render for JobList {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut frame = div().flex_1().p_6().flex().flex_col().gap_2();

        for (ix, job) in self.jobs.iter().enumerate() {
            let status = job.read(cx).status;
            let show = match self.filter {
                Filter::All => true,
                Filter::Processing => matches!(status, Status::Processing),
                Filter::Queued => matches!(status, Status::Queued),
                Filter::Completed => matches!(status, Status::Completed),
            };

            if show {
                let job_element = div() // Wrap in a div to handle clicks if needed, or better yet, make functionality part of job?
                    // Actually Job is an Entity, so it renders itself.
                    // We need to wrap it to detect clicks on the job or make the job itself interactive.
                    // Since Job::render returns a div, we can arguably wrap it or just rely on the job to handle its own state if we passed a callback?
                    // But here we are iterating in JobList.
                    // Let's wrap it in a div that handles the click.
                    .id(ix)
                    .child(job.clone())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_job(ix, cx);
                    }));

                frame = frame.child(job_element);
            }
        }

        div().relative().size_full().child(frame).when_some(
            self.active_menu.clone(),
            |parent, (job_entity, pos)| {
                let status = job_entity.read(cx).status;
                let window_size = _window.viewport_size();
                let menu_width = px(100.0); // Approximate width of w_24 (96px) + padding/border
                // Offsets: Sidebar ~256px, Header ~56px.
                let offset_x = px(256.0);
                let offset_y = px(56.0);

                // Calculate X position relative to JobList
                // pos.x is global window coordinate.

                // Check if menu goes offscreen
                // Right edge of menu in window coordinates = pos.x + menu_width
                let mut menu_left = pos.x - offset_x;

                if pos.x + menu_width > window_size.width {
                    // Shift left: align right edge of menu with mouse (or slightly left of it)
                    // New global left = pos.x - menu_width
                    // New local left = (pos.x - menu_width) - offset_x
                    menu_left = pos.x - menu_width - offset_x;
                }

                parent
                    .child(
                        // Backdrop
                        div()
                            .absolute()
                            .top(px(0.0))
                            .left(px(0.0))
                            .size_full()
                            .w(px(3000.0))
                            .h(px(3000.0))
                            .top(px(-1000.0))
                            .left(px(-1000.0))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.close_menu(cx);
                                }),
                            ),
                    )
                    .child(
                        // Menu
                        div()
                            .absolute()
                            .top(pos.y - offset_y)
                            .left(menu_left)
                            .w_24()
                            .bg(rgb(0x252526))
                            .border_1()
                            .border_color(rgb(0x333333))
                            .rounded_md()
                            .shadow_md()
                            .p_1()
                            .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, _| {}))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .py_1()
                                    .px_2()
                                    .rounded_sm()
                                    .cursor_pointer()
                                    .when(matches!(status, Status::Processing), |p| {
                                        p.opacity(0.5).cursor_not_allowed()
                                    })
                                    .when(!matches!(status, Status::Processing), |p| {
                                        p.hover(|s| s.bg(rgb(0x37373d))).on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |this, _, _, cx| {
                                                this.delete_job(job_entity.clone(), cx);
                                            }),
                                        )
                                    })
                                    .child(div().text_sm().text_color(rgb(0xffaaaa)).child("삭제")),
                            ),
                    )
            },
        )
    }
}
