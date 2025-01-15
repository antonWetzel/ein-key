use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

pub trait VirtualKeyExtension {
    fn name(self) -> &'static str;
}

// https://gist.github.com/kkusch/245bb80ec4e7ab4d8cdc6b7eeb3f330f
// + regex
// + case conversion by ai
impl VirtualKeyExtension for VIRTUAL_KEY {
    fn name(self) -> &'static str {
        match self.0 {
            0x01 => "LButton",
            0x02 => "RButton",
            0x03 => "Cancel",
            0x04 => "MButton",
            0x05 => "XButton1",
            0x06 => "XButton2",
            0x08 => "Back",
            0x09 => "Tab",
            0x0C => "Clear",
            0x0D => "Return",
            0x10 => "Shift",
            0x11 => "Control",
            0x12 => "Menu",
            0x13 => "Pause",
            0x14 => "Capital",
            0x15 => "Kana",
            0x17 => "Junja",
            0x18 => "Final",
            0x19 => "Hanja",
            0x1B => "Escape",
            0x1C => "Convert",
            0x1D => "NonConvert",
            0x1E => "Accept",
            0x1F => "ModeChange",
            0x20 => "Space",
            0x21 => "Prior",
            0x22 => "Next",
            0x23 => "End",
            0x24 => "Home",
            0x25 => "Left",
            0x26 => "Up",
            0x27 => "Right",
            0x28 => "Down",
            0x29 => "Select",
            0x2A => "Print",
            0x2B => "Execute",
            0x2C => "Snapshot",
            0x2D => "Insert",
            0x2E => "Delete",
            0x2F => "Help",
            0x30 => "0",
            0x31 => "1",
            0x32 => "2",
            0x33 => "3",
            0x34 => "4",
            0x35 => "5",
            0x36 => "6",
            0x37 => "7",
            0x38 => "8",
            0x39 => "9",
            0x41 => "A",
            0x42 => "B",
            0x43 => "C",
            0x44 => "D",
            0x45 => "E",
            0x46 => "F",
            0x47 => "G",
            0x48 => "H",
            0x49 => "I",
            0x4A => "J",
            0x4B => "K",
            0x4C => "L",
            0x4D => "M",
            0x4E => "N",
            0x4F => "O",
            0x50 => "P",
            0x51 => "Q",
            0x52 => "R",
            0x53 => "S",
            0x54 => "T",
            0x55 => "U",
            0x56 => "V",
            0x57 => "W",
            0x58 => "X",
            0x59 => "Y",
            0x5A => "Z",
            0x5B => "LWin",
            0x5C => "RWin",
            0x5D => "Apps",
            0x5F => "Sleep",
            0x60 => "Numpad0",
            0x61 => "Numpad1",
            0x62 => "Numpad2",
            0x63 => "Numpad3",
            0x64 => "Numpad4",
            0x65 => "Numpad5",
            0x66 => "Numpad6",
            0x67 => "Numpad7",
            0x68 => "Numpad8",
            0x69 => "Numpad9",
            0x6A => "Multiply",
            0x6B => "Add",
            0x6C => "Separator",
            0x6D => "Subtract",
            0x6E => "Decimal",
            0x6F => "Divide",
            0x70 => "F1",
            0x71 => "F2",
            0x72 => "F3",
            0x73 => "F4",
            0x74 => "F5",
            0x75 => "F6",
            0x76 => "F7",
            0x77 => "F8",
            0x78 => "F9",
            0x79 => "F10",
            0x7A => "F11",
            0x7B => "F12",
            0x7C => "F13",
            0x7D => "F14",
            0x7E => "F15",
            0x7F => "F16",
            0x80 => "F17",
            0x81 => "F18",
            0x82 => "F19",
            0x83 => "F20",
            0x84 => "F21",
            0x85 => "F22",
            0x86 => "F23",
            0x87 => "F24",
            0x90 => "NumLock",
            0x91 => "Scroll",
            0xA0 => "LShift",
            0xA1 => "RShift",
            0xA2 => "LControl",
            0xA3 => "RControl",
            0xA4 => "LMenu",
            0xA5 => "RMenu",
            0xA6 => "BrowserBack",
            0xA7 => "BrowserForward",
            0xA8 => "BrowserRefresh",
            0xA9 => "BrowserStop",
            0xAA => "BrowserSearch",
            0xAB => "BrowserFavorites",
            0xAC => "BrowserHome",
            0xAD => "VolumeMute",
            0xAE => "VolumeDown",
            0xAF => "VolumeUp",
            0xB0 => "MediaNextTrack",
            0xB1 => "MediaPrevTrack",
            0xB2 => "MediaStop",
            0xB3 => "MediaPlayPause",
            0xB4 => "LaunchMail",
            0xB5 => "LaunchMediaSelect",
            0xB6 => "LaunchApp1",
            0xB7 => "LaunchApp2",
            0xBA => "Oem1",
            0xBB => "OemPlus",
            0xBC => "OemComma",
            0xBD => "OemMinus",
            0xBE => "OemPeriod",
            0xBF => "Oem2",
            0xC0 => "Oem3",
            0xDB => "Oem4",
            0xDC => "Oem5",
            0xDD => "Oem6",
            0xDE => "Oem7",
            0xDF => "Oem8",
            0xE2 => "Oem102",
            0xE5 => "ProcessKey",
            0xE7 => "Packet",
            0xF6 => "Attn",
            0xF7 => "CrSel",
            0xF8 => "ExSel",
            0xF9 => "ErEOF",
            0xFA => "Play",
            0xFB => "Zoom",
            0xFC => "NoName",
            0xFD => "Pa1",
            0xFE => "OemClear",
            _ => "...",
        }
    }
}
