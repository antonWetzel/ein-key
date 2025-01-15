use gpui::*;

pub enum Color {
    Background,
    BackgroundHover,
    BackgroundSelected,
    BackgroundDisabled,

    Foreground,
    ForegroundSelected,
    ForegroundDisabled,
}

const HUE: f32 = 0.6;

impl From<Color> for Hsla {
    fn from(color: Color) -> Self {
        match color {
            Color::Background => hsla(HUE, 0.1, 0.1, 1.0),
            Color::BackgroundHover => hsla(HUE, 0.1, 0.15, 1.0),
            Color::BackgroundSelected => hsla(HUE, 0.1, 0.2, 1.0),
            Color::BackgroundDisabled => Color::Background.into(),

            Color::Foreground => hsla(HUE, 0.1, 0.8, 1.0),
            Color::ForegroundSelected => hsla(HUE, 1.0, 1.0, 1.0),
            Color::ForegroundDisabled => hsla(HUE, 0.1, 0.4, 1.0),
        }
    }
}

impl From<Color> for Fill {
    fn from(color: Color) -> Self {
        Hsla::from(color).into()
    }
}
