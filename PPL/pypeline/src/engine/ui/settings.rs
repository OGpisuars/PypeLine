//! Settings window: theme, font, text size and mouse cursor.
//!
//! Saved as `settings.ron` in the player's data folder, next to the scripts
//! and saves, and applied the moment they change.

use std::path::PathBuf;

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::{CursorIcon, CustomCursor, CustomCursorImage, PrimaryWindow, SystemCursorIcon},
};
use bevy_egui::{EguiContexts, egui};
use serde::{Deserialize, Serialize};

use crate::engine::themes::{Theme, ThemeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FontChoice {
    #[default]
    JetBrainsMono,
    MapleMono,
    NotoSansMono,
    Hack,
}

impl FontChoice {
    pub const ALL: [Self; 4] = [
        Self::JetBrainsMono,
        Self::MapleMono,
        Self::NotoSansMono,
        Self::Hack,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::JetBrainsMono => "JetBrains Mono",
            Self::MapleMono => "Maple Mono (rounded)",
            Self::NotoSansMono => "Noto Sans Mono",
            Self::Hack => "Hack",
        }
    }

    /// The font file, or None for fonts egui already has.
    fn data(self) -> Option<&'static [u8]> {
        match self {
            Self::JetBrainsMono => Some(include_bytes!(
                "../../../assets/fonts/JetBrainsMonoNerdFontMono-Regular.ttf"
            )),
            Self::MapleMono => Some(include_bytes!(
                "../../../assets/fonts/MapleMono-Regular.ttf"
            )),
            Self::NotoSansMono => Some(include_bytes!(
                "../../../assets/fonts/NotoSansMono-Regular.ttf"
            )),
            Self::Hack => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CursorChoice {
    /// Dark arrow on light themes, light arrow on dark themes.
    #[default]
    MatchTheme,
    Dark,
    Light,
    /// The operating system's own pointer.
    System,
}

impl CursorChoice {
    pub const ALL: [Self; 4] = [Self::MatchTheme, Self::Dark, Self::Light, Self::System];

    pub fn name(self) -> &'static str {
        match self {
            Self::MatchTheme => "Match the theme",
            Self::Dark => "Black on white",
            Self::Light => "White on black",
            Self::System => "System pointer",
        }
    }

    /// (fill, outline) of the pixel arrow, or None for the system pointer.
    fn colors(self, theme: &Theme) -> Option<([u8; 3], [u8; 3])> {
        match self {
            Self::MatchTheme => Some((theme.cursor_fill, theme.cursor_outline)),
            Self::Dark => Some(([16, 16, 16], [248, 248, 248])),
            Self::Light => Some(([248, 248, 248], [16, 16, 16])),
            Self::System => None,
        }
    }
}

pub const MIN_TEXT_SIZE: f32 = 10.0;
pub const MAX_TEXT_SIZE: f32 = 24.0;

#[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeId,
    pub font: FontChoice,
    /// Body and code text size, in points.
    pub text_size: f32,
    pub cursor: CursorChoice,
    /// The island gently floats up and down. Off keeps it perfectly still.
    pub island_bob: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeId::default(),
            font: FontChoice::default(),
            text_size: 13.0,
            cursor: CursorChoice::default(),
            island_bob: true,
        }
    }
}

impl Settings {
    pub fn theme(&self) -> &'static Theme {
        self.theme.theme()
    }

    fn path() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("PypeLine").join("settings.ron"))
    }

    /// The saved settings; defaults if there are none or they are damaged.
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|text| ron::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::path() else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(std::io::Error::other)?;
        crate::scripting::files::write_atomic(&path, &text)
    }
}

#[derive(Resource, Default)]
pub struct SettingsWindow {
    pub open: bool,
}

pub fn load_settings(mut commands: Commands) {
    commands.insert_resource(Settings::load());
}

