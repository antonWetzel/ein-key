use std::ops::Not;
use std::path::PathBuf;

// from zed editor source
use gpui::*;
use prelude::FluentBuilder;

use crate::global::Global;
use crate::theme::Color;
use crate::ui::{Export, Import, UI};

const HEIGHT: Pixels = px(32.0);

fn icon_font() -> &'static str {
    "Segoe Fluent Icons"
}

pub fn render_title_bar(active: bool, cx: &mut ViewContext<UI>) -> impl IntoElement {
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
        .flex()
        .flex_row()
        .justify_center()
        .content_stretch()
        .max_h(HEIGHT)
        .min_h(HEIGHT)
        .bg(Color::BackgroundHover)
        .text_color(Color::Foreground)
        .child(div().w_4())
        .child(div().child("Ein-Key"))
        .child(div().w_10())
        .child(
            div()
                .when(active, |div| {
                    div.on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _, cx| save_file(cx, Export)),
                    )
                })
                .when(active.not(), |div| {
                    div.text_color(Color::ForegroundDisabled)
                })
                .px_3()
                .child("Export"),
        )
        .child(
            div()
                .when(active, |div| {
                    div.on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _, cx| open_file(cx, Import)),
                    )
                })
                .when(active.not(), |div| {
                    div.text_color(Color::ForegroundDisabled)
                })
                .px_3()
                .child("Import"),
        )
        .child(div().flex_1())
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

fn open_file<T, E>(cx: &ViewContext<T>, event: impl Fn(PathBuf) -> E + 'static)
where
    T: EventEmitter<E>,
    E: 'static,
{
    let channel = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
    });
    cx.spawn(move |model, mut cx| async move {
        let mut paths = match channel.await {
            Ok(Ok(Some(paths))) => paths,
            _ => return,
        };
        assert_eq!(paths.len(), 1);
        let path = paths.pop().unwrap();
        model.update(&mut cx, |_, cx| cx.emit(event(path))).unwrap();
    })
    .detach();
}

fn save_file<T, E>(cx: &ViewContext<T>, event: impl Fn(PathBuf) -> E + 'static)
where
    T: EventEmitter<E>,
    E: 'static,
{
    let channel = cx.prompt_for_new_path(&Global::current_path());
    cx.spawn(move |model, mut cx| async move {
        let path = match channel.await {
            Ok(Ok(Some(paths))) => paths,
            _ => return,
        };
        model.update(&mut cx, |_, cx| cx.emit(event(path))).unwrap();
    })
    .detach();
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
            .font_family(icon_font())
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
