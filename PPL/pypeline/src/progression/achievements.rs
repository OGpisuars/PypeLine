//! Achievements: goals reached by playing, some of which unlock crafter
//! recipes. Checked once per game second; earned ones are kept in the
//! player's `Progress` and saved with the factory.

use std::collections::BTreeSet;

use bevy::prelude::*;

use super::contracts::Progress;
use super::saves::SaveNow;
use crate::audio::SoundCue;
use crate::audio::sfx::Sfx;
use crate::factory::items::ItemKind;
use crate::factory::recipes::Recipe;
use crate::factory::{Factory, SimSet};
use crate::scripting::{Console, ConsoleKind};

pub struct Achievement {
    /// Saved in players' progress, so never change it once released.
    pub id: &'static str,
    pub title: &'static str,
    /// What to do, said to the player.
    pub goal: &'static str,
    /// The recipe it unlocks, if any.
    pub unlocks: Option<Recipe>,
    reached: fn(&Factory) -> bool,
}

impl Achievement {
    pub fn reached(&self, factory: &Factory) -> bool {
        (self.reached)(factory)
    }
}

/// Every achievement, in the order they are usually earned.
pub const ALL: &[Achievement] = &[
    Achievement {
        id: "hot_metal",
        title: "Hot Metal",
        goal: "Make 25 iron plates.",
        unlocks: Some(Recipe::IronGear),
        reached: |f| f.produced(ItemKind::IronPlate) >= 25,
    },
    Achievement {
        id: "fork_in_the_road",
        title: "Fork in the Road",
        goal: "Build a splitter.",
        unlocks: Some(Recipe::IronPipe),
        reached: |f| f.conveyors.values().any(|belt| belt.split.is_some()),
    },
    Achievement {
        id: "gear_head",
        title: "Gear Head",
        goal: "Make 20 iron gears.",
        unlocks: Some(Recipe::Engine),
        reached: |f| f.produced(ItemKind::IronGear) >= 20,
    },
    Achievement {
        id: "full_steam_ahead",
        title: "Full Steam Ahead",
        goal: "Make 5 engines.",
        unlocks: None,
        reached: |f| f.produced(ItemKind::Engine) >= 5,
    },
    Achievement {
        id: "deep_pockets",
        title: "Deep Pockets",
        goal: "Have 2500 coins at once.",
        unlocks: None,
        reached: |f| f.coins >= 2500,
    },
];

/// The achievement that unlocks `recipe`.
pub fn unlocked_by(recipe: Recipe) -> Option<&'static Achievement> {
    ALL.iter().find(|a| a.unlocks == Some(recipe))
}

impl Progress {
    /// Recipes the player's achievements have unlocked.
    pub fn recipes(&self) -> BTreeSet<Recipe> {
        ALL.iter()
            .filter(|a| self.achievements.contains(a.id))
            .filter_map(|a| a.unlocks)
            .collect()
    }
}

pub struct AchievementPlugin;

impl Plugin for AchievementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveNow>()
            .add_systems(FixedUpdate, check_achievements.in_set(SimSet::Progress));
    }
}

fn check_achievements(
    factory: Res<Factory>,
    mut progress: ResMut<Progress>,
    mut console: ResMut<Console>,
    mut sounds: MessageWriter<SoundCue>,
    mut save_now: ResMut<SaveNow>,
) {
    if !factory.ticks.is_multiple_of(20) {
        return;
    }
    for achievement in ALL {
        if progress.achievements.contains(achievement.id) || !achievement.reached(&factory) {
            continue;
        }
        progress.achievements.insert(achievement.id.to_owned());
        console.push(
            ConsoleKind::Info,
            format!("ACHIEVEMENT: {}! {}", achievement.title, achievement.goal),
        );
        if let Some(recipe) = achievement.unlocks {
            console.push(
                ConsoleKind::Info,
                format!(
                    "New crafter recipe: {}. Click a crafter to pick it, or use recipe=\"{}\".",
                    recipe.describe(),
                    recipe.id()
                ),
            );
        }
        sounds.write(SoundCue(Sfx::Boot));
        save_now.0 = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_every_recipe_can_be_unlocked() {
        for (i, a) in ALL.iter().enumerate() {
            assert!(ALL[..i].iter().all(|b| b.id != a.id), "{} twice", a.id);
        }
        for recipe in Recipe::ALL {
            assert!(unlocked_by(recipe).is_some(), "{recipe:?}");
        }
    }

    #[test]
    fn achievements_unlock_recipes() {
        let mut progress = Progress::default();
        assert!(progress.recipes().is_empty());
        let mut factory = Factory::default();
        factory.produced.insert(ItemKind::IronPlate, 30);
        let hot_metal = &ALL[0];
        assert!(hot_metal.reached(&factory));
        progress.achievements.insert(hot_metal.id.to_owned());
        assert_eq!(progress.recipes(), BTreeSet::from([Recipe::IronGear]));
    }
}
