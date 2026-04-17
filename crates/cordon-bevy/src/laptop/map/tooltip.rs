//! Tooltip payload builders: pure data → struct helpers used by
//! both the map spawner (area info cached at spawn time) and the
//! hover system (relic + NPC tooltips built lazily on hover).
//!
//! This is all Fluent localization lookups and catalog field
//! selection — no ECS, no Commands, no queries. Keeping it
//! isolated lets the map spawner stay focused on geometry and
//! lets the hover system stay focused on cursor resolution.

use bevy::prelude::*;
use cordon_core::primitive::Tier;
use cordon_core::world::area::{AreaDef, AreaKind, SettlementRole};
use cordon_sim::plugin::prelude::{CombatTarget, MovementTarget};

use super::AreaTooltipInfo;
use crate::laptop::map::relics::RelicIconAssets;
use crate::laptop::npcs::faction_icon_str;
use crate::laptop::ui::map::TooltipContent;
use crate::locale::L10n;

fn tier_key(t: &Tier) -> &'static str {
    match t {
        Tier::VeryLow => "tier-verylow",
        Tier::Low => "tier-low",
        Tier::Medium => "tier-medium",
        Tier::High => "tier-high",
        Tier::VeryHigh => "tier-veryhigh",
    }
}

/// Build the cached tooltip payload for one area. Called once per
/// area at map spawn time, stored in [`AreaData`](super::AreaData)
/// so the hover system doesn't re-resolve localization strings
/// every frame.
pub fn build_area_info(l10n: &L10n, area: &AreaDef) -> AreaTooltipInfo {
    let tier_label = |t: Tier| -> (String, Tier) { (l10n.get(tier_key(&t)), t) };

    let kind_key = match &area.kind {
        AreaKind::Settlement { .. } => "areakind-settlement",
        AreaKind::Wasteland { .. } => "areakind-wasteland",
        AreaKind::MutantLair { .. } => "areakind-mutant-lair",
        AreaKind::AnomalyField { .. } => "areakind-anomaly-field",
        AreaKind::Anchor { .. } => "areakind-anchor",
    };

    let role = match &area.kind {
        AreaKind::Settlement { role, .. } => Some({
            let key = match role {
                SettlementRole::Outpost => "settlement-role-outpost",
                SettlementRole::Market => "settlement-role-market",
            };
            l10n.get(key)
        }),
        _ => None,
    };

    let creatures = area.kind.creatures().map(tier_label);
    let corruption = area.kind.corruption().map(tier_label);
    let loot = area.kind.loot().map(tier_label);

    AreaTooltipInfo {
        faction_icon: faction_icon_str(area.kind.faction().map(|f| f.as_str())).to_string(),
        name: l10n.get(area.id.as_str()),
        kind_label: l10n.get(kind_key),
        role,
        creatures,
        corruption,
        loot,
    }
}

/// Build a relic tooltip payload from its catalog def + relic
/// data. Called inline by the hover system — resolving strings
/// on-demand avoids duplicating them on every spawned relic
/// entity, and means localization updates apply immediately
/// without rebuilding dots.
pub fn build_relic_tooltip(
    l10n: &L10n,
    icons: &RelicIconAssets,
    def: &cordon_core::item::ItemDef,
    data: &cordon_core::item::RelicData,
) -> TooltipContent {
    use cordon_core::item::{PassiveModifier, StatTarget};
    use cordon_core::primitive::Rarity;

    let name = l10n.get(def.id.as_str());
    let rarity_key = match def.rarity {
        Rarity::Common => "rarity-common",
        Rarity::Uncommon => "rarity-uncommon",
        Rarity::Rare => "rarity-rare",
    };
    let rarity = l10n.get(rarity_key);

    let passives: Vec<String> = data
        .passive
        .iter()
        .map(|PassiveModifier { target, value }| {
            let stat_key = match target {
                StatTarget::MaxHealth => "stat-max-health",
                StatTarget::MaxStamina => "stat-max-stamina",
                StatTarget::BallisticResistance => "resistance-ballistic",
                StatTarget::CorruptionResistance => "resistance-corruption",
            };
            let label = l10n.get(stat_key);
            let sign = if *value >= 0.0 { "+" } else { "" };
            format!("{label}: {sign}{value:.0}")
        })
        .collect();

    // Look up the preloaded handle. A missing icon (def id not in
    // the preload map) falls back to a default `Handle<Image>`,
    // which renders as Bevy's missing-asset placeholder.
    let icon = icons.get(&def.id).unwrap_or_default();

    TooltipContent::Relic {
        name,
        icon,
        rarity,
        passives,
        triggered_count: data.triggered.len(),
    }
}

/// Turn per-NPC movement/combat/loot state into a human-readable
/// status string for the hover tooltip. Not localized — these
/// labels are game-system jargon, not player-facing content.
pub fn format_npc_status(
    movement: &MovementTarget,
    combat: &CombatTarget,
    looting: bool,
    goal: &cordon_core::entity::squad::Goal,
) -> String {
    let doing = if combat.0.is_some() {
        "Fighting"
    } else if looting {
        "Looting"
    } else if movement.0.is_some() {
        "Walking"
    } else {
        "Idle"
    };
    let purpose = match goal {
        cordon_core::entity::squad::Goal::Idle => "idle",
        cordon_core::entity::squad::Goal::Patrol { .. } => "patrolling",
        cordon_core::entity::squad::Goal::Scavenge { .. } => "scavenging",
        cordon_core::entity::squad::Goal::Protect { .. } => "protecting",
        cordon_core::entity::squad::Goal::Find { .. } => "hunting",
        cordon_core::entity::squad::Goal::GoTo { intent, .. } => match intent {
            cordon_core::entity::squad::TravelIntent::Returning => "returning",
            cordon_core::entity::squad::TravelIntent::Arriving => "arriving",
            cordon_core::entity::squad::TravelIntent::Fleeing => "fleeing",
            cordon_core::entity::squad::TravelIntent::Investigating => "investigating",
            cordon_core::entity::squad::TravelIntent::Generic => "traveling",
        },
    };
    format!("{doing} ({purpose})")
}
