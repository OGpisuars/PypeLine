//! UI themes: window colors, syntax colors and the mouse cursor.
//!
//! The pixel-art world keeps its own palette; a theme only restyles the egui
//! windows drawn over it. Every color here is a whole GBA color (channels
//! are multiples of 8), like the rest of the game.

use bevy_egui::egui::{self, Color32, CornerRadius, Shadow, Stroke};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeId {
    /// FireRed-style cream dialogue boxes (the original look).
    #[default]
    Classic,
    IndigoDusk,
    Ember,
    Glasswork,
    MintCart,
    Midnight,
}

impl ThemeId {
    pub const ALL: [Self; 6] = [
        Self::Classic,
        Self::IndigoDusk,
        Self::Ember,
        Self::Glasswork,
        Self::MintCart,
        Self::Midnight,
    ];

    pub fn theme(self) -> &'static Theme {
        match self {
            Self::Classic => &CLASSIC,
            Self::IndigoDusk => &INDIGO_DUSK,
            Self::Ember => &EMBER,
            Self::Glasswork => &GLASSWORK,
            Self::MintCart => &MINT_CART,
            Self::Midnight => &MIDNIGHT,
        }
    }
}

/// Colors for Python code in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Syntax {
    pub plain: [u8; 3],
    pub keyword: [u8; 3],
    pub builtin: [u8; 3],
    pub string: [u8; 3],
    pub number: [u8; 3],
    pub comment: [u8; 3],
    /// Background of the line with an error.
    pub error_line: [u8; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub dark: bool,
    /// Window and panel background.
    pub window: [u8; 3],
    /// Text boxes and code blocks.
    pub field: [u8; 3],
    /// Buttons at rest.
    pub button: [u8; 3],
    pub ink: [u8; 3],
    pub border: [u8; 3],
    /// Selection and hovered buttons.
    pub accent: [u8; 3],
    pub error: [u8; 3],
    pub info: [u8; 3],
    pub syntax: Syntax,
    /// The mouse pointer and the text caret: dark on light themes, light on
    /// dark ones, so it is always easy to find.
    pub cursor_fill: [u8; 3],
    pub cursor_outline: [u8; 3],
}

pub fn rgb([r, g, b]: [u8; 3]) -> Color32 {
    Color32::from_rgb(r, g, b)
}

impl Theme {
    pub fn visuals(&self) -> egui::Visuals {
        let mut v = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        let ink = rgb(self.ink);
        let border = rgb(self.border);
        let button = rgb(self.button);
        let accent = rgb(self.accent);
        v.window_fill = rgb(self.window);
        v.panel_fill = rgb(self.window);
        v.faint_bg_color = rgb(self.field);
        v.extreme_bg_color = rgb(self.field);
        v.code_bg_color = rgb(self.field);
        v.override_text_color = Some(ink);
        v.hyperlink_color = rgb(self.info);
        v.error_fg_color = rgb(self.error);
        v.window_stroke = Stroke::new(3.0, border);
        v.window_corner_radius = CornerRadius::same(8);
        v.window_shadow = Shadow::NONE;
        v.popup_shadow = Shadow::NONE;
        v.selection.bg_fill = accent.gamma_multiply(0.55);
        v.selection.stroke = Stroke::new(1.0, ink);
        v.text_cursor.stroke = Stroke::new(2.0, rgb(self.cursor_fill));
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = rgb(self.window);
        w.noninteractive.bg_stroke = Stroke::new(1.0, border.gamma_multiply(0.6));
        w.noninteractive.fg_stroke = Stroke::new(1.0, ink);
        for (state, fill) in [
            (&mut w.inactive, button),
            (&mut w.hovered, accent.gamma_multiply(0.7)),
            (&mut w.active, accent),
            (&mut w.open, accent.gamma_multiply(0.7)),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.fg_stroke = Stroke::new(1.0, ink);
        }
        w.inactive.bg_stroke = Stroke::new(1.0, border.gamma_multiply(0.5));
        w.hovered.bg_stroke = Stroke::new(1.0, border);
        w.active.bg_stroke = Stroke::new(1.0, border);
        v
    }
}

pub const CLASSIC: Theme = Theme {
    name: "Classic cream",
    dark: false,
    window: [248, 240, 216],
    field: [248, 248, 240],
    button: [232, 224, 200],
    ink: [40, 32, 48],
    border: [72, 96, 152],
    accent: [152, 184, 232],
    error: [200, 48, 48],
    info: [72, 96, 152],
    syntax: Syntax {
        plain: [40, 32, 48],
        keyword: [152, 48, 120],
        builtin: [40, 88, 168],
        string: [48, 120, 40],
        number: [184, 96, 24],
        comment: [128, 120, 104],
        error_line: [248, 208, 200],
    },
    cursor_fill: [24, 24, 32],
    cursor_outline: [248, 248, 248],
};

