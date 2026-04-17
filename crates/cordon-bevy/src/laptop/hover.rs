//! Cursor → hover target resolution + tooltip population.
//!
//! The hover system picks the single entity under the cursor
//! (priority: relic > NPC > area) and writes a matching
//! `TooltipContent`. A `Local<HoverTarget>` memoises the previous
//! frame's pick, so stationary cursor frames early-out without
//! touching `TooltipContent`, `ColorMaterial`, or any other
//! change-detected resource.
//!
//! Tooltip content builders are in `laptop::map::tooltip`; this
//! module just picks the target and calls them on transitions.

use bevy::prelude::*;
use cordon_core::entity::squad::Goal;
use cordon_core::item::{ItemData, ItemInstance};
use cordon_data::gamedata::GameDataResource;
use cordon_sim::plugin::prelude::{
    CombatTarget, LootState, MovementTarget, NpcMarker, RelicMarker, SquadMembership,
};

use crate::PlayingState;
use crate::laptop::LaptopCamera;
use crate::laptop::map::relics::RelicIconAssets;
use crate::laptop::map::tooltip::{build_relic_tooltip, format_npc_status};
use crate::laptop::map::{AreaCircle, AreaData, COLOR_AREA_HOVER};
use crate::laptop::npcs::NpcDotInfo;
use crate::laptop::ui::map::{TooltipContent, cursor_world_pos};
use crate::locale::L10n;

pub struct HoverPlugin;

impl Plugin for HoverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_hover.run_if(in_state(PlayingState::Laptop)));
    }
}

/// What the hover system currently believes the cursor is
/// pointing at. Persisted in a [`Local`] so we can detect
/// *changes* and only mutate `ColorMaterial.color`,
/// `TooltipContent`, etc. when the target actually changes.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum HoverTarget {
    #[default]
    None,
    Area(Entity),
    Npc(Entity),
    Relic(Entity),
}

