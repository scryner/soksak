use gpui::*;

use crate::content::{bottombar::BottomBar, header::Header, job_list::JobList};

mod bottombar;
mod header;
mod job;
mod job_list;

pub struct Content {
    header: Entity<Header>,
    bottombar: Entity<BottomBar>,
    job_list: Entity<JobList>,
}

impl Content {
    pub fn new(app: &mut App) -> Entity<Self> {
        app.new(|inner| Self {
            header: Header::new(inner),
            bottombar: BottomBar::new(inner),
            job_list: JobList::new(inner),
        })
    }
}

impl Render for Content {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Main Content Area
        div()
            .flex_1()
            .h_full()
            .bg(rgb(0x1e1e1e))
            .flex()
            .flex_col()
            .child(self.header.clone())
            .child(self.job_list.clone())
            .child(self.bottombar.clone())
    }
}
