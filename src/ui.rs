use std::path::PathBuf;

use gpui::*;
use prelude::FluentBuilder;

use crate::{
    global::{
        Global, GlobalChanged, GlobalCheck, GlobalChecker, GlobalDelete, GlobalExitEdit,
        GlobalSelect,
    },
    keys::{Side, Stroke},
    theme::Color,
    title_bar::render_title_bar,
};

pub struct UI {
    _global_checker: Model<GlobalChecker>,
    list: ListState,
}

pub struct Import(pub PathBuf);
impl EventEmitter<Import> for UI {}

pub struct Export(pub PathBuf);
impl EventEmitter<Export> for UI {}

impl UI {
    pub fn new(cx: &mut WindowContext) -> View<Self> {
        let global_checker = cx.new_model(|cx| {
            cx.notify();
            GlobalChecker {}
        });
        let list = ListState::new(0, ListAlignment::Top, px(20.0), move |_, _| unreachable!());
        let ui = cx.new_view(|_cx| UI {
            list,
            _global_checker: global_checker.clone(),
        });

        cx.subscribe(
            &global_checker,
            |global_checker, _event: &GlobalCheck, cx| {
                if Global::changed() {
                    cx.update_model(&global_checker, |_, cx| cx.emit(GlobalChanged));
                }

                cx.on_next_frame(move |cx| {
                    cx.update_model(&global_checker, |_, cx| cx.emit(GlobalCheck));
                });
            },
        )
        .detach();

        {
            let global_checker = global_checker.clone();
            cx.on_next_frame(move |cx| {
                cx.update_model(&global_checker, |_, cx| cx.emit(GlobalCheck));
            });
        }

        {
            let ui = ui.clone();
            cx.subscribe(
                &global_checker,
                move |global_checker, _event: &GlobalChanged, cx| {
                    cx.update_view(&ui, |ui, cx| {
                        let scroll = ui.list.logical_scroll_top();
                        ui.list = create_list_state(global_checker);
                        ui.list.scroll_to(scroll);
                        cx.notify()
                    });
                },
            )
            .detach();
        }

        cx.subscribe(&global_checker, move |_, event: &GlobalDelete, _cx| {
            Global::delete(event.0);
        })
        .detach();

        cx.subscribe(&global_checker, move |_, event: &GlobalSelect, _cx| {
            Global::select(event.idx, event.side);
        })
        .detach();

        cx.subscribe(&global_checker, move |_, _event: &GlobalExitEdit, _cx| {
            Global::exit_edit();
        })
        .detach();

        cx.subscribe(&ui, move |_, event: &Import, _cx| {
            Global::import(event.0.clone())
        })
        .detach();

        cx.subscribe(&ui, move |_, event: &Export, _cx| {
            Global::export(event.0.clone())
        })
        .detach();

        ui
    }
}

