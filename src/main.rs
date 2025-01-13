#![allow(static_mut_refs)]

use std::{
    ops::Not,
    sync::{LazyLock, Mutex},
};

use gpui::*;
use windows::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

// todo?: use mutex
static GLOBAL: LazyLock<Mutex<Global>> = LazyLock::new(|| Mutex::new(Global::new()));

#[derive(Debug, Clone, IntoElement)]
pub struct Stroke {
    keyboard: Box<[u8; 256]>,
    key: VIRTUAL_KEY,
}

impl RenderOnce for Stroke {
    fn render(self, _cx: &mut WindowContext) -> impl IntoElement {
        div()
            .border_color(white())
            .border_2()
            .text_color(white())
            .w_full()
            .flex()
            .justify_center()
            .rounded(px(3.0))
            .child(format!("{}", self.char()))
    }
}

#[derive(Debug, Clone)]
pub struct Mapping {
    input: Option<Stroke>,
    output: Option<Stroke>,
}

#[derive(Debug)]
pub struct Global {
    action: Action,
    dirty: bool,
    keyboard: Box<[u8; 256]>,

    mappings: Vec<Mapping>,
}

#[derive(Debug)]
pub enum Action {
    Normal,
    StartRecordIn,
    RecordIn(Stroke),
    StartRecordOut(Stroke),
    RecordOut { input: Stroke, output: Stroke },
}

impl Global {
    pub fn new() -> Self {
        Self {
            action: Action::Normal,
            mappings: Vec::new(),
            dirty: false,
            keyboard: Box::new([0; 256]),
        }
    }

    pub fn start_recording_input(&mut self) {
        println!("start recording input");
        self.action = Action::StartRecordIn;
    }

    pub fn start_recording_output(&mut self) {
        println!("start recording output");
        self.action = match std::mem::replace(&mut self.action, Action::Normal) {
            Action::RecordIn(stroke) => {
                println!("In: {:?}", stroke.char());
                Action::StartRecordOut(stroke)
            }
            action => action,
        };
    }

    pub fn end_recording(&mut self) {
        println!("end recording");
        self.action = match std::mem::replace(&mut self.action, Action::Normal) {
            Action::RecordOut { input, output } => {
                println!("Map: {:?} {:?}", input.char(), output.char());
                for i in 0..256 {
                    if input.keyboard[i] != 0 {
                        println!("  In: {:?}", i);
                    }
                }
                for i in 0..256 {
                    if output.keyboard[i] != 0 {
                        println!("  Out: {:?}", i);
                    }
                }
                self.mappings.push(Mapping {
                    input: Some(input),
                    output: Some(output),
                });
                self.dirty = true;
                Action::Normal
            }
            action => action,
        };
    }

    pub fn mapped_key(&self, key: VIRTUAL_KEY, state: KeyState) -> Status {
        for mapping in self.mappings.iter() {
            let Some(input) = &mapping.input else {
                continue;
            };

            if input.key != key {
                continue;
            }

            let valid = input
                .keyboard
                .iter()
                .copied()
                .zip(self.keyboard.iter().copied())
                .all(|(target, current)| current & 0x80 != 0 || target & 0x80 == 0);

            if valid.not() {
                continue;
            }

            let mut keyboard = Box::new([0u8; 256]);
            unsafe { GetKeyboardState(&mut keyboard) }.unwrap();

            return match mapping.output.clone() {
                Some(stroke) => Status::Replace { stroke, state },
                None => Status::Intercept,
            };
        }
        Status::Allow
    }

    fn handle_key(&mut self, kb_struct: &KBDLLHOOKSTRUCT) -> Status {
        let vk_code = VIRTUAL_KEY(kb_struct.vkCode as u16);

        if kb_struct.flags.contains(LLKHF_INJECTED) {
            return Status::Allow;
        }

        let state = match kb_struct.flags.contains(LLKHF_UP) {
            false => KeyState::Pressed,
            true => KeyState::Released,
        };

        self.handle_real_key(vk_code, state)
    }

