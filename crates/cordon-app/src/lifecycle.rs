//! Run-lifecycle plumbing built on Bevy's state-scoped entities.
//!
//! Everything spawned during a run (NPCs, squads, relics, items on
//! racks, upgrade-conditional props, visitor sprites, toasts) is
//! tagged `DespawnOnExit(AppState::Playing)`. When the player
//! leaves Playing — dies into Ending, hits Main Menu from pause —
//! the state-scoped system despawns every tagged entity.
//!
//! cordon-sim spawns NPCs / squads / relics without knowing about
//! `AppState`. Observers here add the despawn tag from outside so
//! those entities participate in the cleanup without the sim
//! importing view-layer types.
//!
//! On `OnEnter(AppState::Playing)` we reseed the world resources
//! and reset the FPS camera. There is no separate `ResetRun`
//! message — state transitions *are* the reset.

use bevy::prelude::*;
use cordon_sim::plugin::SimActive;
use cordon_sim::plugin::prelude::{NpcMarker, RelicMarker, SquadMarker};
use cordon_sim::resources::init_world_resources;

use crate::AppState;
use crate::bunker::resources::{CameraMode, Layout, StopDialogue};
use crate::bunker::{FpsCamera, VisitorQueue};
use crate::quest::DialogueInFlight;

/// Player eye height — must match `bunker::input::controller::CAMERA_EYE_Y`.
/// Duplicated here because that constant's module is private; a
/// dedicated `camera_start_transform()` helper on the bunker
/// would be nicer but overkill for one reset point.
const CAMERA_EYE_Y: f32 = 1.65;

pub struct LifecyclePlugin;

impl Plugin for LifecyclePlugin {
    fn build(&self, app: &mut App) {
        // Seed sim resources the moment the catalog is loaded. The
        // bunker scene and laptop UI both read sim resources while
        // the player is in `AppState::Menu` (scene is a backdrop,
        // laptop renders under the pause overlay), so they need to
        // exist before Menu is ever entered.
        app.add_systems(OnExit(AppState::Loading), run_init_world_resources);
        // Enter Playing: reseed sim state so a New Game erases any
        // accumulated run state. The state-scoped despawn from the
        // previous Playing exit already cleared the old entities,
        // so this is genuinely a fresh start.
        app.add_systems(
            OnEnter(AppState::Playing),
            (
                begin_dialogue,
                reset_camera,
                crate::bunker::rack::reset_rack_state,
                crate::bunker::toast::reset_toast_queue,
                crate::bunker::reset_visitor_state,
                run_init_world_resources,
                start_sim,
            )
                .chain(),
        );
        // Exit Playing: stop any in-flight dialogue and park the sim
        // so NPC spawners / BT tick systems stop running. Entities
        // with `DespawnOnExit(AppState::Playing)` are handled by Bevy.
        app.add_systems(OnExit(AppState::Playing), (end_dialogue, stop_sim));
        // Auto-tag sim-layer entities so they get cleaned up on exit.
        // Observers keep cordon-sim free of AppState knowledge.
        app.add_observer(tag_npc_on_spawn);
        app.add_observer(tag_squad_on_spawn);
        app.add_observer(tag_relic_on_spawn);
    }
}

fn begin_dialogue(
    mut in_flight: ResMut<DialogueInFlight>,
    mut visitor_queue: ResMut<VisitorQueue>,
) {
    in_flight.0 = None;
    visitor_queue.0.clear();
}

fn end_dialogue(mut stop_dialogue: MessageWriter<StopDialogue>) {
    stop_dialogue.write(StopDialogue);
}

fn start_sim(mut commands: Commands) {
    commands.insert_resource(SimActive);
}

fn stop_sim(mut commands: Commands) {
    commands.remove_resource::<SimActive>();
}

fn reset_camera(
    mut camera_q: Query<&mut Transform, With<FpsCamera>>,
    mut camera_mode: ResMut<CameraMode>,
) {
    // Matches the spawn position in `bunker/systems.rs`: just south
    // of the desk, looking toward the front of the bunker.
    let layout = Layout::new();
    let start = Vec3::new(0.0, CAMERA_EYE_Y, layout.desk_z() - 0.5);
    let target = Vec3::new(0.0, 1.2, layout.front_z);
    if let Ok(mut t) = camera_q.single_mut() {
        *t = Transform::from_translation(start).looking_at(target, Vec3::Y);
    }
    *camera_mode = CameraMode::Free;
}

fn run_init_world_resources(world: &mut World) {
    if let Err(err) = world.run_system_cached(init_world_resources) {
        warn!("begin_run: init_world_resources failed: {err}");
    }
}

fn tag_npc_on_spawn(trigger: On<Add, NpcMarker>, mut commands: Commands) {
    commands
        .entity(trigger.event().entity)
        .insert(DespawnOnExit(AppState::Playing));
}

fn tag_squad_on_spawn(trigger: On<Add, SquadMarker>, mut commands: Commands) {
    commands
        .entity(trigger.event().entity)
        .insert(DespawnOnExit(AppState::Playing));
}

fn tag_relic_on_spawn(trigger: On<Add, RelicMarker>, mut commands: Commands) {
    commands
        .entity(trigger.event().entity)
        .insert(DespawnOnExit(AppState::Playing));
}