/// Write the settings to disk whenever they change.
pub fn save_settings(settings: Res<Settings>, mut console: ResMut<crate::scripting::Console>) {
    if settings.is_changed()
        && !settings.is_added()
        && let Err(err) = settings.save()
    {
        console.push(
            crate::scripting::ConsoleKind::Error,
            format!("Could not save settings: {err}"),
        );
    }
}

/// Restyle egui when the theme, font or text size changes.
pub fn apply_settings(
    mut contexts: EguiContexts,
    settings: Res<Settings>,
    mut applied: Local<Option<Settings>>,
) -> Result {
    if applied.as_ref() == Some(&*settings) {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    if applied.as_ref().map(|a| a.font) != Some(settings.font) {
        ctx.set_fonts(fonts(settings.font));
    }
    let visuals = settings.theme().visuals();
    let size = settings.text_size;
    ctx.all_styles_mut(|style| {
        style.visuals = visuals.clone();
        for (text_style, font) in style.text_styles.iter_mut() {
            font.size = match text_style {
                egui::TextStyle::Heading => size * 1.4,
                egui::TextStyle::Small => (size * 0.75).max(MIN_TEXT_SIZE * 0.75),
                _ => size,
            };
        }
    });
    *applied = Some(settings.clone());
    Ok(())
}

fn fonts(choice: FontChoice) -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    // egui's own fonts stay as fallbacks for symbols and emoji.
    let name = match choice.data() {
        Some(data) => {
            fonts.font_data.insert(
                "chosen".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(data)),
            );
            "chosen"
        }
        None => "Hack",
    };
    for family in [egui::FontFamily::Monospace, egui::FontFamily::Proportional] {
        let list = fonts.families.entry(family).or_default();
        list.retain(|f| f != name);
        list.insert(0, name.to_owned());
    }
    fonts
}

pub fn settings_window(
    mut contexts: EguiContexts,
    mut window: ResMut<SettingsWindow>,
    mut settings: ResMut<Settings>,
) -> Result {
    let mut open = window.open;
    let mut edited = settings.clone();
    egui::Window::new("Settings")
        .open(&mut open)
        .resizable(false)
        .default_pos(egui::pos2(360.0, 80.0))
        .show(contexts.ctx_mut()?, |ui| {
            egui::Grid::new("settings_grid")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Theme");
                    egui::ComboBox::from_id_salt("theme")
                        .selected_text(edited.theme().name)
                        .show_ui(ui, |ui| {
                            for id in ThemeId::ALL {
                                ui.selectable_value(&mut edited.theme, id, id.theme().name);
                            }
                        });
                    ui.end_row();

                    ui.label("Font");
                    egui::ComboBox::from_id_salt("font")
                        .selected_text(edited.font.name())
                        .show_ui(ui, |ui| {
                            for font in FontChoice::ALL {
                                ui.selectable_value(&mut edited.font, font, font.name());
                            }
                        });
                    ui.end_row();

                    ui.label("Text size");
                    ui.add(
                        egui::Slider::new(&mut edited.text_size, MIN_TEXT_SIZE..=MAX_TEXT_SIZE)
                            .step_by(1.0),
                    );
                    ui.end_row();

                    ui.label("Mouse cursor");
                    egui::ComboBox::from_id_salt("cursor")
                        .selected_text(edited.cursor.name())
                        .show_ui(ui, |ui| {
                            for cursor in CursorChoice::ALL {
                                ui.selectable_value(&mut edited.cursor, cursor, cursor.name());
                            }
                        });
                    ui.end_row();

                    ui.label("Island");
                    ui.checkbox(&mut edited.island_bob, "Bob up and down");
                    ui.end_row();
                });
            ui.add_space(6.0);
            if ui.button("Back to defaults").clicked() {
                edited = Settings::default();
            }
        });
    window.open = open;
    if edited != *settings {
        *settings = edited;
    }
    Ok(())
}

