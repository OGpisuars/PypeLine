//! The title menu: the gold PYPELINE title over the island, with Play,
//! Settings and Quit. The factory waits until Play. Enter also plays, and
//! View > Title screen comes back here.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use super::settings::SettingsWindow;
use crate::engine::screens::Screen;
use crate::engine::splash::title_grid;

/// Size of one title pixel, in screen points, at most.
const TITLE_PIXEL: f32 = 9.0;

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

    // Dim the island behind the menu.
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(
        screen,
        0.0,
        egui::Color32::from_black_alpha((110.0 * appear) as u8),
    );

    // The title, gently floating.
    let rows = title_grid("PYPELINE");
    let (cols, lines) = (rows[0].len() as f32, rows.len() as f32);
    let px = (screen.width() * 0.7 / cols)
        .min(TITLE_PIXEL)
        .floor()
        .max(2.0);
    let float = (now * 1.6).sin() * px * 0.6;
    let top_left = egui::pos2(
        (screen.center().x - cols * px / 2.0).round(),
        (screen.min.y + screen.height() * 0.2 + float).round(),
    );
    let title_painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("menu_title"),
    ));
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            let color = match c {
                'y' => egui::Color32::from_rgb(224, 168, 56),
                'Y' => egui::Color32::from_rgb(168, 112, 32),
                'k' => egui::Color32::from_rgb(40, 32, 48),
                _ => continue,
            };
            let at = top_left + egui::vec2(x as f32, y as f32) * px;
            title_painter.rect_filled(
                egui::Rect::from_min_size(at, egui::vec2(px, px)),
                0.0,
                color.gamma_multiply(appear),
            );
        }
    }

    let below_title = top_left.y + lines * px + 24.0;
    let mut play = ctx.input(|i| i.key_pressed(egui::Key::Enter));
    let mut quit = false;
    egui::Area::new(egui::Id::new("main_menu"))
        .order(egui::Order::Middle)
        .fixed_pos(egui::pos2(screen.center().x, below_title))
        .pivot(egui::Align2::CENTER_TOP)
        .show(ctx, |ui| {
            ui.set_opacity(appear);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("A factory that runs on real Python")
                        .size(18.0)
                        .color(egui::Color32::from_rgb(248, 240, 208)),
                );
                ui.add_space(28.0);
                let button = |ui: &mut egui::Ui, text: &str| {
                    ui.add_sized(
                        [240.0, 46.0],
                        egui::Button::new(egui::RichText::new(text).size(22.0)),
                    )
                };
                if button(ui, "▶  Play").on_hover_text("Enter").clicked() {
                    play = true;
                }
                ui.add_space(10.0);
                if button(ui, "⚙  Settings").clicked() {
                    settings.open = !settings.open;
                }
                ui.add_space(10.0);
                if button(ui, "✖  Quit").clicked() {
                    quit = true;
                }
            });
        });

    egui::Area::new(egui::Id::new("main_menu_footer"))
        .order(egui::Order::Middle)
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(16.0, -12.0))
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_opacity(appear);
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