fn update_hover(
    mut last_target: Local<HoverTarget>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<LaptopCamera>>,
    cam_proj: Query<&Projection, With<LaptopCamera>>,
    game_data: Res<GameDataResource>,
    relic_icons: Option<Res<RelicIconAssets>>,
    l10n: L10n,
    areas: Query<(
        Entity,
        &AreaCircle,
        &AreaData,
        &Transform,
        &MeshMaterial2d<ColorMaterial>,
        &Visibility,
    )>,
    npcs: Query<
        (
            Entity,
            &NpcDotInfo,
            &Transform,
            &MovementTarget,
            &CombatTarget,
            &SquadMembership,
            Option<&LootState>,
            &Visibility,
        ),
        With<NpcMarker>,
    >,
    relics: Query<(Entity, &ItemInstance, &Transform, &Visibility), With<RelicMarker>>,
    squad_goals: Query<&Goal>,
    mut mats: ResMut<Assets<ColorMaterial>>,
    mut tooltip: ResMut<TooltipContent>,
) {
    let Some(cursor) = cursor_world_pos(&windows, &cameras) else {
        *tooltip = TooltipContent::Hidden;
        return;
    };
    let scale = cam_proj
        .iter()
        .next()
        .and_then(|p| match p {
            Projection::Orthographic(o) => Some(o.scale),
            _ => None,
        })
        .unwrap_or(1.0);
    let npc_hit = 20.0 * scale;
    let relic_hit = 12.0 * scale;

    // Find the new hover target. Priority is relic > NPC > area,
    // so relics tucked inside dense anomaly zones stay
    // tooltip-reachable even when NPCs cross the same pixel.
    let mut new_target = HoverTarget::None;

    let mut closest_relic: Option<(Entity, f32)> = None;
    for (entity, _item, transform, vis) in &relics {
        if matches!(vis, Visibility::Hidden) {
            continue;
        }
        let dist = cursor.distance(transform.translation.truncate());
        if dist < relic_hit && closest_relic.is_none_or(|(_, d)| dist < d) {
            closest_relic = Some((entity, dist));
        }
    }
    if let Some((entity, _)) = closest_relic {
        new_target = HoverTarget::Relic(entity);
    }

    if matches!(new_target, HoverTarget::None) {
        let mut closest_npc: Option<(Entity, f32)> = None;
        for (entity, _info, transform, _mvt, _cmb, _mem, _loot, vis) in &npcs {
            if matches!(vis, Visibility::Hidden) {
                continue;
            }
            let dist = cursor.distance(transform.translation.truncate());
            if dist < npc_hit && closest_npc.is_none_or(|(_, d)| dist < d) {
                closest_npc = Some((entity, dist));
            }
        }
        if let Some((entity, _)) = closest_npc {
            new_target = HoverTarget::Npc(entity);
        }
    }

    if matches!(new_target, HoverTarget::None) {
        for (entity, circle, _data, transform, _mat, vis) in &areas {
            if matches!(vis, Visibility::Hidden) {
                continue;
            }
            let dist = cursor.distance(transform.translation.truncate());
            if dist < circle.radius {
                new_target = HoverTarget::Area(entity);
                break;
            }
        }
    }

    // Early out: target unchanged → don't touch the tooltip or
    // any materials. Stationary-cursor hot path.
    if *last_target == new_target {
        return;
    }

    // Target changed. Repaint the *old* hovered area back to its
    // base colour (if the previous target was an area), and
    // paint the *new* area with the hover colour (if the new
    // target is an area). All other areas keep whatever colour
    // they already had — no mass rewrite.
    if let HoverTarget::Area(prev) = *last_target
        && let Ok((_, circle, _, _, mat_handle, _)) = areas.get(prev)
        && let Some(m) = mats.get_mut(&mat_handle.0)
    {
        m.color = circle.base_color;
    }
    if let HoverTarget::Area(curr) = new_target
        && let Ok((_, _, _, _, mat_handle, _)) = areas.get(curr)
        && let Some(m) = mats.get_mut(&mat_handle.0)
    {
        m.color = COLOR_AREA_HOVER;
    }

    // Build the new tooltip content. This runs only on the
    // transition frame, so the string allocations here are cheap.
    *tooltip = match new_target {
        HoverTarget::None => TooltipContent::Hidden,
        HoverTarget::Relic(entity) => {
            let mut out = TooltipContent::Hidden;
            if let Ok((_, item, _, _)) = relics.get(entity)
                && let Some(def) = game_data.0.items.get(&item.def_id)
                && let ItemData::Relic(relic_data) = &def.data
                && let Some(icons) = relic_icons.as_deref()
            {
                out = build_relic_tooltip(&l10n, icons, def, relic_data);
            }
            out
        }
        HoverTarget::Npc(entity) => {
            let mut out = TooltipContent::Hidden;
            if let Ok((_, info, _, movement, combat, member, loot, _)) = npcs.get(entity) {
                let goal = squad_goals.get(member.squad).cloned().unwrap_or(Goal::Idle);
                out = TooltipContent::Npc {
                    faction_icon: info.faction_icon.clone(),
                    name: info.name.clone(),
                    faction: info.faction.clone(),
                    rank: info.rank.clone(),
                    status: format_npc_status(movement, combat, loot.is_some(), &goal),
                };
            }
            out
        }
        HoverTarget::Area(entity) => {
            let mut out = TooltipContent::Hidden;
            if let Ok((_, _, data, _, _, _)) = areas.get(entity) {
                let i = &data.0;
                out = TooltipContent::Area {
                    faction_icon: i.faction_icon.clone(),
                    name: i.name.clone(),
                    kind_label: i.kind_label.clone(),
                    role: i.role.clone(),
                    creatures: i.creatures.clone(),
                    corruption: i.corruption.clone(),
                    loot: i.loot.clone(),
                };
            }
            out
        }
    };

    *last_target = new_target;
}
