use gpui::*;

#[derive(Clone, Copy)]
pub enum Icon {
    EllipsisVertical,
    FileVideo,
}

impl Icon {
    pub fn path(self) -> SharedString {
        match self {
            Icon::EllipsisVertical => "icons/ellipsis-vertical.svg".into(),
            Icon::FileVideo => "icons/file-video.svg".into(),
        }
    }

    pub fn render(self) -> Svg {
        svg().path(self.path()).size_6()
    }
}
