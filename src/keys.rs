use std::ops::Not;

use gpui::*;
use prelude::FluentBuilder;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

use crate::{ui::Interactivity, vk_table::VirtualKeyExtension};

#[derive(Debug, Clone)]
pub struct Stroke {
    keyboard: Box<[u8; 256]>,
    key: VIRTUAL_KEY,
}

pub const SET_BIT: u8 = 0x80;

impl Stroke {
    pub fn render(&self, interactivity: Interactivity) -> impl IntoElement {
        let mut modifier = false;
        div()
            .w_full()
            .h_full()
            .flex()
            .gap_2()
            .justify_center()
            .items_center()
            .text_color(interactivity.foreground())
            .children(
                self.keyboard
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(_, v)| v & SET_BIT != 0)
                    .map(|(idx, _)| VIRTUAL_KEY(idx as u16).name().into())
                    .map(|text| render_key(text, interactivity))
                    .map(|v| {
                        modifier = true;
                        v
                    }),
            )
            .when(modifier, |div| {
                div.child(
                    svg()
                        .path("plus.svg")
                        .min_w_6()
                        .min_h_6()
                        .text_color(interactivity.foreground()),
                )
            })
            .child(render_key(self.key.name().into(), interactivity))
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

fn render_key(text: String, interactivity: Interactivity) -> impl IntoElement {
    div()
        .px_3()
        .py_1()
        .border_1()
        .rounded(px(10.0))
        .bg(interactivity.background())
        .border_color(interactivity.foreground())
        .child(text)
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