pub const INDIGO_DUSK: Theme = Theme {
    name: "Indigo dusk",
    dark: true,
    window: [32, 32, 56],
    field: [24, 24, 40],
    button: [48, 48, 80],
    ink: [232, 224, 248],
    border: [120, 104, 216],
    accent: [104, 88, 200],
    error: [248, 104, 112],
    info: [136, 176, 248],
    syntax: Syntax {
        plain: [232, 224, 248],
        keyword: [216, 136, 248],
        builtin: [128, 192, 248],
        string: [160, 224, 136],
        number: [248, 184, 104],
        comment: [136, 128, 168],
        error_line: [96, 32, 48],
    },
    cursor_fill: [248, 248, 248],
    cursor_outline: [16, 16, 24],
};

pub const EMBER: Theme = Theme {
    name: "Ember",
    dark: true,
    window: [40, 24, 24],
    field: [32, 16, 16],
    button: [64, 40, 32],
    ink: [248, 224, 200],
    border: [224, 120, 56],
    accent: [184, 88, 40],
    error: [248, 96, 80],
    info: [248, 184, 88],
    syntax: Syntax {
        plain: [248, 224, 200],
        keyword: [248, 136, 88],
        builtin: [248, 200, 104],
        string: [184, 216, 120],
        number: [240, 152, 176],
        comment: [160, 128, 112],
        error_line: [104, 32, 24],
    },
    cursor_fill: [248, 232, 208],
    cursor_outline: [24, 8, 8],
};

pub const GLASSWORK: Theme = Theme {
    name: "Glasswork",
    dark: false,
    window: [232, 240, 248],
    field: [248, 248, 248],
    button: [208, 224, 240],
    ink: [32, 48, 64],
    border: [72, 144, 200],
    accent: [136, 192, 232],
    error: [200, 56, 72],
    info: [40, 112, 176],
    syntax: Syntax {
        plain: [32, 48, 64],
        keyword: [120, 56, 176],
        builtin: [24, 104, 176],
        string: [32, 128, 96],
        number: [192, 104, 32],
        comment: [120, 136, 152],
        error_line: [248, 216, 224],
    },
    cursor_fill: [16, 32, 48],
    cursor_outline: [248, 248, 248],
};

pub const MINT_CART: Theme = Theme {
    name: "Mint cart",
    dark: false,
    window: [224, 240, 232],
    field: [240, 248, 240],
    button: [200, 232, 216],
    ink: [24, 56, 40],
    border: [56, 144, 104],
    accent: [136, 208, 168],
    error: [192, 56, 56],
    info: [40, 120, 88],
    syntax: Syntax {
        plain: [24, 56, 40],
        keyword: [152, 56, 112],
        builtin: [32, 96, 152],
        string: [40, 120, 40],
        number: [184, 104, 24],
        comment: [112, 136, 120],
        error_line: [248, 216, 208],
    },
    cursor_fill: [16, 40, 24],
    cursor_outline: [240, 248, 240],
};

pub const MIDNIGHT: Theme = Theme {
    name: "Midnight (high contrast)",
    dark: true,
    window: [8, 8, 8],
    field: [0, 0, 0],
    button: [32, 32, 32],
    ink: [248, 248, 248],
    border: [248, 248, 248],
    accent: [88, 88, 88],
    error: [248, 88, 88],
    info: [120, 200, 248],
    syntax: Syntax {
        plain: [248, 248, 248],
        keyword: [248, 200, 64],
        builtin: [96, 216, 248],
        string: [128, 248, 128],
        number: [248, 152, 216],
        comment: [168, 168, 168],
        error_line: [112, 16, 16],
    },
    cursor_fill: [248, 248, 248],
    cursor_outline: [0, 0, 0],
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Light themes get a dark cursor and dark themes a light one.
    #[test]
    fn cursor_stands_out() {
        let brightness = |[r, g, b]: [u8; 3]| u32::from(r) + u32::from(g) + u32::from(b);
        for id in ThemeId::ALL {
            let theme = id.theme();
            let (fill, window) = (brightness(theme.cursor_fill), brightness(theme.window));
            if theme.dark {
                assert!(fill > window + 300, "{}", theme.name);
            } else {
                assert!(fill + 300 < window, "{}", theme.name);
            }
        }
    }

    #[test]
    fn colors_are_gba_colors() {
        for id in ThemeId::ALL {
            let t = id.theme();
            let s = t.syntax;
            let all = [
                t.window,
                t.field,
                t.button,
                t.ink,
                t.border,
                t.accent,
                t.error,
                t.info,
                s.plain,
                s.keyword,
                s.builtin,
                s.string,
                s.number,
                s.comment,
                s.error_line,
            ];
            for color in all {
                assert!(color.iter().all(|c| c % 8 == 0), "{} {color:?}", t.name);
            }
        }
    }
}
