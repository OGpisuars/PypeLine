//! The HUD card in the top-right corner of the game area: coins, what the
//! factory has made, the active contract's progress bar, a PAUSED / HALTED
//! badge, and small details (tick, fps, zoom, the tile under the mouse).
//! It follows the Settings theme like every window.

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{EguiContexts, egui};

use super::manual::ManualState;
use crate::engine::camera::{GameArea, HoveredTile, PixelScale};
use crate::factory::items::ItemKind;
use crate::factory::tick::SimControl;
use crate::factory::{Factory, ProductionHistory, SimTick};
use crate::progression::chapters;
use crate::progression::contracts::{self, Progress};

/// Size of one icon pixel, in screen points.
const ICON_PIXEL: f32 = 3.0;

// 8x8 pixel icons. `.` is clear; other letters are colors (see `ink`).
const COIN: [&str; 8] = [
    "..####..", ".#wyyy#.", "#wyyyyo#", "#yyoyyo#", "#yyoyyo#", "#yyyyoo#", ".#yooo#.", "..####..",
];
const ORE: [&str; 8] = [
    "...###..", "..#llm#.", ".#lmmGm#", "#lmrmGG#", "#mmGmrG#", "#mGGGGG#", ".######.", "........",
];
const PLATE: [&str; 8] = [
    "........", ".######.", "#wllllm#", "#llllmm#", "#llmmmG#", "#mmmmGG#", ".######.", "........",
];
const CONTRACT: [&str; 8] = [
    "..#yy#..", ".######.", ".#wwww#.", ".#w##w#.", ".#wwww#.", ".#w##w#.", ".#wwww#.", ".######.",
];

/// The color of one icon pixel (the world palette's outline, brass and metal).
fn ink(c: char) -> Option<egui::Color32> {
    let [r, g, b] = match c {
        '#' => [40, 32, 48],
        'w' => [248, 240, 208],
        'y' => [248, 200, 72],
        'o' => [216, 144, 40],
        'l' => [184, 192, 200],
        'm' => [144, 152, 168],
        'G' => [88, 96, 112],
        'r' => [200, 112, 64],
        _ => return None,
    };
    Some(egui::Color32::from_rgb(r, g, b))
}

fn icon(ui: &mut egui::Ui, rows: &[&str; 8]) {
    let (rect, _) =
        ui.allocate_exact_size(egui::Vec2::splat(8.0 * ICON_PIXEL), egui::Sense::hover());
    let painter = ui.painter();
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            if let Some(color) = ink(c) {
                let at = rect.min + egui::vec2(x as f32, y as f32) * ICON_PIXEL;
                painter.rect_filled(
                    egui::Rect::from_min_size(at, egui::Vec2::splat(ICON_PIXEL)),
                    0.0,
                    color,
                );
            }
        }
    }
}

/// A small rounded label, like PAUSED.
fn badge(ui: &mut egui::Ui, text: &str, fill: egui::Color32) {
    egui::Frame::new()
        .fill(fill)
        .corner_radius(6)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .strong()
                    .size(13.0)
                    .color(egui::Color32::WHITE),
            );
        });
}

/// What the HUD shows about the contract being worked on.
enum ContractLine {
    None,
    Active { title: String, have: u64, need: u64 },
}

