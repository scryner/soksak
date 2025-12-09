use gpui::InteractiveElement;
use gpui::prelude::*;
use gpui::*;
use rust_i18n::t;

pub struct Sidebar {
    job_list: Entity<crate::content::job_list::JobList>,
}

impl Sidebar {
    pub fn new(app: &mut App, job_list: Entity<crate::content::job_list::JobList>) -> Entity<Self> {
        app.new(|_| Self { job_list })
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let job_list = self.job_list.read(cx);
        let counts = job_list.counts(cx);
        let active_filter = job_list.filter;
        let job_list_entity = self.job_list.clone();

        div()
            .w_64()
            .h_full()
            .flex()
            .flex_col()
            .bg(rgb(0x252526)) // Slightly lighter sidebar
            .border_r_1()
            .border_color(rgb(0x333333))
            .child(
                // Spacer for native traffic lights
                div().h_10(),
            )
            .child(self.render_item(
                cx,
                crate::content::job_list::Filter::All,
                active_filter,
                t!("list.all"),
                counts.0,
                crate::icon::Icon::List,
                job_list_entity.clone(),
            ))
            .child(self.render_item(
                cx,
                crate::content::job_list::Filter::Queued,
                active_filter,
                t!("list.queued"),
                counts.1,
                crate::icon::Icon::PauseCircle,
                job_list_entity.clone(),
            ))
            .child(self.render_item(
                cx,
                crate::content::job_list::Filter::Processing,
                active_filter,
                t!("list.processing"),
                counts.2,
                crate::icon::Icon::ProgressActivity,
                job_list_entity.clone(),
            ))
            .child(self.render_item(
                cx,
                crate::content::job_list::Filter::Completed,
                active_filter,
                t!("list.completed"),
                counts.3,
                crate::icon::Icon::CheckCircle,
                job_list_entity.clone(),
            ))
            .child(self.render_item(
                cx,
                crate::content::job_list::Filter::Canceled,
                active_filter,
                t!("list.canceled"),
                counts.4,
                crate::icon::Icon::Delete,
                job_list_entity.clone(),
            ))
            .child(self.render_item(
                cx,
                crate::content::job_list::Filter::Failed,
                active_filter,
                t!("list.failed"),
                counts.5,
                crate::icon::Icon::Warning,
                job_list_entity.clone(),
            ))
    }
}

impl Sidebar {
    fn render_item(
        &self,
        cx: &mut Context<Self>,
        filter: crate::content::job_list::Filter,
        active_filter: crate::content::job_list::Filter,
        label: std::borrow::Cow<'static, str>,
        count: usize,
        icon: crate::icon::Icon,
        job_list: Entity<crate::content::job_list::JobList>,
    ) -> impl IntoElement {
        let is_active = filter == active_filter;
        let bg_color = if is_active {
            rgb(0x28324a) // Active: Dark Blue
        } else {
            rgb(0x252526)
        }; // Active vs Transparent
        let text_color = if is_active {
            rgb(0xffffff)
        } else {
            rgb(0xaaaaaa)
        };

        div()
            .mx_2()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(bg_color)
            .text_color(text_color)
            .cursor_pointer()
            .when(!is_active, |this| {
                this.hover(|style| style.bg(rgb(0x37373d))) // Hover: Light Gray
            })
            .id(SharedString::from(label.clone()))
            .on_click(cx.listener(move |_this, _, _, cx| {
                job_list.update(cx, |jl, cx| jl.set_filter(filter, cx));
            }))
            .flex()
            .justify_between()
            .items_center()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(icon.render().size_5().text_color(text_color))
                    .child(SharedString::from(label)),
            )
            .child(
                div()
                    .px_1()
                    .rounded_sm()
                    .bg(if is_active {
                        rgb(0x424b61) // Active Badge: Lighter Blue-Grey
                    } else {
                        rgb(0x00000000)
                    })
                    .text_xs()
                    .child(format!("{}", count)),
            )
    }
}
