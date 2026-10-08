//! The title menu: the gold PYPELINE title over the island, with Play,
//! Settings and Quit. The factory waits until Play. Enter also plays, and
//! the pause menu (Esc) or View > Title screen comes back here.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use super::settings::SettingsWindow;
use crate::engine::screens::Screen;
use crate::engine::splash::letter;

/// Size of one title pixel, in screen points, at most.
const TITLE_PIXEL: f32 = 9.0;
const TITLE: &str = "PYPELINE";

const GOLD: egui::Color32 = egui::Color32::from_rgb(248, 200, 72);
const GOLD_DARK: egui::Color32 = egui::Color32::from_rgb(216, 144, 40);
const OUTLINE: egui::Color32 = egui::Color32::from_rgb(40, 32, 48);

/// The title's lit pixels as (column, row, color), and its size in pixels
/// including a one-pixel outline all round and a shadow below.
fn title_pixels() -> (Vec<(i32, i32, egui::Color32)>, i32, i32) {
    let mut lit = Vec::new();
    for (i, c) in TITLE.chars().enumerate() {
        for (y, row) in letter(c).iter().enumerate() {
            for (x, px) in row.chars().enumerate() {
                if px == '#' {
                    lit.push((i as i32 * 6 + x as i32 + 1, y as i32 + 1));
                }
            }
        }
    }
    let mut pixels = Vec::new();
    // Outline and a one-pixel shadow first, then the gold on top.
    for &(x, y) in &lit {
        for (dx, dy) in [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
            (0, 2),
            (1, 2),
        ] {
            pixels.push((x + dx, y + dy, OUTLINE));
        }
    }
    for &(x, y) in &lit {
        pixels.push((x, y, if y <= 3 { GOLD } else { GOLD_DARK }));
    }
    let width = TITLE.len() as i32 * 6 + 1;
    (pixels, width, 7 + 3)
}

/// A dialogue box, so menu buttons read clearly over the island.
pub fn menu_box(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::window(ui.style())
        .inner_margin(egui::Margin::same(18))
        .show(ui, add);
}

/// One big menu button.
pub fn menu_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add_sized(
        [240.0, 46.0],
        egui::Button::new(egui::RichText::new(text).size(22.0)),
    )
}

pub fn main_menu(
    mut contexts: EguiContexts,
    time: Res<Time<Real>>,
    mut next: ResMut<NextState<Screen>>,
    mut settings: ResMut<SettingsWindow>,
    mut exit: MessageWriter<AppExit>,
    mut shown_at: Local<Option<f32>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let screen = ctx.viewport_rect();
    let now = time.elapsed_secs();
    let since = now - *shown_at.get_or_insert(now);
    let appear = (since / 0.6).clamp(0.0, 1.0);

    // Dim the island, then draw the title, both on the background layer so
    // the Settings window always covers them.
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Background,
        egui::Id::new("menu_title"),
    ));
    painter.rect_filled(
        screen,
        0.0,
        egui::Color32::from_black_alpha((110.0 * appear) as u8),
    );

    // The title, gently floating.
    let (pixels, cols, lines) = title_pixels();
    let (cols, lines) = (cols as f32, lines as f32);
    let px = (screen.width() * 0.8 / cols)
        .min(TITLE_PIXEL)
        .floor()
        .max(2.0);
    let float = ((now * 1.6).sin() * px * 0.6).round();
    let top_left = egui::pos2(
        (screen.center().x - cols * px / 2.0).round(),
        (screen.min.y + screen.height() * 0.14).round() + float,
    );
    for (x, y, color) in pixels {
        let at = top_left + egui::vec2(x as f32, y as f32) * px;
        painter.rect_filled(
            egui::Rect::from_min_size(at, egui::vec2(px, px)),
            0.0,
            color.gamma_multiply(appear),
        );
    }

    let below_title = top_left.y - float + lines * px + 20.0;
    let mut play = !settings.open && ctx.input(|i| i.key_pressed(egui::Key::Enter));
    if settings.open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        settings.open = false;
    }
    let mut quit = false;
    egui::Area::new(egui::Id::new("main_menu"))
        .order(egui::Order::Middle)
        .fixed_pos(egui::pos2(screen.center().x, below_title))
        .pivot(egui::Align2::CENTER_TOP)
        .show(ctx, |ui| {
            ui.set_opacity(appear);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("A factory that runs on real Python")
                        .size(18.0)
                        .color(egui::Color32::from_rgb(248, 240, 208)),
                );
                ui.add_space(16.0);
                // While Settings is open it takes the buttons' place.
                if settings.open {
                    return;
                }
                menu_box(ui, |ui| {
                    if menu_button(ui, "▶  Play").on_hover_text("Enter").clicked() {
                        play = true;
                    }
                    ui.add_space(10.0);
                    if menu_button(ui, "⚙  Settings").clicked() {
                        settings.open = true;
                    }
                    ui.add_space(10.0);
                    if menu_button(ui, "✖  Quit").clicked() {
                        quit = true;
                    }
                });
            });
        });

    egui::Area::new(egui::Id::new("main_menu_footer"))
        .order(egui::Order::Middle)
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(16.0, -12.0))
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_opacity(appear);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.label(
                egui::RichText::new(format!(
                    "v{}  ·  © KiloKilo Games  ·  M mutes the music",
                    env!("CARGO_PKG_VERSION")
                ))
                .color(egui::Color32::from_rgb(208, 208, 224)),
            );
        });

    if play {
        settings.open = false;
        *shown_at = None;
        next.set(Screen::Playing);
    }
    if quit {
        exit.write(AppExit::Success);
    }
    ctx.request_repaint();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_fits_its_box() {
        let (pixels, width, height) = title_pixels();
        assert!(!pixels.is_empty());
        for (x, y, _) in pixels {
            assert!(
                (0..width).contains(&x) && (0..height).contains(&y),
                "({x}, {y})"
            );
        }
    }
}
