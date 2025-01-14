use std::sync::{LazyLock, Mutex};

use gpui::*;
use windows::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

use crate::keys::{KeyState, Mapping, Status, Stroke};

#[derive(Debug)]
pub struct Global {
    selected: Option<(usize, bool)>,
    dirty: bool,
    keyboard: Box<[u8; 256]>,

    mappings: Vec<Mapping>,
}

static GLOBAL: LazyLock<Mutex<Global>> = LazyLock::new(|| Mutex::new(Global::new()));

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
            send_key(&stroke, state);
            return LRESULT(1);
        }
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

fn send_key(stroke: &Stroke, state: KeyState) {
    let mut keyboard = Box::new([0u8; 256]);
    unsafe { GetKeyboardState(&mut keyboard) }.unwrap();

    let mut input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let mut inputs = Vec::new();
    for (idx, value) in stroke.keyboard().iter().copied().enumerate() {
        match VIRTUAL_KEY(idx as u16) {
            VK_LSHIFT | VK_RSHIFT => continue,
            VK_LCONTROL | VK_RCONTROL => continue,
            VK_LMENU | VK_RMENU => continue,
            _ => {}
        }
        if value & 0x80 != keyboard[idx] & 0x80 {
            input.Anonymous.ki.dwFlags = if value & 0x80 != 0 {
                KEYBD_EVENT_FLAGS(0)
            } else {
                KEYEVENTF_KEYUP
            };
            input.Anonymous.ki.wVk = VIRTUAL_KEY(idx as u16);
            inputs.push(input);
        }
    }

    input.Anonymous.ki.dwFlags = match state {
        KeyState::Pressed => KEYBD_EVENT_FLAGS(0),
        KeyState::Released => KEYEVENTF_KEYUP,
    };
    input.Anonymous.ki.wVk = stroke.key();
    inputs.push(input);

    for idx in (0..(inputs.len() / 2)).rev() {
        let mut input = inputs[idx];
        unsafe {
            input.Anonymous.ki.dwFlags.0 ^= KEYEVENTF_KEYUP.0;
        };
        inputs.push(input);
    }

    unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
}

impl Global {
    fn new() -> Self {
        Self {
            selected: None,
            mappings: vec![Mapping::new_empty()],
            dirty: true,
            keyboard: Box::new([0; 256]),
        }
    }

    pub fn install_hook() -> HHOOK {
        unsafe {
            SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), None, 0).unwrap()
        }
    }

    pub fn delete_hook(hook: HHOOK) {
        unsafe { UnhookWindowsHookEx(hook) }.unwrap();
    }

    pub fn select(idx: usize, input: bool) {
        let mut global = GLOBAL.lock().unwrap();
        global.selected = Some((idx, input));
        global.mappings[idx].clear(input);
        global.dirty = true;
    }

    pub fn exit_edit() {
        let mut global = GLOBAL.lock().unwrap();
        global.selected = None;
        global.dirty = true;
        global.maybe_add_empty();
    }

    pub fn delete(idx: usize) {
        let mut global = GLOBAL.lock().unwrap();
        if global.selected.is_some() {
            return;
        }
        if idx >= global.mappings.len() {
            println!("Remove out of bounds of mappings, how?");
            return;
        }
        global.mappings.remove(idx);
        global.dirty = true;
        global.maybe_add_empty();
    }

    fn maybe_add_empty(&mut self) {
        let ok = match self.mappings.last() {
            None => false,
            Some(mapping) => mapping.is_empty(),
        };
        if ok {
            return;
        }
        self.mappings.push(Mapping::new_empty());
    }

    pub fn changed() -> bool {
        let mut global = GLOBAL.lock().unwrap();
        std::mem::take(&mut global.dirty)
    }

    pub fn state() -> (Vec<Mapping>, (usize, bool)) {
        let global = GLOBAL.lock().unwrap();
        let items = global.mappings.clone();
        let selected = match global.selected {
            Some((idx, input)) => (idx, input),
            None => (usize::MAX, true),
        };
        (items, selected)
    }

    fn mapped_key(&self, key: VIRTUAL_KEY, state: KeyState) -> Status {
        self.mappings
            .iter()
            .find_map(|mapping| mapping.status(&self.keyboard, key, state))
            .unwrap_or(Status::Allow)
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

        let status = match self.selected {
            Some((index, input)) if state.pressed() => {
                self.mappings[index].update(input, self.keyboard.clone(), key);
                self.dirty = true;
                Status::Intercept
            }
            Some(_) => Status::Intercept,
            None => self.mapped_key(key, state),
        };

        if state.pressed() {
            self.set_key(key, state);
        }
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

pub struct GlobalChecker {}

pub struct GlobalCheck;
impl EventEmitter<GlobalCheck> for GlobalChecker {}

pub struct GlobalChanged;
impl EventEmitter<GlobalChanged> for GlobalChecker {}

pub struct GlobalDelete(pub usize);
impl EventEmitter<GlobalDelete> for GlobalChecker {}

pub struct GlobalExitEdit;
impl EventEmitter<GlobalExitEdit> for GlobalChecker {}

pub struct GlobalSelect {
    pub idx: usize,
    pub input: bool,
}

impl EventEmitter<GlobalSelect> for GlobalChecker {}
