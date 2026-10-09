//! Contract progress: accepting contracts, tracking their goals, checking
//! the script uses the concepts they ask for, paying rewards, and unlocking
//! chapters (roadmap Phase 3A).

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::chapters::{self, Chapter, Contract, Goal};
use crate::audio::SoundCue;
use crate::audio::sfx::Sfx;
use crate::factory::items::ItemKind;
use crate::factory::{Factory, SimSet};
use crate::scripting::concepts::{self, ScriptShape};
use crate::scripting::runtime::EventArg;
use crate::scripting::{Console, ConsoleKind, DefinedHandlers, LastGoodScript, ScriptEvents};

/// A contract being worked on, with the factory's counters when it started.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveContract {
    pub id: String,
    pub produced_at_start: BTreeMap<ItemKind, u64>,
    pub coins_at_start: u64,
    pub started_tick: u64,
    /// Already told the player the goal is met but the script is missing a
    /// required concept (so the console is not flooded).
    #[serde(default)]
    pub warned: bool,
}

/// How a contract was beaten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Score {
    pub lines: usize,
    pub steam: u64,
    pub seconds: u64,
}

/// The player's manual progress. Saved with the factory.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub completed: BTreeSet<String>,
    pub active: Option<ActiveContract>,
    pub best: BTreeMap<String, Score>,
    /// How many hints the player has opened, per contract.
    pub hints_seen: BTreeMap<String, usize>,
    /// Achievements earned, by id (see `achievements.rs`).
    pub achievements: BTreeSet<String>,
}

impl Progress {
    /// Done by its own contracts: all of them, or its chapter test.
    fn chapter_done_itself(&self, chapter: &Chapter) -> bool {
        chapter
            .contracts
            .iter()
            .all(|c| self.completed.contains(&c.id))
            || chapter
                .contracts
                .iter()
                .any(|c| c.chapter_test && self.completed.contains(&c.id))
    }

    /// A chapter counts as done if it, or any later chapter, is done
    /// (passing a later chapter test skips the earlier ones).
    pub fn chapter_done(&self, number: u32) -> bool {
        chapters::chapters()
            .iter()
            .filter(|ch| ch.number >= number)
            .any(|ch| self.chapter_done_itself(ch))
    }

    pub fn chapter_unlocked(&self, number: u32) -> bool {
        number == 1 || self.chapter_done(number - 1)
    }

    /// Can this contract be accepted? Contracts in unlocked chapters can, and
    /// any chapter test can (that is how experienced coders skip ahead).
    pub fn can_accept(&self, chapter: &Chapter, contract: &Contract) -> bool {
        !self.completed.contains(&contract.id)
            && (self.chapter_unlocked(chapter.number) || contract.chapter_test)
    }

    pub fn accept(&mut self, contract: &Contract, factory: &Factory) {
        if self.completed.contains(&contract.id) {
            return;
        }
        self.active = Some(ActiveContract {
            id: contract.id.clone(),
            produced_at_start: factory.produced.clone(),
            coins_at_start: factory.coins,
            started_tick: factory.ticks,
            warned: false,
        });
    }
}

/// Progress toward a goal: (done so far, target).
/// How far along a goal is: (have, need). `per_minute` is the factory's
/// items made over the last minute.
pub fn goal_progress(
    goal: &Goal,
    active: &ActiveContract,
    factory: &Factory,
    per_minute: &std::collections::BTreeMap<crate::factory::items::ItemKind, u64>,
) -> (u64, u64) {
    match *goal {
        Goal::Rate {
            item,
            per_minute: target,
        } => (
            per_minute.get(&item).copied().unwrap_or(0).min(target),
            target,
        ),
        Goal::Produce { item, count } => {
            let start = active.produced_at_start.get(&item).copied().unwrap_or(0);
            (
                factory.produced(item).saturating_sub(start).min(count),
                count,
            )
        }
        Goal::Earn { coins } => (
            factory
                .coins
                .saturating_sub(active.coins_at_start)
                .min(coins),
            coins,
        ),
    }
}

/// What the script still lacks for this contract (empty = all good).
pub fn missing_requirements(contract: &Contract, shape: &ScriptShape) -> Vec<String> {
    let mut missing: Vec<String> = contract
        .requires
        .iter()
        .filter(|c| !shape.concepts.contains(c))
        .map(|c| c.describe().to_owned())
        .collect();
    if let Some(max) = contract.max_lines
        && shape.code_lines > max
    {
        missing.push(format!(
            "at most {max} lines of code (yours has {})",
            shape.code_lines
        ));
    }
    missing
}

/// The default success banner (players can print their own from chapter 9).
pub fn banner(title: &str, reward: u64) -> Vec<String> {
    let lines = [
        format!("CONTRACT COMPLETE: {title}"),
        format!("+{reward} coins"),
    ];
    let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) + 4;
    let edge = "*".repeat(width + 2);
    let mut out = vec![edge.clone()];
    for line in lines {
        out.push(format!("*  {line:<w$}*", w = width - 2));
    }
    out.push(edge);
    out
}

pub struct ContractPlugin;

impl Plugin for ContractPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Progress>()
            .init_resource::<super::saves::SaveNow>()
            .add_systems(
                FixedUpdate,
                (track_contract, train_events).in_set(SimSet::Progress),
            );
    }
}