#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
pub fn hud(
    mut contexts: EguiContexts,
    tick: Res<SimTick>,
    scale: Res<PixelScale>,
    factory: Res<Factory>,
    hovered: Res<HoveredTile>,
    progress: Res<Progress>,
    control: Res<SimControl>,
    area: Res<GameArea>,
    diagnostics: Res<DiagnosticsStore>,
    history: Res<ProductionHistory>,
    mut manual: ResMut<ManualState>,
) -> Result {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let contract = match progress
        .active
        .as_ref()
        .and_then(|a| chapters::contract(&a.id).map(|c| (a, c.1)))
    {
        Some((active, contract)) => {
            let (have, need) = contracts::goal_progress(
                &contract.goal,
                active,
                &factory,
                &history.per_minute(&factory),
            );
            ContractLine::Active {
                title: contract.title.clone(),
                have,
                need,
            }
        }
        None => ContractLine::None,
    };

    egui::Area::new(egui::Id::new("hud"))
        // Under every window, so it never covers the Manual or Help.
        .order(egui::Order::Background)
        .pivot(egui::Align2::RIGHT_TOP)
        .fixed_pos(area.0.map_or(egui::pos2(8.0, 8.0), |a| {
            egui::pos2(a.max.x - 10.0, a.min.y + 10.0)
        }))
        .show(contexts.ctx_mut()?, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            let visuals = ui.visuals().clone();
            let weak = visuals.weak_text_color();
            egui::Frame::window(ui.style())
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    // A fixed width, so separators do not stretch the card.
                    ui.set_width(262.0);

                    // Coins, big, with the status badge on the right.
                    ui.horizontal(|ui| {
                        icon(ui, &COIN);
                        ui.label(
                            egui::RichText::new(factory.coins.to_string())
                                .size(24.0)
                                .strong(),
                        );
                        ui.label(egui::RichText::new("coins").color(weak));
                        // (Not right-aligned: that would stretch the card.)
                        ui.add_space(8.0);
                        if factory.halted {
                            badge(ui, "HALTED", visuals.error_fg_color);
                        } else if control.paused {
                            badge(ui, "PAUSED", visuals.selection.bg_fill);
                        } else if control.speed > 1 {
                            badge(
                                ui,
                                &format!("{}x", control.speed),
                                visuals.selection.bg_fill,
                            );
                        }
                    });
                    if let Some(sale) = factory.last_sale.as_ref().filter(|s| s.coins > 0) {
                        ui.label(
                            egui::RichText::new(format!("last train  +{}", sale.coins))
                                .small()
                                .color(weak),
                        );
                    }

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        icon(ui, &ORE);
                        ui.label(
                            egui::RichText::new(factory.produced(ItemKind::IronOre).to_string())
                                .strong(),
                        );
                        ui.label(egui::RichText::new("ore").color(weak));
                        ui.add_space(10.0);
                        icon(ui, &PLATE);
                        ui.label(
                            egui::RichText::new(factory.produced(ItemKind::IronPlate).to_string())
                                .strong(),
                        );
                        ui.label(egui::RichText::new("plates").color(weak));
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        icon(ui, &CONTRACT);
                        ui.vertical(|ui| match &contract {
                            ContractLine::Active { title, have, need } => {
                                ui.label(egui::RichText::new("CONTRACT").small().color(weak));
                                ui.label(egui::RichText::new(title).strong());
                                ui.add(
                                    egui::ProgressBar::new(*have as f32 / (*need).max(1) as f32)
                                        .desired_width(200.0)
                                        .fill(visuals.selection.bg_fill)
                                        .text(format!("{have} / {need}")),
                                );
                                if have >= need {
                                    ui.label(
                                        egui::RichText::new("Goal met! See the console.")
                                            .small()
                                            .color(weak),
                                    );
                                }
                            }
                            ContractLine::None => {
                                ui.label(egui::RichText::new("CONTRACT").small().color(weak));
                                ui.label("None yet");
                                if ui.small_button("Open the Manual (F2)").clicked() {
                                    manual.open = true;
                                }
                            }
                        });
                    });

                    ui.separator();
                    let status = format!("tick {}  ·  {fps:.0} fps  ·  zoom {}x", tick.0, scale.0);
                    ui.label(egui::RichText::new(status).small().monospace().color(weak));
                    let mouse = match hovered.0 {
                        Some(pos) => format!("mouse on x={}, y={}", pos.x, pos.y),
                        None => "point at the island to see x and y".to_owned(),
                    };
                    ui.label(egui::RichText::new(mouse).small().monospace().color(weak));
                });
        });
    Ok(())
}
