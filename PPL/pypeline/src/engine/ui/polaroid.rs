//! Polaroid hover card: point at a machine or belt to see a snapshot with its
//! live stats (roadmap Phase 2).

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};

use crate::engine::camera::HoveredTile;
use crate::engine::sprites::SpriteSheet;
use crate::factory::machines::{BUFFER_CAP, MachineKind, STATION_CAP};
use crate::factory::train::TRAIN_INTERVAL;
use crate::factory::{Dir, Factory};

/// Size of the photo in the card (a 16 px sprite at 4x).
const PHOTO: f32 = 64.0;

pub fn polaroid(
    mut contexts: EguiContexts,
    hovered: Res<HoveredTile>,
    factory: Res<Factory>,
    sheet: Option<Res<SpriteSheet>>,
) -> Result {
    let (Some(pos), Some(sheet)) = (hovered.0, sheet) else {
        return Ok(());
    };
    let card = if let Some(name) = factory.machine_at.get(&pos) {
        let machine = &factory.machines[name];
        let image = sheet.machine(machine.kind);
        let mut title = machine.kind.name().replace('_', " ");
        if machine.tier > 1 {
            title.push_str(&format!(" Mk{}", machine.tier));
        }
        let mut lines = vec![title];
        let mut progress = None;
        if machine.kind.needs_power() {
            let powered = factory.is_powered(name);
            lines.push(if powered {
                "power: on".into()
            } else {
                "power: OFF (connect it)".into()
            });
            if !machine.enabled {
                lines.push("switched off by your script".into());
            }
            progress = Some(machine.progress as f32 / machine.work_ticks().max(1) as f32);
            if let Some(ore) = machine.ore {
                lines.push(format!("digging: {}", ore.name()));
            }
            if machine.kind == MachineKind::Smelter {
                lines.push(format!("input: {}/{BUFFER_CAP}", machine.input.len()));
            }
            if machine.kind == MachineKind::Crafter {
                match machine.recipe {
                    Some(recipe) => {
                        lines.push(format!("making: {}", recipe.output().name()));
                        for &(item, count) in recipe.inputs() {
                            let held = machine.input.iter().filter(|&&i| i == item).count();
                            lines.push(format!("  {}: {held}/{count}", item.name()));
                        }
                    }
                    None => lines.push("no recipe yet".into()),
                }
                lines.push("click it to pick a recipe".into());
            }
            lines.push(format!("output: {}/{BUFFER_CAP}", machine.output.len()));
            lines.push(format!("faces: {}", dir_name(machine.dir)));
        } else if machine.kind == MachineKind::Station {
            lines.push(format!(
                "waiting for the train: {}/{STATION_CAP}",
                machine.input.len()
            ));
            let next = TRAIN_INTERVAL - factory.ticks % TRAIN_INTERVAL;
            lines.push(format!("next train in {} s", next / 20));
        } else {
            let powering = factory.power.values().filter(|g| *g == name).count();
            lines.push(format!("powering {powering} machine(s)"));
            let target = crate::factory::thermal::target_temperature(&factory, name);
            lines.push(format!(
                "temperature: {} (heading for {target})",
                machine.heat
            ));
            if machine.overheated {
                lines.push(format!(
                    "OVERHEATED: back on at {}",
                    crate::factory::thermal::RESTART
                ));
            } else if target >= crate::factory::thermal::OVERHEAT {
                lines.push("will overheat: switch machines off".into());
            }
        }
        Some((name.clone(), image, lines, progress))
    } else {
        factory.conveyors.get(&pos).map(|belt| {
            let (image, mut lines, kind) = match belt.split {
                Some(split) => (
                    sheet.splitter(split.outputs),
                    vec![
                        format!(
                            "splits {} / {}",
                            dir_name(split.outputs[0]),
                            dir_name(split.outputs[1])
                        ),
                        format!("next item goes {}", dir_name(split.current())),
                        format!("items on it: {}", belt.items.len()),
                    ],
                    "splitter",
                ),
                None => (
                    sheet.belt(belt.dir, 0),
                    vec![
                        format!("moving {}", dir_name(belt.dir)),
                        format!("items on it: {}", belt.items.len()),
                    ],
                    "conveyor",
                ),
            };
            match belt.tier {
                2 => lines.push("fast belt (tier 2)".into()),
                3 => lines.push("express belt (tier 3)".into()),
                _ => {}
            }
            (format!("{kind} ({}, {})", pos.x, pos.y), image, lines, None)
        })
    };
    let Some((title, image, lines, progress)) = card else {
        return Ok(());
    };

    let texture = contexts.add_image(EguiTextureHandle::Weak(image.id()));
    let ctx = contexts.ctx_mut()?;
    let Some(pointer) = ctx.pointer_hover_pos() else {
        return Ok(());
    };
    egui::Area::new(egui::Id::new("polaroid"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pointer + egui::vec2(18.0, 18.0))
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(248, 248, 240))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(176, 168, 152),
                ))
                .inner_margin(egui::Margin {
                    left: 8,
                    right: 8,
                    top: 8,
                    bottom: 14,
                })
                .show(ui, |ui| {
                    ui.set_width(PHOTO + 72.0);
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgb(120, 192, 248))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add(egui::Image::new((texture, egui::vec2(PHOTO, PHOTO))));
                            });
                        });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(title).strong());
                    for line in lines {
                        ui.label(egui::RichText::new(line).small());
                    }
                    if let Some(progress) = progress {
                        ui.add(egui::ProgressBar::new(progress).desired_height(6.0));
                    }
                });
        });
    Ok(())
}

fn dir_name(dir: Dir) -> &'static str {
    match dir {
        Dir::North => "north",
        Dir::East => "east",
        Dir::South => "south",
        Dir::West => "west",
    }
}
