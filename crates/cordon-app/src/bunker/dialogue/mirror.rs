//! Mirror relevant player state into the active [`DialogueRunner`]'s
//! variable storage so Yarn `<<if>>` guards can read it directly.
//!
//! Yarn evaluates option eligibility (`<<if $var>>`) when it
//! publishes the options list. For the guards to see current
//! state, the variables have to be up to date *before* the
//! runner reaches that point. Two entry points cover that:
//!
//! 1. **Before `start_node`** — [`push_snapshot`] is called
//!    inline in [`super::systems::apply_start_dialogue`], right
//!    before asking the runner to enter the node. Guarantees
//!    freshness on entry and on re-entry after the runner has
//!    been stopped, regardless of whether anything changed since
//!    the last conversation.
//! 2. **On change** — a per-frame system watches
//!    [`Carrying`](crate::bunker::rack::Carrying),
//!    [`PlayerStash`], and [`PlayerIdentity`] for `is_changed()`
//!    and pushes when any differs. Covers mid-dialogue mutations
//!    from Yarn commands (see [`super::commands`]), quest
//!    consequences, and day-rollover expense deductions.
//!
//! Variable naming convention:
//!
//! | yarn var         | source                                             |
//! |------------------|----------------------------------------------------|
//! | `$carrying`      | item id in hand, `""` if empty                     |
//! | `$credits`       | player credits (number)                            |
//! | `$debt`          | outstanding debt (number)                          |
//! | `$<item_id>`     | total count across racks + pending stash + in-hand |
//! | `$<decision_id>` | recorded value for the decision, `""` if unset     |
//!
//! Per-item counts flatten directly onto the yarn var namespace
//! because item ids already carry an `item_` prefix (see
//! `assets/data/items/`), making the yarn var `$item_medkit`
//! self-describing without extra scaffolding. Yarn has no map
//! type, so one variable per id is the idiomatic expression.
//! Mirroring is blanket — every item def in the catalog gets a
//! variable set to the current count (0 if absent) on every
//! refresh. A few hundred `set` calls is cheap; the alternative
//! (whitelisting ids) would require a code change every time a
//! yarn file wants to gate on a new item.
//!
//! "Where does the count come from?" Items the player owns live
//! in three places: rack slots (physical world, most items
//! end up here via `drain_pending_to_racks`), `PlayerStash`
//! (the staging queue before items hit racks, plus the hidden
//! stash), and `Carrying` (whatever's in hand). The mirror sums
//! all three so a yarn gate like `<<if $item_medkit >= 1>>`
//! answers the question the player actually cares about: "do I
//! have one anywhere in the bunker right now?".

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_yarnspinner::prelude::{DialogueRunner, YarnValue};
use cordon_core::item::{Item, StashScope};
use cordon_core::primitive::Id;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::resources::{PlayerDecisions, PlayerIdentity, PlayerStash};

use super::systems::DialogueRunnerMarker;
use crate::bunker::rack::Carrying;
use crate::bunker::rack::components::RackSlot;

