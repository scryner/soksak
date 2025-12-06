use gpui::*;

#[derive(Clone, Copy)]
pub enum Icon {
    EllipsisVertical,
    FileVideo,
    List,
    ProgressActivity,
    PauseCircle,
    CheckCircle,
}

impl Icon {
    pub fn path(self) -> SharedString {
        match self {
            Icon::EllipsisVertical => "icons/ellipsis-vertical.svg".into(),
            Icon::FileVideo => "icons/file-video.svg".into(),
            Icon::List => "icons/list.svg".into(),
            Icon::ProgressActivity => "icons/progress_activity.svg".into(),
            Icon::PauseCircle => "icons/pause_circle.svg".into(),
            Icon::CheckCircle => "icons/check_circle.svg".into(),
        }
    }

    pub fn render(self) -> Svg {
        svg().path(self.path()).size_6()
    }
}
