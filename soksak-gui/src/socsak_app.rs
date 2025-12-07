use gpui::*;

use crate::{content::Content, sidebar::Sidebar};

pub struct SoksakApp {
    sidebar: Entity<Sidebar>,
    content: Entity<Content>,
    _profile_manager: Entity<crate::profile::ProfileManager>,
}

impl SoksakApp {
    pub fn new(app: &mut App, profile_manager: Entity<crate::profile::ProfileManager>) -> Self {
        let job_list = crate::content::job_list::JobList::new(app);
        Self {
            sidebar: Sidebar::new(app, job_list.clone()),
            content: Content::new(app, job_list, profile_manager.clone()),
            _profile_manager: profile_manager,
        }
    }
}

impl Render for SoksakApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .bg(rgb(0x1e1e1e)) // Dark background
            .text_color(rgb(0xffffff))
            .key_context("Workspace")
            .child(self.sidebar.clone())
            .child(self.content.clone())
    }
}