/// Per-frame: push a fresh snapshot whenever any mirrored
/// resource changed. Skips the push when nothing moved — the
/// dominant case — so the cost is just three `is_changed` bools
/// per frame.
///
/// `PlayerStash` / `PlayerIdentity` / `Carrying` only exist
/// inside `AppState::Playing`. The plugin registers the system
/// in every state so the `YarnProject`-gated spawn can happen
/// during loading, so we wrap the run-only resources in
/// `Option` and bail out during Menu / Loading instead of
/// scheduling with a state run-condition (which would couple
/// this module back to `AppState`).
///
/// Rack slots don't go through change detection — take/place
/// mutates the slot component, not the `Carrying` resource,
/// when the player is emptying hands onto a shelf. But every
/// such interaction *also* flips `Carrying`, so gating on
/// `carrying.is_changed()` transitively covers rack state for
/// the player's own actions. Items added to racks via
/// `drain_pending_to_racks` go through `PlayerStash`, so
/// `stash.is_changed()` covers that path. No rack-slot query
/// change-detection is needed as long as those two chains hold.
pub(super) fn mirror_on_change(
    carrying: Option<Res<Carrying>>,
    stash: Option<Res<PlayerStash>>,
    identity: Option<Res<PlayerIdentity>>,
    decisions: Option<Res<PlayerDecisions>>,
    game_data: Option<Res<GameDataResource>>,
    rack_slots: Query<&RackSlot>,
    mut runner_q: Query<&mut DialogueRunner, With<DialogueRunnerMarker>>,
) {
    let (Some(carrying), Some(stash), Some(identity), Some(decisions), Some(game_data)) =
        (carrying, stash, identity, decisions, game_data)
    else {
        return;
    };
    if !carrying.is_changed()
        && !stash.is_changed()
        && !identity.is_changed()
        && !decisions.is_changed()
    {
        return;
    }
    let Ok(mut runner) = runner_q.single_mut() else {
        return;
    };
    push_snapshot(
        &mut runner,
        &carrying,
        &stash,
        &identity,
        &decisions,
        &game_data,
        &rack_slots,
    );
}

/// Push every mirrored variable. Called by both the per-frame
/// change-detected path and the inline start-of-dialogue path in
/// [`super::systems::apply_start_dialogue`] — one definition of
/// the variable set so the two paths can't drift.
pub(super) fn push_snapshot(
    runner: &mut DialogueRunner,
    carrying: &Carrying,
    stash: &PlayerStash,
    identity: &PlayerIdentity,
    decisions: &PlayerDecisions,
    game_data: &GameDataResource,
    rack_slots: &Query<&RackSlot>,
) {
    let storage = runner.variable_storage_mut();

    let carrying_id = carrying
        .0
        .as_ref()
        .map(|c| c.instance.def_id.as_str().to_string())
        .unwrap_or_default();
    let _ = storage.set("$carrying".into(), YarnValue::String(carrying_id));
    let _ = storage.set(
        "$credits".into(),
        YarnValue::Number(identity.credits.value() as f32),
    );
    let _ = storage.set(
        "$debt".into(),
        YarnValue::Number(identity.debt.value() as f32),
    );

    // Sum rack + carrying contents into a per-item count map.
    // The stash is added below — this block covers the two
    // storage surfaces that don't live in `PlayerStash`.
    let mut extra_counts: HashMap<Id<Item>, u32> = HashMap::new();
    for slot in rack_slots.iter() {
        if let Some(instance) = &slot.item {
            *extra_counts.entry(instance.def_id.clone()).or_default() += instance.count;
        }
    }
    if let Some(carried) = &carrying.0 {
        *extra_counts
            .entry(carried.instance.def_id.clone())
            .or_default() += carried.instance.count;
    }

    // Blanket mirror of every known item into `$<item_id>`.
    // Items not present anywhere get 0 (not "undefined") — so
    // yarn authors can write `<<if $item_medkit >= 1>>` without
    // worrying about whether the variable exists yet.
    for item_id in game_data.0.items.keys() {
        let stash_count = stash.item_count(item_id, StashScope::Any);
        let rack_count = extra_counts.get(item_id).copied().unwrap_or(0);
        let total = stash_count + rack_count;
        let name = format!("${}", item_id.as_str());
        let _ = storage.set(name, YarnValue::Number(total as f32));
    }

    // Blanket mirror of every known decision into `$<decision_id>`.
    // Unset decisions read as the empty string — yarn authors gate
    // on `<<if $decision_garrison_support == "accept">>` without
    // worrying about the "undefined" case.
    for decision_id in game_data.0.decisions.keys() {
        let value = decisions.get(decision_id).unwrap_or("").to_string();
        let name = format!("${}", decision_id.as_str());
        let _ = storage.set(name, YarnValue::String(value));
    }
}
