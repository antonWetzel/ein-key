use std::{
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
    keyboard: Box<[u8; 256]>,

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
            keyboard: Box::new([0; 256]),

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

    fn mapped_key(&self, key: VIRTUAL_KEY) -> Option<Stroke> {
        self.mappings
            .iter()
            .find_map(|mapping| mapping.status(&self.keyboard, key))
            .unwrap_or(None)
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
            self.set_key(key, state);
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
                Some(stroke) => Status::Replace(self.create_inputs(&stroke, state)),
            },
        };

        if state.pressed() {
            self.set_key(key, state);
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
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        let mut inputs = Vec::new();
        for (idx, value) in stroke.keyboard().iter().copied().enumerate() {
            if value & SET_BIT != self.keyboard[idx] & SET_BIT {
                input.Anonymous.ki.dwFlags = if value & SET_BIT != 0 {
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

        inputs
    }

    fn set_key(&mut self, key: VIRTUAL_KEY, state: KeyState) {
        let value = match state {
            KeyState::Pressed => SET_BIT,
            KeyState::Released => 0,
        };
        self.keyboard[key.0 as usize] = value;
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
