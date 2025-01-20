use std::{
    ops::Not,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use gpui::*;
use windows::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

use crate::keys::{KeyState, Mapping, MappingData, Side, Status, Stroke, SET_BIT};

#[derive(Debug)]
pub struct Global {
    selected: Option<(usize, Side)>,
    dirty: bool,
    keyboard: Vec<VIRTUAL_KEY>,

    mappings: Vec<Mapping>,
    path: PathBuf,
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
            Status::Replace(inputs) => Some(inputs),
            Status::Allow => None,
        };
        drop(global);
        if let Some(inputs) = stroke {
            unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
            return LRESULT(1);
        }
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

impl Global {
    fn new() -> Self {
        Self {
            selected: None,
            mappings: vec![Mapping::new_empty()],
            dirty: true,
            keyboard: Vec::new(),

            path: PathBuf::new(),
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

    pub fn select(idx: usize, side: Side) {
        let mut global = GLOBAL.lock().unwrap();
        global.selected = Some((idx, side));
        global.mappings[idx].clear(side);
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

    pub fn current_path() -> PathBuf {
        let global = GLOBAL.lock().unwrap();
        global.path.clone()
    }

    pub fn import(path: PathBuf) {
        let file = std::fs::File::open(&path).unwrap();
        let data = serde_json::from_reader::<_, Vec<MappingData>>(file).unwrap();
        let data = data.into_iter().map(Into::into).collect();

        let mut global = GLOBAL.lock().unwrap();
        global.mappings = data;
        global.path = path;
        global.dirty = true;
    }

    pub fn export(path: PathBuf) {
        let mut global = GLOBAL.lock().unwrap();
        let data = global
            .mappings
            .clone()
            .into_iter()
            .map(Into::into)
            .collect();
        global.path = path.clone();
        drop(global);

        let file = std::fs::File::create(path).unwrap();
        serde_json::to_writer::<_, Vec<MappingData>>(file, &data).unwrap();
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

    pub fn state() -> (Vec<Mapping>, (usize, Side)) {
        let global = GLOBAL.lock().unwrap();
        let items = global.mappings.clone();
        let selected = match global.selected {
            Some((idx, input)) => (idx, input),
            None => (usize::MAX, Side::Input),
        };
        (items, selected)
    }

    pub fn mapping_selected() -> bool {
        let global = GLOBAL.lock().unwrap();
        global.selected.is_some()
    }

    fn mapped_key(&self, key: VIRTUAL_KEY) -> Option<Option<Stroke>> {
        self.mappings
            .iter()
            .find_map(|mapping| mapping.status(&self.keyboard, key))
    }

    fn handle_key(&mut self, kb_struct: &KBDLLHOOKSTRUCT) -> Status {
        let key = VIRTUAL_KEY(kb_struct.vkCode as u16);

        if kb_struct.flags.contains(LLKHF_INJECTED) {
            return Status::Allow;
        }

        let state = match kb_struct.flags.contains(LLKHF_UP) {
            false => KeyState::Pressed,
            true => KeyState::Released,
        };

        if state.released() {
            self.release_key(key);
        }

        let status = match self.selected {
            Some((index, input)) if state.pressed() => {
                self.mappings[index].update(input, self.keyboard.clone(), key);
                self.dirty = true;
                Status::Intercept
            }
            Some(_) => Status::Intercept,
            None => match self.mapped_key(key) {
                None => Status::Allow,
                Some(None) => Status::Intercept,
                Some(Some(stroke)) => Status::Replace(self.create_inputs(&stroke, state)),
            },
        };

        if state.pressed() {
            self.press_key(key);
        }
        status
    }

    fn create_inputs(&self, stroke: &Stroke, state: KeyState) -> Vec<INPUT> {
        let mut input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let mut inputs = Vec::new();

        // release
        for key in self
            .keyboard
            .iter()
            .copied()
            .filter(|key| stroke.keyboard().contains(key).not())
        {
            match key {
                VK_SHIFT => {
                    input.Anonymous.ki.wVk = VK_LSHIFT;
                    inputs.push(input);
                    input.Anonymous.ki.wVk = VK_RSHIFT;
                    inputs.push(input);
                }
                VK_CONTROL => {
                    input.Anonymous.ki.wVk = VK_LCONTROL;
                    inputs.push(input);
                    input.Anonymous.ki.wVk = VK_RCONTROL;
                    inputs.push(input);
                }
                VK_MENU => {
                    input.Anonymous.ki.wVk = VK_LMENU;
                    inputs.push(input);
                    input.Anonymous.ki.wVk = VK_RMENU;
                    inputs.push(input);
                }
                _ => {
                    input.Anonymous.ki.wVk = key;
                    inputs.push(input);
                }
            }
        }

        // press
        input.Anonymous.ki.dwFlags = KEYBD_EVENT_FLAGS(0);
        for key in stroke
            .keyboard()
            .iter()
            .copied()
            .filter(|key| self.keyboard.contains(key).not())
        {
            input.Anonymous.ki.wVk = key;
            inputs.push(input);
        }

        input.Anonymous.ki.dwFlags = match state {
            KeyState::Pressed => KEYBD_EVENT_FLAGS(0),
            KeyState::Released => KEYEVENTF_KEYUP,
        };
        input.Anonymous.ki.wVk = stroke.key();
        inputs.push(input);

        for idx in (0..(inputs.len() - 1)).rev() {
            let mut input = inputs[idx];
            unsafe {
                input.Anonymous.ki.dwFlags.0 ^= KEYEVENTF_KEYUP.0;
            };
            inputs.push(input);
        }

        inputs
    }

    fn press_key(&mut self, key: VIRTUAL_KEY) {
        match key {
            VK_LSHIFT if self.has_key(VK_RSHIFT) => self.replace_key(VK_RSHIFT, VK_SHIFT),
            VK_RSHIFT if self.has_key(VK_LSHIFT) => self.replace_key(VK_LSHIFT, VK_SHIFT),
            VK_LSHIFT | VK_RSHIFT if self.has_key(VK_SHIFT) => {}

            VK_LCONTROL if self.has_key(VK_RCONTROL) => self.replace_key(VK_RCONTROL, VK_CONTROL),
            VK_RCONTROL if self.has_key(VK_LCONTROL) => self.replace_key(VK_LCONTROL, VK_CONTROL),
            VK_LCONTROL | VK_RCONTROL if self.has_key(VK_CONTROL) => {}

            VK_LMENU if self.has_key(VK_RMENU) => self.replace_key(VK_RMENU, VK_MENU),
            VK_RMENU if self.has_key(VK_LMENU) => self.replace_key(VK_LMENU, VK_MENU),
            VK_LMENU | VK_RMENU if self.has_key(VK_MENU) => {}

            _ if self.keyboard.contains(&key).not() => self.keyboard.push(key),
            _ => {}
        }
    }

    fn has_key(&self, key: VIRTUAL_KEY) -> bool {
        self.keyboard.contains(&key)
    }

    fn replace_key(&mut self, old: VIRTUAL_KEY, new: VIRTUAL_KEY) {
        let Some(v) = self.keyboard.iter_mut().find(|key| **key == old) else {
            return;
        };
        *v = new;
    }

    fn release_key(&mut self, key: VIRTUAL_KEY) {
        match key {
            VK_LSHIFT if self.has_key(VK_SHIFT) => self.replace_key(VK_SHIFT, VK_RSHIFT),
            VK_RSHIFT if self.has_key(VK_SHIFT) => self.replace_key(VK_SHIFT, VK_LSHIFT),

            VK_LCONTROL if self.has_key(VK_CONTROL) => self.replace_key(VK_CONTROL, VK_RCONTROL),
            VK_RCONTROL if self.has_key(VK_CONTROL) => self.replace_key(VK_CONTROL, VK_LCONTROL),

            VK_LMENU if self.has_key(VK_MENU) => self.replace_key(VK_CONTROL, VK_RMENU),
            VK_RMENU if self.has_key(VK_MENU) => self.replace_key(VK_CONTROL, VK_LMENU),
            _ => {
                self.keyboard.retain_mut(|v| *v != key);
            }
        }
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
    pub side: Side,
}

impl EventEmitter<GlobalSelect> for GlobalChecker {}