#[allow(clippy::too_many_arguments)] // A Bevy system: each argument is one resource or query.
fn track_contract(
    mut progress: ResMut<Progress>,
    mut factory: ResMut<Factory>,
    last_good: Res<LastGoodScript>,
    mut console: ResMut<Console>,
    mut sounds: MessageWriter<SoundCue>,
    handlers: Res<DefinedHandlers>,
    mut events: ResMut<ScriptEvents>,
    history: Res<crate::factory::ProductionHistory>,
    mut save_now: ResMut<super::saves::SaveNow>,
    mut was_hot: Local<bool>,
) {
    let hot = factory.machines.values().any(|m| m.overheated);
    let just_overheated = hot && !*was_hot;
    *was_hot = hot;
    let Some(active) = progress.active.clone() else {
        return;
    };
    let Some((chapter, contract)) = chapters::contract(&active.id) else {
        progress.active = None;
        return;
    };
    // Each contract pays once. (Older saves could hold one that was both.)
    if progress.completed.contains(&contract.id) {
        progress.active = None;
        return;
    }
    if contract.keep_cool && hot {
        // Start over from now, so the goal counts only cool running.
        progress.accept(contract, &factory);
        if !just_overheated {
            return;
        }
        console.push(
            ConsoleKind::Error,
            format!(
                "A boiler overheated! \"{}\" needs it to stay cool, so it starts over.",
                contract.title
            ),
        );
        return;
    }
    let rates = history.per_minute(&factory);
    let (done, target) = goal_progress(&contract.goal, &active, &factory, &rates);
    if done < target {
        return;
    }
    let shape = concepts::analyze_program(last_good.program.sources());
    let missing = missing_requirements(contract, &shape);
    if !missing.is_empty() {
        if !active.warned {
            console.push(
                ConsoleKind::Error,
                if last_good.program.main.trim().is_empty() {
                    // Loaded from an older save that did not keep the script.
                    format!(
                        "Goal reached! Press Run once so \"{}\" can check your script.",
                        contract.title
                    )
                } else {
                    format!(
                        "Goal reached! But \"{}\" also needs {}. Change your script and Run again.",
                        contract.title,
                        missing.join(" and ")
                    )
                },
            );
            if let Some(active) = progress.active.as_mut() {
                active.warned = true;
            }
        }
        return;
    }

    let was_done = progress.chapter_done(chapter.number);
    factory.coins += contract.reward;
    progress.completed.insert(contract.id.clone());
    progress.active = None;
    // Save now, so a crash or quit cannot hand out the same contract again.
    save_now.0 = true;
    let score = Score {
        lines: shape.code_lines,
        steam: last_good.steps,
        seconds: factory.ticks.saturating_sub(active.started_tick) / 20,
    };
    let best = progress.best.entry(contract.id.clone()).or_insert(score);
    if score.lines < best.lines || (score.lines == best.lines && score.steam < best.steam) {
        *best = score;
    }
    if handlers.on_contract_complete {
        // The player's own celebration (chapter 9) replaces the default banner.
        events.0.push((
            "on_contract_complete",
            vec![EventArg::Text(contract.title.clone())],
        ));
    } else {
        for line in banner(&contract.title, contract.reward) {
            console.push(ConsoleKind::Info, line);
        }
    }
    console.push(
        ConsoleKind::Info,
        format!(
            "Score: {} lines, {} steam, {} s.",
            score.lines, score.steam, score.seconds
        ),
    );
    if !was_done && progress.chapter_done(chapter.number) {
        let next = chapters::chapters()
            .iter()
            .find(|ch| ch.number == chapter.number + 1);
        console.push(
            ConsoleKind::Info,
            match next {
                Some(next) => format!(
                    "Chapter {} complete! Chapter {} unlocked: {}. Open the Manual (F2).",
                    chapter.number, next.number, next.title
                ),
                None => format!(
                    "Chapter {} complete! That is every chapter for now. Well done, engineer!",
                    chapter.number
                ),
            },
        );
    }
    sounds.write(SoundCue(Sfx::Boot));
}

/// Tell the script about each train visit: on_train(coins).
fn train_events(
    factory: Res<Factory>,
    mut events: ResMut<ScriptEvents>,
    mut seen: Local<Option<u64>>,
) {
    let Some(sale) = &factory.last_sale else {
        return;
    };
    // The first sale seen may come from a loaded save; do not replay it.
    if seen.is_none() {
        *seen = Some(sale.tick);
        return;
    }
    if *seen != Some(sale.tick) {
        *seen = Some(sale.tick);
        events
            .0
            .push(("on_train", vec![EventArg::Int(sale.coins as i64)]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapters_unlock_in_order_and_tests_skip_ahead() {
        let mut progress = Progress::default();
        assert!(progress.chapter_unlocked(1));
        assert!(!progress.chapter_unlocked(2));
        // Passing chapter 3's test skips chapters 1-3.
        let test_id = chapters::chapters()[2]
            .contracts
            .iter()
            .find(|c| c.chapter_test)
            .unwrap()
            .id
            .clone();
        progress.completed.insert(test_id);
        assert!(progress.chapter_done(1));
        assert!(progress.chapter_done(3));
        assert!(progress.chapter_unlocked(4));
        assert!(!progress.chapter_unlocked(5));
    }

    #[test]
    fn a_finished_contract_cannot_be_taken_again() {
        let (chapter, contract) = chapters::contract("ch3_long_haul").unwrap();
        let mut progress = Progress::default();
        progress.completed.insert(contract.id.clone());
        assert!(!progress.can_accept(chapter, contract));
        progress.accept(contract, &Factory::default());
        assert_eq!(progress.active, None);
    }

    #[test]
    fn requirements_are_checked() {
        let (_, contract) = chapters::contract("ch3_long_haul").unwrap();
        let shape = concepts::analyze("x = 1").unwrap();
        assert_eq!(missing_requirements(contract, &shape), vec!["a for loop"]);
    }

    #[test]
    fn banner_is_a_box() {
        let lines = banner("First Plates", 10);
        let width = lines[0].chars().count();
        assert!(
            lines.iter().all(|l| l.chars().count() == width),
            "{lines:#?}"
        );
    }
}
