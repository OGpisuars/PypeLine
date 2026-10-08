//! Stats window (F4): items per minute, coins, steam, uptime, and what every
//! machine is doing, with bottlenecks glowing on the island (roadmap Phase
//! 3B: stats dashboard).

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::factory::Factory;
use crate::factory::ProductionHistory;
use crate::factory::items::ItemKind;
use crate::factory::stats::{self, MachineState, StatsHistory};
use crate::scripting::LastGoodScript;
use crate::scripting::budget::{DEPLOY_BUDGET, TICK_BUDGET};

#[derive(Resource)]
pub struct StatsWindow {
    pub open: bool,
    /// Outline blocked and starved machines on the island.
    pub glow: bool,
}

impl Default for StatsWindow {
    fn default() -> Self {
        Self {
            open: false,
            glow: true,
        }
    }
}

impl StatsWindow {
    /// Should the island show the bottleneck glow right now?
    pub fn glowing(&self) -> bool {
        self.open && self.glow
    }
}

pub fn toggle_stats(keys: Res<ButtonInput<KeyCode>>, mut window: ResMut<StatsWindow>) {
    if keys.just_pressed(KeyCode::F4) {
        window.open = !window.open;
    }
}

pub fn stats_window(
    mut contexts: EguiContexts,
    mut window: ResMut<StatsWindow>,
    factory: Res<Factory>,
    history: Res<StatsHistory>,
    production: Res<ProductionHistory>,
    last_good: Res<LastGoodScript>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let screen = ctx.viewport_rect();
    let mut open = window.open;
    let mut glow = window.glow;
    egui::Window::new("Stats")
        .open(&mut open)
        .default_pos(egui::pos2(screen.max.x - 400.0, 120.0))
        .default_size(egui::vec2(380.0, 480.0))
        .show(ctx, |ui| {
            let seconds = history.seconds_covered();
            ui.weak(if seconds >= 60 {
                "Over the last minute of game time:".to_owned()
            } else {
                format!("Over the last {seconds} seconds of game time:")
            });
            let per_minute = production.per_minute(&factory);
            egui::Grid::new("stats_totals")
                .num_columns(3)
                .spacing([14.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.strong("");
                    ui.strong("per minute");
                    ui.strong("total");
                    ui.end_row();
                    for item in ItemKind::ALL {
                        ui.label(item.name());
                        ui.label(per_minute.get(&item).copied().unwrap_or(0).to_string());
                        ui.label(factory.produced(item).to_string());
                        ui.end_row();
                    }
                    ui.label("coins");
                    ui.label(history.earned_last_minute().to_string());
                    ui.label(factory.coins.to_string());
                    ui.end_row();
                });
            ui.add_space(6.0);

            let uptime = history.uptime().unwrap_or(100);
            meter(ui, "Uptime", u64::from(uptime), &format!("{uptime}% running"));
            meter(
                ui,
                "Steam: main.py",
                last_good.steps * 100 / DEPLOY_BUDGET,
                &format!("{} / {DEPLOY_BUDGET} steps", last_good.steps),
            );
            match history.tick_steam() {
                Some(steps) => meter(
                    ui,
                    "Steam: tick()",
                    steps * 100 / TICK_BUDGET,
                    &format!("{steps} / {TICK_BUDGET} steps a tick"),
                ),
                None => {
                    ui.weak("Steam: tick() is not running.");
                }
            }
            ui.separator();

            ui.horizontal(|ui| {
                ui.strong("Machines");
                ui.checkbox(&mut glow, "Glow problems on the island");
            });
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut rows: Vec<_> = factory
                    .machines
                    .keys()
                    .filter_map(|name| Some((stats::machine_state(&factory, name)?, name)))
                    .collect();
                // Problems first, then by name.
                rows.sort_by_key(|(state, name)| (*state == MachineState::Working, *name));
                if rows.is_empty() {
                    ui.weak("No machines yet. Build some and press Run.");
                }
                egui::Grid::new("stats_machines")
                    .num_columns(3)
                    .spacing([10.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for (state, name) in rows {
                            ui.label(egui::RichText::new(name).monospace());
                            let color = state_color(ui, state);
                            ui.colored_label(color, state.name())
                                .on_hover_text(state.advice());
                            let busy = history.busy(name).unwrap_or(0);
                            ui.add(
                                egui::ProgressBar::new(busy as f32 / 100.0)
                                    .desired_width(90.0)
                                    .text(format!("{busy}% busy")),
                            );
                            ui.end_row();
                        }
                    });
            });
            ui.add_space(4.0);
            ui.weak("Scripts can read these too: stats.bottlenecks(), machines.status(name)[\"state\"].");
        });
    window.open = open;
    window.glow = glow;
    Ok(())
}

/// A labeled bar, `percent` full, showing `text`.
fn meter(ui: &mut egui::Ui, label: &str, percent: u64, text: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::ProgressBar::new(percent.min(100) as f32 / 100.0)
                .desired_width(170.0)
                .text(text),
        );
    });
}

fn state_color(ui: &egui::Ui, state: MachineState) -> egui::Color32 {
    let visuals = ui.visuals();
    match state {
        MachineState::Working => visuals.text_color(),
        MachineState::Blocked | MachineState::NoPower | MachineState::Overheated => {
            visuals.error_fg_color
        }
        MachineState::Starved => visuals.warn_fg_color,
        MachineState::Off | MachineState::Idle => visuals.weak_text_color(),
    }
}
