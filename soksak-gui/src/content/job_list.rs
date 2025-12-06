use gpui::*;
use std::path::PathBuf;

use crate::content::job::{Job, Status};

pub struct JobList {
    jobs: Vec<Entity<Job>>,
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
        });

        self.jobs.push(job);
        cx.notify();
    }
}

impl Render for JobList {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut frame = div().flex_1().p_6().flex().flex_col().gap_2();

        for job in self.jobs.iter() {
            frame = frame.child(job.clone());
        }

        frame
    }
}
