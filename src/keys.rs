use std::ops::Not;

use gpui::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

#[derive(Debug, Clone, IntoElement)]
pub struct Stroke {
    keyboard: Box<[u8; 256]>,
    key: VIRTUAL_KEY,
}

impl RenderOnce for Stroke {
    fn render(self, _cx: &mut WindowContext) -> impl IntoElement {
        div()
            .text_color(white())
            .w_full()
            .h_full()
            .flex()
            .gap_2()
            .justify_center()
            .items_center()
            .children(
                self.keyboard
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(_, v)| v & 0x80 != 0)
                    .filter_map(|(idx, _)| Self::to_unicode(VIRTUAL_KEY(idx as u16)))
                    .map(Key),
            )
            .child(Key(Self::to_unicode(self.key).unwrap_or(String::new())))
            .child("| ")
            .child(Key(
                Self::to_unicode_with(self.key, &self.keyboard).unwrap_or(String::new())
            ))
    }
}

impl Stroke {
    pub fn new(keyboard: Box<[u8; 256]>, key: VIRTUAL_KEY) -> Self {
        Self { keyboard, key }
    }

    pub fn key(&self) -> VIRTUAL_KEY {
        self.key
    }

    pub fn keyboard(&self) -> &[u8; 256] {
        &self.keyboard
    }

    fn to_unicode(key: VIRTUAL_KEY) -> Option<String> {
        Self::to_unicode_with(key, &[0; 256])
    }

    fn to_unicode_with(key: VIRTUAL_KEY, keyboard: &[u8; 256]) -> Option<String> {
        match key {
            VK_LSHIFT | VK_RSHIFT => return None,
            VK_LCONTROL | VK_RCONTROL => return None,
            VK_LMENU | VK_RMENU => return None,
            VK_SHIFT => return Some("Shift".into()),
            VK_CONTROL => return Some("Control".into()),
            VK_MENU => return Some("Menu".into()),
            _ => {}
        }

        let mut unicode_buffer = [0u16; 2];

        unsafe {
            ToUnicodeEx(
                key.0 as u32,
                MapVirtualKeyW(key.0 as u32, MAPVK_VK_TO_VSC),
                keyboard,
                &mut unicode_buffer,
                0,
                None,
            )
        };

        String::from_utf16(&unicode_buffer).ok()
    }
}

#[derive(Debug, Clone, IntoElement)]
pub struct Key(String);

impl RenderOnce for Key {
    fn render(self, _cx: &mut WindowContext) -> impl IntoElement {
        div()
            .bg(opaque_grey(0.1, 1.0))
            .px_3()
            .py_1()
            .border_1()
            .border_color(opaque_grey(0.4, 1.0))
            .rounded(px(5.0))
            .child(self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Mapping {
    input: Option<Stroke>,
    output: Option<Stroke>,
}

impl Mapping {
    pub fn new_empty() -> Self {
        Self {
            input: None,
            output: None,
        }
    }

    pub fn clear(&mut self, input: bool) {
        match input {
            true => self.input = None,
            false => self.output = None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.input.is_none() && self.output.is_none()
    }

    pub fn status(
        &self,
        keyboard: &[u8; 256],
        key: VIRTUAL_KEY,
        state: KeyState,
    ) -> Option<Status> {
        let Some(input) = &self.input else {
            return None;
        };

        if input.key != key {
            return None;
        }

        let valid = input
            .keyboard
            .iter()
            .copied()
            .zip(keyboard.iter().copied())
            .all(|(target, current)| current & 0x80 != 0 || target & 0x80 == 0);

        if valid.not() {
            return None;
        }

        let status = match self.output.clone() {
            Some(stroke) => Status::Replace { stroke, state },
            None => Status::Intercept,
        };
        Some(status)
    }

    pub fn update(&mut self, input: bool, keyboard: Box<[u8; 256]>, key: VIRTUAL_KEY) {
        let target = match input {
            true => &mut self.input,
            false => &mut self.output,
        };
        *target = Some(Stroke::new(keyboard, key));
    }

    pub fn get(&self, input: bool) -> Option<&Stroke> {
        match input {
            true => self.input.as_ref(),
            false => self.output.as_ref(),
        }
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