impl Render for UI {
    fn render(&mut self, cx: &mut gpui::ViewContext<Self>) -> impl gpui::IntoElement {
        let selected = Global::mapping_selected();

        let menu_interactivity = match selected {
            false => Interactivity::Normal,
            true => Interactivity::Disabled,
        };

        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .bg(Color::Background)
            .child(render_title_bar(menu_interactivity.normal(), cx))
            .child(
                div()
                    .w_full()
                    .h_full()
                    .px_10()
                    .child(list(self.list.clone()).w_full().h_full()),
            )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Interactivity {
    Disabled,
    Normal,
    Selected,
}

impl Interactivity {
    pub fn stroke(selected: (usize, Side), idx: usize, side: Side) -> Self {
        if selected.0 == usize::MAX {
            return Self::Normal;
        }

        if selected == (idx, side) {
            return Self::Selected;
        }
        if selected.0 == idx {
            return Self::Normal;
        }

        Self::Disabled
    }

    pub fn close(selected: (usize, Side), idx: usize) -> Self {
        if selected.0 == usize::MAX || selected.0 == idx {
            return Self::Normal;
        }
        Self::Disabled
    }

    pub fn normal(self) -> bool {
        matches!(self, Self::Normal)
    }

    pub fn background(self) -> Color {
        match self {
            Self::Normal => Color::Background,
            Self::Disabled => Color::BackgroundDisabled,
            Self::Selected => Color::BackgroundSelected,
        }
    }

    pub fn foreground(self) -> Color {
        match self {
            Self::Normal => Color::Foreground,
            Self::Disabled => Color::ForegroundDisabled,
            Self::Selected => Color::ForegroundSelected,
        }
    }
}

fn optional_stroke(
    interactivity: Interactivity,
    global_checker: Model<GlobalChecker>,
    stroke: Option<Stroke>,
    event: impl Fn(&mut GlobalChecker, &mut ModelContext<GlobalChecker>) + 'static + Copy,
) -> impl IntoElement {
    div()
        .w_full()
        .min_h_16()
        .bg(interactivity.background())
        .when(interactivity.normal(), |div| {
            div.hover(|div| div.bg(Color::BackgroundSelected))
        })
        .text_color(interactivity.foreground())
        .border_color(interactivity.foreground())
        .on_mouse_down(MouseButton::Left, move |_, cx| {
            cx.update_model(&global_checker, event)
        })
        .border_2()
        .rounded(px(15.0))
        .child(match stroke {
            Some(stroke) => stroke.render(interactivity).into_any_element(),
            None => div().w_full().h_full().into_any_element(),
        })
}

fn create_list_state(global_checker: Model<GlobalChecker>) -> ListState {
    let (items, selected) = Global::state();

    ListState::new(
        items.len(),
        ListAlignment::Top,
        px(20.0),
        move |idx, _cx| {
            let global_checker_del = global_checker.clone();
            let interactivity = Interactivity::close(selected, idx);
            div()
                .flex()
                .flex_row()
                .justify_center()
                .items_center()
                .w_full()
                .gap_2()
                .py_2()
                .text_color(interactivity.foreground())
                .child(optional_stroke(
                    Interactivity::stroke(selected, idx, Side::Input),
                    global_checker.clone(),
                    items[idx].get(Side::Input).cloned(),
                    move |_, cx| {
                        cx.emit(GlobalSelect {
                            idx,
                            side: Side::Input,
                        })
                    },
                ))
                .child(
                    svg()
                        .path("chevron-right.svg")
                        .min_w_10()
                        .min_h_10()
                        .text_color(interactivity.foreground()),
                )
                .child(optional_stroke(
                    Interactivity::stroke(selected, idx, Side::Output),
                    global_checker.clone(),
                    items[idx].get(Side::Output).cloned(),
                    move |_, cx| {
                        cx.emit(GlobalSelect {
                            idx,
                            side: Side::Output,
                        })
                    },
                ))
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .items_center()
                        .min_w_16()
                        .min_h_16()
                        .bg(interactivity.background())
                        .border_2()
                        .rounded(px(15.0))
                        .border_color(interactivity.foreground())
                        .text_color(interactivity.foreground())
                        .when(interactivity.normal(), |div| {
                            div.hover(|div| div.bg(Color::BackgroundHover))
                        })
                        .on_mouse_down(MouseButton::Left, move |_, cx| {
                            if idx == selected.0 {
                                cx.update_model(&global_checker_del, |_, cx| {
                                    cx.emit(GlobalExitEdit)
                                })
                            } else {
                                cx.update_model(&global_checker_del, |_, cx| {
                                    cx.emit(GlobalDelete(idx))
                                })
                            }
                        })
                        .child(
                            svg()
                                .path(if idx == selected.0 {
                                    "check.svg"
                                } else {
                                    "x.svg"
                                })
                                .text_color(interactivity.foreground())
                                .min_w_10()
                                .min_h_10(),
                        ),
                )
                .into_any_element()
        },
    )
}
