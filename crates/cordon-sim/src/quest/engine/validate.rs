//! Load-time catalog validation.
//!
//! Runs once as a Bevy system when
//! [`GameDataResource`](cordon_data::gamedata::GameDataResource)
//! first appears. Emits warnings for authoring mistakes the
//! type system can't catch:
//!
//! - dangling quest references in trigger definitions
//! - quests with zero stages
//! - stage branch / fallback / on_success / on_failure
//!   references that don't match any stage ID in the quest
//! - duplicate [`TalkBranch::choice`](cordon_core::world::narrative::TalkBranch)
//!   values (serde keeps the first, silently shadowing the rest)
//! - authored consequences that hit the stub path in the
//!   applier ([`SpawnNpc`](Consequence::SpawnNpc),
//!   [`GiveNpcXp`](Consequence::GiveNpcXp),
//!   [`DangerModifier`](Consequence::DangerModifier),
//!   [`PriceModifier`](Consequence::PriceModifier))

use std::collections::HashSet;

use bevy::prelude::*;
use cordon_core::world::narrative::{Consequence, QuestDef, QuestStageKind};
use cordon_data::catalog::GameData;
use cordon_data::gamedata::GameDataResource;

/// Minimal type-check that the quest + trigger catalog is
/// internally consistent and that authored content does not
/// rely on consequence variants that are currently stubbed.
///
/// Scheduled with `.run_if(resource_added::<GameDataResource>)`
/// so it runs exactly once, on the frame the catalog first
/// appears. No `Local<bool>` guard needed — Bevy's resource
/// change detection handles the "fire once" semantic natively.
pub fn validate_catalog(data: Res<GameDataResource>) {
    let catalog = &data.0;
    for trigger in catalog.triggers.values() {
        if !catalog.quests.contains_key(&trigger.quest) {
            warn!(
                "quest trigger `{}` references unknown quest `{}`",
                trigger.id.as_str(),
                trigger.quest.as_str()
            );
        }
    }
    // Also sanity-check that every quest has at least one stage.
    for def in catalog.quests.values() {
        if def.stages.is_empty() {
            warn!("quest `{}` has no stages", def.id.as_str());
        }
        validate_stage_references(def);
    }
    warn_on_stub_consequences(catalog);
}

/// Walk every consequence in every quest stage and every
/// event definition, counting how many times each currently-
/// stubbed variant appears. Emits one summary warning per
/// stub variant that is actually authored against, so a
/// quest designer sees the problem at game-load time rather
/// than only when the consequence fires at runtime.
fn warn_on_stub_consequences(catalog: &GameData) {
    let mut spawn_npc = 0usize;
    let mut give_npc_xp = 0usize;
    let mut danger_modifier = 0usize;
    let mut price_modifier = 0usize;

    let mut count = |c: &Consequence| match c {
        Consequence::SpawnNpc { .. } => spawn_npc += 1,
        Consequence::GiveNpcXp { .. } => give_npc_xp += 1,
        Consequence::DangerModifier { .. } => danger_modifier += 1,
        Consequence::PriceModifier { .. } => price_modifier += 1,
        _ => {}
    };

    for def in catalog.quests.values() {
        for stage in &def.stages {
            let QuestStageKind::Outcome { consequences, .. } = &stage.kind else {
                continue;
            };
            for bundle in consequences {
                for consequence in &bundle.apply {
                    count(consequence);
                }
            }
        }
    }
    for event in catalog.events.values() {
        for consequence in &event.consequences {
            count(consequence);
        }
    }

    if spawn_npc > 0 {
        warn!(
            "STUB CONSEQUENCE `spawn_npc` referenced {spawn_npc}× in authored content \
             — no visitor queue bridge yet, these will no-op at runtime."
        );
    }
    if give_npc_xp > 0 {
        warn!(
            "STUB CONSEQUENCE `give_npc_xp` referenced {give_npc_xp}× in authored content \
             — no template→entity resolver yet, these will no-op at runtime."
        );
    }
    if danger_modifier > 0 {
        warn!(
            "STUB CONSEQUENCE `danger_modifier` referenced {danger_modifier}× in authored content \
             — no AreaStates bridge yet, these will no-op at runtime."
        );
    }
    if price_modifier > 0 {
        warn!(
            "STUB CONSEQUENCE `price_modifier` referenced {price_modifier}× in authored content \
             — no market system yet, these will no-op at runtime."
        );
    }
}

fn validate_stage_references(def: &QuestDef) {
    let ids: HashSet<_> = def.stages.iter().map(|s| &s.id).collect();
    for stage in &def.stages {
        match &stage.kind {
            QuestStageKind::Talk {
                branches, fallback, ..
            } => {
                if !ids.contains(fallback) {
                    warn!(
                        "quest `{}` stage `{}` has unknown fallback `{}`",
                        def.id.as_str(),
                        stage.id.as_str(),
                        fallback.as_str()
                    );
                }
                // Duplicate choice strings silently shadow —
                // serde is happy to let you have two branches
                // keyed by "accept" but the engine only ever
                // reaches the first. Flag it so authors catch
                // the typo at load time.
                let mut seen_choices: HashSet<&str> = HashSet::new();
                for branch in branches {
                    if !seen_choices.insert(branch.choice.as_str()) {
                        warn!(
                            "quest `{}` stage `{}` has duplicate TalkBranch choice `{}` — \
                             only the first matching branch will ever be taken",
                            def.id.as_str(),
                            stage.id.as_str(),
                            branch.choice
                        );
                    }
                    if !ids.contains(&branch.next_stage) {
                        warn!(
                            "quest `{}` stage `{}` branch `{}` → unknown stage `{}`",
                            def.id.as_str(),
                            stage.id.as_str(),
                            branch.choice,
                            branch.next_stage.as_str()
                        );
                    }
                }
            }
            QuestStageKind::Objective {
                on_success,
                on_failure,
                ..
            } => {
                if !ids.contains(on_success) {
                    warn!(
                        "quest `{}` stage `{}` on_success → unknown stage `{}`",
                        def.id.as_str(),
                        stage.id.as_str(),
                        on_success.as_str()
                    );
                }
                if let Some(on_failure) = on_failure
                    && !ids.contains(on_failure)
                {
                    warn!(
                        "quest `{}` stage `{}` on_failure → unknown stage `{}`",
                        def.id.as_str(),
                        stage.id.as_str(),
                        on_failure.as_str()
                    );
                }
            }
            QuestStageKind::Branch { arms, fallback } => {
                if !ids.contains(fallback) {
                    warn!(
                        "quest `{}` stage `{}` branch fallback → unknown stage `{}`",
                        def.id.as_str(),
                        stage.id.as_str(),
                        fallback.as_str()
                    );
                }
                for (i, arm) in arms.iter().enumerate() {
                    if !ids.contains(&arm.next_stage) {
                        warn!(
                            "quest `{}` stage `{}` branch arm #{i} → unknown stage `{}`",
                            def.id.as_str(),
                            stage.id.as_str(),
                            arm.next_stage.as_str()
                        );
                    }
                }
            }
            QuestStageKind::Outcome { .. } => {}
        }
    }
}
