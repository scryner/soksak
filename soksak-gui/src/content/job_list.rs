use gpui::*;
use std::path::PathBuf;

use crate::content::job::{Job, Status};

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
}

impl JobList {
    pub fn new(app: &mut App) -> Entity<Self> {
        // app.new(|_| Self { jobs: Vec::new() })
        app.new(|inner| Self {
            jobs: vec![
                Job::new(inner, "A.mp4", "Profile 1", Status::Queued),
                Job::new(inner, "B.mp4", "Profile 2", Status::Queued),
                Job::new(inner, "C.mp4", "Profile 3", Status::Queued),
            ],
            filter: Filter::All,
            selected_index: None,
        })
    }

    pub fn add_job(&mut self, cx: &mut Context<Self>, path: PathBuf) {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let job = cx.new(|_cx| Job {
            name: name.into(),
            profile: "Default".into(),
            status: Status::Queued,
            selected: false,
        });

        self.jobs.push(job);
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

        frame
    }
}