    fn handle_real_key(&mut self, key: VIRTUAL_KEY, state: KeyState) -> Status {
        if state.released() {
            self.set_key(key, state);
        }

        let (action, status) = match std::mem::replace(&mut self.action, Action::Normal) {
            Action::Normal => (Action::Normal, self.mapped_key(key, state)),

            Action::StartRecordIn | Action::RecordIn { .. } if state.pressed() => {
                self.dirty = true;
                let keyboard = self.keyboard.clone();
                let stroke = Stroke { key, keyboard };

                println!("In: {:?}", stroke.char());

                let action = Action::RecordIn(stroke);

                (action, Status::Allow)
            }
            Action::StartRecordOut(input) | Action::RecordOut { input, .. } if state.pressed() => {
                self.dirty = true;
                let keyboard = self.keyboard.clone();
                let output = Stroke { key, keyboard };
                println!("Out: {:?}", output.char());
                let action = Action::RecordOut { input, output };
                (action, Status::Allow)
            }
            action => (action, Status::Allow),
        };
        if state.pressed() {
            self.set_key(key, state);
        }
        self.action = action;
        status
    }

    fn set_key(&mut self, key: VIRTUAL_KEY, state: KeyState) {
        let value = match state {
            KeyState::Pressed => 0x80,
            KeyState::Released => 0,
        };
        self.keyboard[key.0 as usize] = value;

        match key {
            VK_LSHIFT | VK_RSHIFT => self.combine_keys(VK_SHIFT, VK_LSHIFT, VK_RSHIFT),
            VK_LCONTROL | VK_RCONTROL => self.combine_keys(VK_CONTROL, VK_LCONTROL, VK_RCONTROL),
            VK_LMENU | VK_RMENU => self.combine_keys(VK_MENU, VK_LMENU, VK_RMENU),
            _ => {}
        }
    }

