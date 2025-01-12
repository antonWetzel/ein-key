#![allow(static_mut_refs)]

use std::{
    collections::{HashMap, HashSet},
    mem::MaybeUninit,
    ops::Not,
};

use gpui::*;
use windows::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

static mut GLOBAL: MaybeUninit<Global> = MaybeUninit::uninit();

#[derive(Debug)]
pub struct Global {
    handle: HHOOK,
    action: Action,
    dirty: bool,
    keyboard: Box<[u8; 256]>,

    disable: HashSet<u16>,
    mappings: HashMap<u16, Vec<Mapping>>,
}

#[derive(Debug)]
struct Mapping {
    in_keyboard: Box<[u8; 256]>,
    out_keyboard: Box<[u8; 256]>,
    key: VIRTUAL_KEY,
}

#[derive(Debug)]
pub enum Action {
    Normal,
    StartDisable,
    Disable(VIRTUAL_KEY),
    StartRecordIn,
    RecordIn {
        keyboard: Box<[u8; 256]>,
        key: VIRTUAL_KEY,
    },
    StartRecordOut {
        in_keyboard: Box<[u8; 256]>,
        in_key: VIRTUAL_KEY,
    },
    RecordOut {
        in_keyboard: Box<[u8; 256]>,
        in_key: VIRTUAL_KEY,

        out_keyboard: Box<[u8; 256]>,
        out_key: VIRTUAL_KEY,
    },
}

impl Global {
    pub fn start_disable(&mut self) {
        println!("start disable");
        self.action = Action::StartDisable;
    }

    pub fn start_recording_input(&mut self) {
        println!("start recording input");
        self.action = Action::StartRecordIn;
    }

    pub fn start_recording_output(&mut self) {
        println!("start recording output");
        self.action = match std::mem::replace(&mut self.action, Action::Normal) {
            Action::RecordIn { keyboard, key } => {
                let key_2 = virtual_key_to_unicode(&keyboard, key);
                println!("In: {:?}", key_2);
                Action::StartRecordOut {
                    in_keyboard: keyboard,
                    in_key: key,
                }
            }
            action => action,
        };
    }

    pub fn end_recording(&mut self) {
        println!("end recording");
        self.action = match std::mem::replace(&mut self.action, Action::Normal) {
            Action::Disable(key) => {
                self.disable.insert(key.0);
                Action::Normal
            }
            Action::RecordOut {
                in_keyboard,
                in_key,
                out_keyboard,
                out_key,
            } => {
                let key_in = virtual_key_to_unicode(&in_keyboard, in_key);
                let key_out = virtual_key_to_unicode(&out_keyboard, out_key);
                println!("Map: {:?} {:?}", key_in, key_out);
                for i in 0..256 {
                    if in_keyboard[i] != 0 {
                        println!("  In: {:?}", i);
                    }
                }
                for i in 0..256 {
                    if out_keyboard[i] != 0 {
                        println!("  Out: {:?}", i);
                    }
                }

                self.mappings.entry(in_key.0).or_default().push(Mapping {
                    in_keyboard,
                    out_keyboard,
                    key: out_key,
                });
                Action::Normal
            }
            action => action,
        };
    }

