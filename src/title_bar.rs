// copied from zed editor and edited

use gpui::*;

#[derive(IntoElement)]
pub struct WindowsWindowControls {
    button_height: Pixels,
}

impl WindowsWindowControls {
    pub fn new() -> Self {
        Self {
            button_height: px(32.0),
        }
    }

    fn get_font() -> &'static str {
        "Segoe Fluent Icons"
    }
}

impl RenderOnce for WindowsWindowControls {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let close_button_hover_color = Rgba {
            r: 232.0 / 255.0,
            g: 17.0 / 255.0,
            b: 32.0 / 255.0,
            a: 1.0,
        };

        let button_hover_color = Rgba {
            r: 0.9,
            g: 0.9,
            b: 0.9,
            a: 0.1,
        };

        div()
            .id("windows-window-controls")
            .font_family(Self::get_font())
            .flex()
            .flex_row()
            .justify_center()
            .content_stretch()
            .max_h(self.button_height)
            .min_h(self.button_height)
            .child(WindowsCaptionButton::new(
                "minimize",
                WindowsCaptionButtonIcon::Minimize,
                button_hover_color,
            ))
            .child(WindowsCaptionButton::new(
                "maximize-or-restore",
                if cx.is_maximized() {
                    WindowsCaptionButtonIcon::Restore
                } else {
                    WindowsCaptionButtonIcon::Maximize
                },
                button_hover_color,
            ))
            .child(WindowsCaptionButton::new(
                "close",
                WindowsCaptionButtonIcon::Close,
                close_button_hover_color,
            ))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
enum WindowsCaptionButtonIcon {
    Minimize,
    Restore,
    Maximize,
    Close,
}

#[derive(IntoElement)]
struct WindowsCaptionButton {
    id: ElementId,
    icon: WindowsCaptionButtonIcon,
    hover_background_color: Rgba,
}

impl WindowsCaptionButton {
    pub fn new(
        id: impl Into<ElementId>,
        icon: WindowsCaptionButtonIcon,
        hover_background_color: Rgba,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            hover_background_color,
        }
    }
}

impl RenderOnce for WindowsCaptionButton {
    fn render(self, _cx: &mut WindowContext) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .id(self.id)
            .justify_center()
            .content_center()
            .w(px(36.0))
            .h_full()
            .text_size(px(10.0))
            .hover(|style| style.bg(self.hover_background_color))
            .child(match self.icon {
                WindowsCaptionButtonIcon::Minimize => "\u{e921}",
                WindowsCaptionButtonIcon::Restore => "\u{e923}",
                WindowsCaptionButtonIcon::Maximize => "\u{e922}",
                WindowsCaptionButtonIcon::Close => "\u{e8bb}",
            })
    }
}
