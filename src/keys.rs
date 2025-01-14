use std::ops::Not;

use gpui::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

#[derive(Debug, Clone, IntoElement)]
pub struct Stroke {
    keyboard: Box<[u8; 256]>,
    key: VIRTUAL_KEY,
}

pub const SET_BIT: u8 = 0x80;

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
                    .filter(|(_, v)| v & SET_BIT != 0)
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StrokeData {
    key: u16,
    keyboard: Vec<u8>,
}

impl From<StrokeData> for Stroke {
    fn from(stroke_data: StrokeData) -> Self {
        let mut keyboard = Box::new([0; 256]);
        for idx in stroke_data.keyboard {
            keyboard[idx as usize] = SET_BIT;
        }
        Self {
            key: VIRTUAL_KEY(stroke_data.key),
            keyboard,
        }
    }
}

impl From<Stroke> for StrokeData {
    fn from(stroke: Stroke) -> Self {
        Self {
            key: stroke.key.0,
            keyboard: stroke
                .keyboard
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(_, v)| (v & SET_BIT != 0).then_some(v as u8))
                .collect(),
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Input,
    Output,
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

    pub fn clear(&mut self, side: Side) {
        match side {
            Side::Input => self.input = None,
            Side::Output => self.output = None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.input.is_none() && self.output.is_none()
    }

    pub fn status(&self, keyboard: &[u8; 256], key: VIRTUAL_KEY) -> Option<Option<Stroke>> {
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
            .all(|(target, current)| current & SET_BIT != 0 || target & SET_BIT == 0);

        if valid.not() {
            return None;
        }

        Some(self.output.clone())
    }

    pub fn update(&mut self, side: Side, keyboard: Box<[u8; 256]>, key: VIRTUAL_KEY) {
        let target = match side {
            Side::Input => &mut self.input,
            Side::Output => &mut self.output,
        };
        *target = Some(Stroke::new(keyboard, key));
    }

    pub fn get(&self, side: Side) -> Option<&Stroke> {
        match side {
            Side::Input => self.input.as_ref(),
            Side::Output => self.output.as_ref(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MappingData {
    input: Option<StrokeData>,
    output: Option<StrokeData>,
}

impl From<MappingData> for Mapping {
    fn from(mapping_data: MappingData) -> Self {
        Self {
            input: mapping_data.input.map(Into::into),
            output: mapping_data.output.map(Into::into),
        }
    }
}

impl From<Mapping> for MappingData {
    fn from(mapping: Mapping) -> Self {
        Self {
            input: mapping.input.map(Into::into),
            output: mapping.output.map(Into::into),
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
    Replace(Vec<INPUT>),
}
