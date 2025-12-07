use gpui::prelude::*;
use gpui::*;

use crate::content::{
    bottombar::BottomBar,
    header::{Header, HeaderEvent},
    job_list::JobList,
};
use crate::profile::ProfileManager;

mod bottombar;
mod header;
mod job;
pub mod job_list;

pub struct Content {
    header: Entity<Header>,
    bottombar: Entity<BottomBar>,
    job_list: Entity<JobList>,
    profile_manager: Entity<ProfileManager>,
    profile_menu_open: bool,
    menu_position: Option<Point<Pixels>>,
}

impl Content {
    pub fn new(
        app: &mut App,
        job_list: Entity<JobList>,
        profile_manager: Entity<ProfileManager>,
    ) -> Entity<Self> {
        app.new(|inner| {
            let header = Header::new(inner, job_list.clone(), profile_manager.clone());
            inner.subscribe(&header, Self::on_header_event).detach();

            Self {
                header,
                bottombar: BottomBar::new(inner),
                job_list,
                profile_manager,
                profile_menu_open: false,
                menu_position: None,
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

        // Main Content Area
        div()
            .relative() // Make relative so absolute children are relative to this
            .flex_1()
            .h_full()
            .bg(rgb(0x1e1e1e))
            .flex()
            .flex_col()
            .child(self.header.clone())
            .child(self.job_list.clone())
            .child(self.bottombar.clone())
            .when(self.profile_menu_open, |parent| {
                if let Some(pos) = self.menu_position {
                    // Sidebar is width 256px.
                    // The `Content` div starts at x=256 (roughly).
                    // `pos.x` includes the sidebar width.

                    let sidebar_width = px(256.0);

                    let menu_left = pos.x - sidebar_width - px(150.0); // Shift left to align

                    parent
                        .child(
                            // Backdrop to close
                            div().absolute().size_full().top_0().left_0().on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.close_profile_menu(cx);
                                }),
                            ),
                        )
                        .child(
                            // Menu
                            div()
                                .absolute()
                                .top(pos.y - px(10.0)) // Just under the click y
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
