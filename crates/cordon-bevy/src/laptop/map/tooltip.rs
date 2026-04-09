//! Tooltip payload builders: pure data → struct helpers used by
//! both the map spawner (area info cached at spawn time) and the
//! hover system (relic + NPC tooltips built lazily on hover).
//!
//! This is all Fluent localization lookups and catalog field
//! selection — no ECS, no Commands, no queries. Keeping it
//! isolated lets the map spawner stay focused on geometry and
//! lets the hover system stay focused on cursor resolution.

use bevy::prelude::*;
use bevy_fluent::prelude::*;
use cordon_core::primitive::{HazardType, Tier};
use cordon_core::world::area::{AreaDef, AreaKind, SettlementRole};
use cordon_sim::behavior::{CombatTarget, MovementTarget};

use super::AreaTooltipInfo;
use crate::laptop::map::relics::RelicIconAssets;
use crate::laptop::npcs::faction_icon_str;
use crate::laptop::ui::map::TooltipContent;
use crate::locale::l10n_or;

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
pub fn build_area_info(l10n: &Localization, area: &AreaDef) -> AreaTooltipInfo {
    let tier_label =
        |t: Tier| -> (String, Tier) { (l10n_or(l10n, tier_key(&t), &format!("{:?}", t)), t) };
    let hazard_image = |h: &cordon_core::world::area::Hazard| -> String {
        match h.kind {
            HazardType::Chemical => "icons/hazards/chemical.png".to_string(),
            HazardType::Thermal => "icons/hazards/thermal.png".to_string(),
            HazardType::Electric => "icons/hazards/electric.png".to_string(),
            HazardType::Gravitational => "icons/hazards/gravitational.png".to_string(),
        }
    };
    let hazard_count = |t: Tier| -> u8 {
        match t {
            Tier::VeryLow => 1,
            Tier::Low => 2,
            Tier::Medium => 3,
            Tier::High => 4,
            Tier::VeryHigh => 5,
        }
    };

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
            l10n_or(l10n, key, key)
        }),
        _ => None,
    };

    let creatures = area.kind.creatures().map(tier_label);
    let radiation = area.kind.radiation().map(tier_label);
    let loot = area.kind.loot().map(tier_label);
    let (hazard_image, hazard_count_v) = match area.kind.hazard() {
        Some(h) => (Some(hazard_image(&h)), hazard_count(h.intensity)),
        None => (None, 0),
    };

    AreaTooltipInfo {
        faction_icon: faction_icon_str(area.kind.faction().map(|f| f.as_str())).to_string(),
        name: l10n_or(
            l10n,
            &format!("area-{}", area.id.as_str()),
            area.id.as_str(),
        ),
        kind_label: l10n_or(l10n, kind_key, kind_key),
        role,
        creatures,
        radiation,
        hazard_image,
        hazard_count: hazard_count_v,
        loot,
    }
}

/// Build a relic tooltip payload from its catalog def + relic
/// data. Called inline by the hover system — resolving strings
/// on-demand avoids duplicating them on every spawned relic
/// entity, and means localization updates apply immediately
/// without rebuilding dots.
pub fn build_relic_tooltip(
    l10n: &Localization,
    icons: &RelicIconAssets,
    def: &cordon_core::item::ItemDef,
    data: &cordon_core::item::RelicData,
) -> TooltipContent {
    use cordon_core::item::{PassiveModifier, StatTarget};
    use cordon_core::primitive::{HazardType, Rarity};

    let name = l10n_or(l10n, &format!("item-{}", def.id.as_str()), def.id.as_str());
    let origin_key = match data.origin {
        HazardType::Chemical => "hazard-chemical",
        HazardType::Thermal => "hazard-thermal",
        HazardType::Electric => "hazard-electric",
        HazardType::Gravitational => "hazard-gravitational",
    };
    let origin = l10n_or(l10n, origin_key, origin_key);
    let rarity_key = match def.rarity {
        Rarity::Common => "rarity-common",
        Rarity::Uncommon => "rarity-uncommon",
        Rarity::Rare => "rarity-rare",
    };
    let rarity = l10n_or(l10n, rarity_key, rarity_key);

    let passives: Vec<String> = data
        .passive
        .iter()
        .map(|PassiveModifier { target, value }| {
            let (stat_key, fallback) = match target {
                StatTarget::MaxHealth => ("stat-max-health", "Max HP"),
                StatTarget::MaxStamina => ("stat-max-stamina", "Max Stamina"),
                StatTarget::MaxHunger => ("stat-max-hunger", "Max Hunger"),
                StatTarget::BallisticResistance => ("hazard-ballistic", "Ballistic"),
                StatTarget::RadiationResistance => ("hazard-radiation", "Radiation"),
                StatTarget::ChemicalResistance => ("hazard-chemical", "Chemical"),
                StatTarget::ThermalResistance => ("hazard-thermal", "Thermal"),
                StatTarget::ElectricResistance => ("hazard-electric", "Electric"),
                StatTarget::GravitationalResistance => ("hazard-gravitational", "Gravitational"),
            };
            let label = l10n_or(l10n, stat_key, fallback);
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
        origin,
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
        cordon_core::entity::squad::Goal::Deliver { .. } => "delivering",
    };
    format!("{doing} ({purpose})")
}
