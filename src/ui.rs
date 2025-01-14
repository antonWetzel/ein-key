use std::ops::Not;

use gpui::*;
use prelude::FluentBuilder;

use crate::{
    global::{
        Global, GlobalChanged, GlobalCheck, GlobalChecker, GlobalDelete, GlobalExitEdit,
        GlobalSelect,
    },
    keys::Stroke,
};

pub struct UI {
    _global_checker: Model<GlobalChecker>,
    list: ListState,
}

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
            Global::select(event.idx, event.input);
        })
        .detach();

        cx.subscribe(&global_checker, move |_, _event: &GlobalExitEdit, _cx| {
            Global::exit_edit();
        })
        .detach();

        ui
    }
}

impl gpui::Render for UI {
    fn render(&mut self, _cx: &mut gpui::ViewContext<Self>) -> impl gpui::IntoElement {
        div()
            .text_color(white())
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .p_10()
            .child(
                list(self.list.clone())
                    .w_full()
                    .h_full()
                    .p_3()
                    .border_2()
                    .border_color(white()),
            )
    }
}

fn optional_stroke(
    selected: bool,
    global_checker: Model<GlobalChecker>,
    stroke: Option<Stroke>,
    event: impl Fn(&mut GlobalChecker, &mut ModelContext<GlobalChecker>) + 'static + Copy,
) -> impl IntoElement {
    div()
        .w_full()
        .min_h_24()
        .when(selected, |div| div.bg(opaque_grey(0.2, 1.0)))
        .on_mouse_down(MouseButton::Left, move |_, cx| {
            cx.update_model(&global_checker, event)
        })
        .border_2()
        .border_color(white())
        .rounded(px(15.0))
        .child(match stroke {
            Some(stroke) => stroke.into_any_element(),
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
            div()
                .flex()
                .flex_row()
                .justify_center()
                .items_center()
                .w_full()
                .gap_2()
                .py_2()
                .child(optional_stroke(
                    idx == selected.0 && selected.1,
                    global_checker.clone(),
                    items[idx].get(true).cloned(),
                    move |_, cx| cx.emit(GlobalSelect { idx, input: true }),
                ))
                .child(">")
                .child(optional_stroke(
                    idx == selected.0 && selected.1.not(),
                    global_checker.clone(),
                    items[idx].get(false).cloned(),
                    move |_, cx| cx.emit(GlobalSelect { idx, input: false }),
                ))
                .child("|")
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .items_center()
                        .min_w_24()
                        .min_h_24()
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
                        .border_2()
                        .border_color(white())
                        .rounded(px(15.0))
                        .child(if idx == selected.0 { "O" } else { "X" }),
                )
                .into_any_element()
        },
    )
}