    fn combine_keys(&mut self, target: VIRTUAL_KEY, left: VIRTUAL_KEY, right: VIRTUAL_KEY) {
        self.keyboard[target.0 as usize] =
            self.keyboard[left.0 as usize] | self.keyboard[right.0 as usize]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum KeyState {
    Released,
    Pressed,
}

impl KeyState {
    pub fn pressed(self) -> bool {
        matches!(self, Self::Pressed)
    }

    pub fn released(self) -> bool {
        matches!(self, Self::Released)
    }
}

pub enum Status {
    Intercept,
    Allow,
    Replace { stroke: Stroke, state: KeyState },
}

fn send_key(key: VIRTUAL_KEY, state: KeyState) {
    let flags = match state {
        KeyState::Pressed => KEYBD_EVENT_FLAGS(0),
        KeyState::Released => KEYEVENTF_KEYUP,
    };

    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe { SendInput(&[input], size_of::<INPUT>() as i32) };
}

extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code as u32 == HC_ACTION
        && (w_param == WPARAM(WM_KEYDOWN as usize)
            || w_param == WPARAM(WM_SYSKEYDOWN as usize)
            || w_param == WPARAM(WM_KEYUP as usize)
            || w_param == WPARAM(WM_SYSKEYUP as usize))
    {
        let kb_struct = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };

        let mut global = GLOBAL.lock().unwrap();
        let stroke = match global.handle_key(kb_struct) {
            Status::Intercept => return LRESULT(1),
            Status::Replace { stroke, state } => Some((stroke, state)),
            Status::Allow => None,
        };
        drop(global);
        if let Some((stroke, state)) = stroke {
            unsafe { SetKeyboardState(&stroke.keyboard) }.unwrap();
            send_key(stroke.key, state);
            unsafe { SetKeyboardState(&stroke.keyboard) }.unwrap();
            return LRESULT(1);
        }
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

fn create_list_state(facade: Model<Facade>) -> ListState {
    let items = GLOBAL.lock().unwrap().mappings.clone();

    ListState::new(
        items.len(),
        ListAlignment::Top,
        px(20.0),
        move |idx, _cx| {
            let facade = facade.clone();
            div()
                .flex()
                .flex_row()
                .w_full()
                .gap_2()
                .py_2()
                .child(match items[idx].input.clone() {
                    Some(stroke) => stroke.into_any_element(),
                    None => div().into_any_element(),
                })
                .child(">")
                .child(match items[idx].output.clone() {
                    Some(stroke) => stroke.into_any_element(),
                    None => div().into_any_element(),
                })
                .child("|")
                .child(
                    div()
                        .on_mouse_down(MouseButton::Left, move |_, cx| {
                            cx.update_model(&facade, |_, cx| cx.emit(GlobalDelete(idx)))
                        })
                        .border_2()
                        .border_color(white())
                        .rounded(px(5.0))
                        .w_20()
                        .flex()
                        .justify_center()
                        .child("X"),
                )
                .into_any_element()
        },
    )
}

fn main() {
    let handle = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), None, 0).unwrap()
    };

    gpui::App::new().run(|cx| {
        cx.open_window(gpui::WindowOptions::default(), |cx| {
            let facade = cx.new_model(|cx| {
                cx.notify();
                Facade {}
            });
            let list = ListState::new(0, ListAlignment::Top, px(20.0), move |_, _| unreachable!());
            let ui = cx.new_view(|_cx| UI {
                list,
                _facade: facade.clone(),
            });

            cx.subscribe(&facade, |facade, _event: &FacadeCheck, cx| {
                let mut global = GLOBAL.lock().unwrap();
                if global.dirty {
                    global.dirty = false;
                    cx.update_model(&facade, |_, cx| cx.emit(GlobalChanged));
                }

                cx.on_next_frame(move |cx| {
                    cx.update_model(&facade, |_, cx| cx.emit(FacadeCheck));
                });
            })
            .detach();

            {
                let facade = facade.clone();
                cx.on_next_frame(move |cx| {
                    cx.update_model(&facade, |_, cx| cx.emit(FacadeCheck));
                });
            }

            {
                let ui = ui.clone();
                cx.subscribe(&facade, move |facade, _event: &GlobalChanged, cx| {
                    cx.update_view(&ui, |ui, cx| {
                        ui.list = create_list_state(facade);
                        cx.notify()
                    });
                })
                .detach();
            }

            {
                cx.subscribe(&facade, move |_facade, event: &GlobalDelete, _cx| {
                    let mut global = GLOBAL.lock().unwrap();
                    if event.0 >= global.mappings.len() {
                        println!("Remove out of bounds of mappings, how?");
                        return;
                    }
                    global.mappings.swap_remove(event.0);
                    global.dirty = true;
                })
                .detach();
            }

            ui
        })
        .unwrap();
        cx.activate(true);
    });

    unsafe { UnhookWindowsHookEx(handle) }.unwrap();
}

struct UI {
    _facade: Model<Facade>,
    list: ListState,
}

struct Facade {}

struct FacadeCheck;
impl EventEmitter<FacadeCheck> for Facade {}

struct GlobalChanged;
impl EventEmitter<GlobalChanged> for Facade {}

struct GlobalDelete(usize);
impl EventEmitter<GlobalDelete> for Facade {}

impl gpui::Render for UI {
    fn render(&mut self, cx: &mut gpui::ViewContext<Self>) -> impl gpui::IntoElement {
        div()
            .text_color(white())
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_5()
                    .child(
                        div()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |_ui, _event, _cx| {
                                    GLOBAL.lock().unwrap().start_recording_input();
                                }),
                            )
                            .child("Input"),
                    )
                    .child(
                        div()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |_ui, _event, _cx| {
                                    GLOBAL.lock().unwrap().start_recording_output();
                                }),
                            )
                            .child("Output"),
                    )
                    .child(
                        div()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |_ui, _event, _cx| {
                                    GLOBAL.lock().unwrap().end_recording();
                                }),
                            )
                            .child("Save"),
                    ),
            )
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

impl Stroke {
    fn char(&self) -> char {
        let mut unicode_buffer = [0u16; 2];

        unsafe {
            ToUnicodeEx(
                self.key.0 as u32,
                MapVirtualKeyW(self.key.0 as u32, MAPVK_VK_TO_VSC),
                &self.keyboard,
                &mut unicode_buffer,
                0,
                None,
            )
        };

        String::from_utf16(&unicode_buffer)
            .unwrap()
            .chars()
            .next()
            .unwrap()
    }
}
