//! NPC selection + click handling.
//!
//! Clicking an NPC dot on the map toggles it as the selected NPC.
//! The on-map visual is an outline ring child entity that gets
//! spawned around the selected NPC (thick yellow) and their
//! squadmates (thin yellow) — the underlying dot keeps its
//! faction colour. Pressing Esc clears the selection; pressing E
//! or Esc again exits back to the bunker.

use bevy::prelude::*;
use cordon_sim::plugin::prelude::{NpcMarker, SquadMembership};

use super::NpcAssets;
use crate::PlayingState;
use crate::laptop::LaptopCamera;
use crate::laptop::input::CameraTarget;
use crate::laptop::ui::map::cursor_world_pos;

/// Currently-selected NPC entity. `None` means nothing is
/// selected. Owned by the npcs subsystem; exported through
/// `laptop::npcs` for the few other systems that need to read
/// it (the roster widget in `ui::map`).
#[derive(Resource, Default)]
pub struct SelectedNpc(pub Option<Entity>);

/// Marker for the ring child entity that draws the selection /
/// squadmate outline around an NPC dot. Keeping it as its own
/// component lets the selection system find and despawn the ring
/// without touching anything else parented to the NPC.
#[derive(Component)]
pub(super) struct SelectionRing;

pub(super) fn handle_npc_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<LaptopCamera>>,
    interactions: Query<&Interaction>,
    dots: Query<(Entity, &Transform, &Visibility), With<NpcMarker>>,
    mut selected: ResMut<SelectedNpc>,
    mut camera_target: ResMut<CameraTarget>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // Swallow the click when it's on a UI element so clicking
    // a tab-bar button or a roster slot doesn't leak through
    // to deselect/refollow an NPC. Any interaction reporting
    // Pressed or Hovered this frame means the cursor is over
    // pickable UI.
    if interactions
        .iter()
        .any(|i| matches!(i, Interaction::Pressed | Interaction::Hovered))
    {
        return;
    }
    let Some(cursor_world) = cursor_world_pos(&windows, &cameras) else {
        return;
    };

    let hit_radius = 20.0;
    let mut closest: Option<(Entity, f32)> = None;
    for (entity, transform, vis) in &dots {
        // Fog-hidden NPCs aren't clickable.
        if matches!(vis, Visibility::Hidden) {
            continue;
        }
        let pos = transform.translation.truncate();
        let dist = pos.distance(cursor_world);
        if dist <= hit_radius && (closest.is_none() || dist < closest.unwrap().1) {
            closest = Some((entity, dist));
        }
    }

    // Clicking an NPC selects and follows it; clicking the
    // same NPC a second time clears both. Clicking empty
    // space is a no-op — an active follow is only broken by
    // explicit camera movement (WASD / drag / edge-scroll),
    // not by a miss. This matches the expectation "I'm
    // watching Ivan, don't unlock until I pan away."
    match closest {
        Some((entity, _)) if selected.0 == Some(entity) => {
            selected.0 = None;
            camera_target.following = None;
        }
        Some((entity, _)) => {
            selected.0 = Some(entity);
            camera_target.following = Some(entity);
        }
        None => {}
    }
}

pub(super) fn update_npc_selection(
    selected: Res<SelectedNpc>,
    npc_assets: Res<NpcAssets>,
    mut commands: Commands,
    dots: Query<(Entity, &SquadMembership, Option<&Children>), With<NpcMarker>>,
    rings: Query<Entity, With<SelectionRing>>,
) {
    if !selected.is_changed() {
        return;
    }

    // Despawn all existing rings first. Rings are children of
    // their NPC, so despawning the ring entity is enough — the
    // NPC stays.
    for ring in &rings {
        commands.entity(ring).despawn();
    }

    // Nothing selected → nothing to draw.
    let Some(selected_entity) = selected.0 else {
        return;
    };

    // Find the selected NPC's squad so we can mark its squadmates.
    let Some(selected_squad) = dots
        .iter()
        .find(|(e, _, _)| *e == selected_entity)
        .map(|(_, m, _)| m.squad)
    else {
        return;
    };

    // Spawn a ring under each matching NPC. Focused NPC gets the
    // thicker "selected" ring; squadmates get the thinner one.
    // Rings sit at local z = 0.5 so they render just above the
    // dot (at 0) but below any later overlay.
    for (entity, member, _) in &dots {
        let (mesh, mat) = if entity == selected_entity {
            (
                npc_assets.selected_ring_mesh.clone(),
                npc_assets.selected_ring_mat.clone(),
            )
        } else if member.squad == selected_squad {
            (
                npc_assets.squad_ring_mesh.clone(),
                npc_assets.squad_ring_mat.clone(),
            )
        } else {
            continue;
        };
        let ring = commands
            .spawn((
                SelectionRing,
                Mesh2d(mesh),
                MeshMaterial2d(mat),
                Transform::from_xyz(0.0, 0.0, 0.5),
            ))
            .id();
        commands.entity(entity).add_child(ring);
    }
}

pub(super) fn deselect_or_exit(
    keys: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedNpc>,
    mut next_state: ResMut<NextState<PlayingState>>,
) {
    if keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::Escape) {
        if selected.0.is_some() && keys.just_pressed(KeyCode::Escape) {
            selected.0 = None;
        } else {
            *next_state = NextState::Pending(PlayingState::Bunker);
        }
    }
}