    pub fn mapped_key(&self, key: VIRTUAL_KEY, state: KeyState) -> Status {
        if self.disable.contains(&key.0) {
            return Status::Intercept;
        }
        let Some(mappings) = self.mappings.get(&key.0) else {
            return Status::Allow;
        };

        for mapping in mappings.iter() {
            let valid = mapping
                .in_keyboard
                .iter()
                .copied()
                .zip(self.keyboard.iter().copied())
                .all(|(target, current)| current & 0x80 != 0 || target & 0x80 == 0);

            if valid.not() {
                continue;
            }

            let mut keyboard = Box::new([0u8; 256]);
            unsafe { GetKeyboardState(&mut keyboard) }.unwrap();

            unsafe { SetKeyboardState(&mapping.out_keyboard) }.unwrap();
            send_key(mapping.key, state);
            unsafe { SetKeyboardState(&keyboard) }.unwrap();

            return Status::Intercept;
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

            Action::StartDisable | Action::Disable(_) if state.pressed() => {
                self.dirty = true;
                (Action::Disable(key), Status::Allow)
            }

            Action::StartRecordIn | Action::RecordIn { .. } if state.pressed() => {
                self.dirty = true;
                let keyboard = self.keyboard.clone();

                println!("In: {:?}", virtual_key_to_unicode(&keyboard, key));

                let action = Action::RecordIn { keyboard, key };

                (action, Status::Allow)
            }
            Action::StartRecordOut {
                in_keyboard,
                in_key,
            }
            | Action::RecordOut {
                in_keyboard,
                in_key,
                ..
            } if state.pressed() => {
                self.dirty = true;
                let keyboard = self.keyboard.clone();
                self.dirty = true;
                println!("Out: {:?}", virtual_key_to_unicode(&keyboard, key));
                let action = Action::RecordOut {
                    in_keyboard,
                    in_key,
                    out_keyboard: keyboard,
                    out_key: key,
                };
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
    let global = unsafe { GLOBAL.assume_init_mut() };

    if n_code as u32 == HC_ACTION
        && (w_param == WPARAM(WM_KEYDOWN as usize)
            || w_param == WPARAM(WM_SYSKEYDOWN as usize)
            || w_param == WPARAM(WM_KEYUP as usize)
            || w_param == WPARAM(WM_SYSKEYUP as usize))
    {
        let kb_struct = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
        match global.handle_key(kb_struct) {
            Status::Intercept => return LRESULT(1),
            Status::Allow => {}
        }
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

fn main() {
    let handle = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), None, 0).unwrap()
    };

    unsafe {
        GLOBAL.write(Global {
            handle,
            action: Action::Normal,
            dirty: false,

            keyboard: Box::new([0; 256]),

            disable: HashSet::new(),
            mappings: HashMap::new(),
        })
    };

    gpui::App::new().run(|cx| {
        cx.open_window(gpui::WindowOptions::default(), |cx| {
            let global = cx.new_model(|cx| {
                cx.notify();
                GlobalModel {}
            });
            let ui = cx.new_view(|_cx| UI {
                _global: global.clone(),
            });

            cx.subscribe(&global, |global, _event: &GlobalModelCheck, cx| {
                let context = unsafe { GLOBAL.assume_init_mut() };
                if context.dirty {
                    context.dirty = false;
                    cx.update_model(&global, |_, cx| cx.emit(GlobalModelChanged));
                }

                cx.on_next_frame(move |cx| {
                    cx.update_model(&global, |_, cx| cx.emit(GlobalModelCheck));
                });
            })
            .detach();

            {
                let global = global.clone();
                cx.on_next_frame(move |cx| {
                    cx.update_model(&global, |_, cx| cx.emit(GlobalModelCheck));
                });
            }

            {
                let ui = ui.clone();
                cx.subscribe(&global, move |_global, _event: &GlobalModelChanged, cx| {
                    cx.update_view(&ui, |_, cx| cx.notify());
                })
                .detach();
            }

            ui
        })
        .unwrap();
        cx.activate(true);
    });

    unsafe { UnhookWindowsHookEx(GLOBAL.assume_init_ref().handle) }.unwrap();
}

struct UI {
    _global: Model<GlobalModel>,
}

struct GlobalModel {}

struct GlobalModelCheck;
impl EventEmitter<GlobalModelCheck> for GlobalModel {}

struct GlobalModelChanged;
impl EventEmitter<GlobalModelChanged> for GlobalModel {}

impl gpui::Render for UI {
    fn render(&mut self, cx: &mut gpui::ViewContext<Self>) -> impl gpui::IntoElement {
        //println!("render");
        div()
            .text_color(white())
            .child("UI")
            .child(
                div()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_ui, _event, _cx| {
                            unsafe { GLOBAL.assume_init_mut() }.start_disable()
                        }),
                    )
                    .child("Disable"),
            )
            .child(
                div()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_ui, _event, _cx| {
                            unsafe { GLOBAL.assume_init_mut() }.start_recording_input()
                        }),
                    )
                    .child("Input"),
            )
            .child(
                div()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_ui, _event, _cx| {
                            unsafe { GLOBAL.assume_init_mut() }.start_recording_output()
                        }),
                    )
                    .child("Output"),
            )
            .child(
                div()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_ui, _event, _cx| {
                            unsafe { GLOBAL.assume_init_mut() }.end_recording()
                        }),
                    )
                    .child("Save"),
            )
    }
}

fn virtual_key_to_unicode(keyboard: &[u8; 256], vk_code: VIRTUAL_KEY) -> char {
    let mut unicode_buffer = [0u16; 2];

    unsafe {
        ToUnicodeEx(
            vk_code.0 as u32,
            MapVirtualKeyW(vk_code.0 as u32, MAPVK_VK_TO_VSC),
            keyboard,
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