/// The pixel arrow: `#` is the outline, `.` the fill.
const ARROW: [&str; 17] = [
    "#           ",
    "##          ",
    "#.#         ",
    "#..#        ",
    "#...#       ",
    "#....#      ",
    "#.....#     ",
    "#......#    ",
    "#.......#   ",
    "#........#  ",
    "#.....#####",
    "#..#..#     ",
    "#.# #..#    ",
    "##  #..#    ",
    "#    #..#   ",
    "     #..#   ",
    "      ##    ",
];

fn arrow_image(fill: [u8; 3], outline: [u8; 3], scale: u32) -> Image {
    let w = ARROW.iter().map(|row| row.len()).max().unwrap_or(0) as u32;
    let h = ARROW.len() as u32;
    let (width, height) = (w * scale, h * scale);
    let mut data = vec![0u8; (width * height * 4) as usize];
    for (y, row) in ARROW.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            let rgb = match c {
                '#' => outline,
                '.' => fill,
                _ => continue,
            };
            for dy in 0..scale {
                for dx in 0..scale {
                    let px = x as u32 * scale + dx;
                    let py = y as u32 * scale + dy;
                    let i = ((py * width + px) * 4) as usize;
                    data[i..i + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
                }
            }
        }
    }
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

/// What the arrow looks like: fill, outline and pixel scale.
type ArrowKey = ([u8; 3], [u8; 3], u32);

/// The arrow for the current settings, rebuilt when they change.
#[derive(Default)]
pub struct ThemeCursor {
    key: Option<ArrowKey>,
    cursor: Option<CursorIcon>,
}

/// Swap the plain system arrow for the theme's pixel arrow. egui still
/// shows the system text, resize and hand pointers where they belong.
pub fn update_cursor(
    mut commands: Commands,
    settings: Res<Settings>,
    window: Single<(Entity, &Window, Option<&CursorIcon>), With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut state: Local<ThemeCursor>,
) {
    let (entity, window, current) = *window;
    let scale = (window.scale_factor() * 1.5).round().max(1.0) as u32;
    let wanted = settings
        .cursor
        .colors(settings.theme())
        .map(|(fill, outline)| (fill, outline, scale));
    if state.key != wanted {
        state.key = wanted;
        state.cursor = wanted.map(|(fill, outline, scale)| {
            CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
                handle: images.add(arrow_image(fill, outline, scale)),
                hotspot: (0, 0),
                ..default()
            }))
        });
    }
    let plain = CursorIcon::System(SystemCursorIcon::Default);
    let next = match (&state.cursor, current) {
        // Only the plain arrow is replaced; egui's other pointers stay.
        (Some(custom), None) => custom,
        (Some(custom), Some(now)) if *now == plain || matches!(now, CursorIcon::Custom(_)) => {
            custom
        }
        (None, Some(CursorIcon::Custom(_))) => &plain,
        _ => return,
    };
    if current != Some(next) {
        commands.entity(entity).insert(next.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_rows_fit_the_image() {
        let image = arrow_image([0, 0, 0], [255, 255, 255], 2);
        assert_eq!(image.width(), 24);
        assert_eq!(image.height(), 34);
        // The hotspot pixel (top-left) is part of the arrow.
        assert_eq!(image.data.as_ref().unwrap()[3], 255);
    }

    #[test]
    fn settings_round_trip() {
        let settings = Settings {
            theme: ThemeId::Ember,
            font: FontChoice::MapleMono,
            text_size: 16.0,
            cursor: CursorChoice::Light,
            island_bob: false,
        };
        let text = ron::to_string(&settings).unwrap();
        assert_eq!(ron::from_str::<Settings>(&text).unwrap(), settings);
        // Missing fields load as defaults.
        let old: Settings = ron::from_str("(theme: Midnight)").unwrap();
        assert_eq!(old.font, FontChoice::JetBrainsMono);
        assert!(old.island_bob);
    }
}
