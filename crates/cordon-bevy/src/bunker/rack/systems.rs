//! Rack interaction systems: spawn slot entities when a rack
//! appears, handle take/place/swap on interact, and manage the
//! carried-item visual attached to the camera.

use bevy::prelude::*;
use bevy_fluent::prelude::Localization;
use cordon_core::item::ItemInstance;
use cordon_core::primitive::Id;
use cordon_data::gamedata::GameDataResource;

use super::components::*;
use super::shelf_prop::ShelfProp;
use crate::bunker::components::FpsCamera;
use crate::bunker::geometry::{Prop, PropPlacement};
use crate::bunker::interaction::{Interact, Interactable};
use crate::locale::l10n_or;

/// In camera-local space: centred, below the crosshair, forward.
const CARRY_OFFSET: Vec3 = Vec3::new(0.0, -0.35, -0.55);

/// Amplitude / speed of the carry-bob when walking.
const BOB_AMPLITUDE: f32 = 0.012;
const BOB_SPEED: f32 = 5.0;

const RACK_SFX_VOLUME: f32 = 0.5;

/// Preloaded rack take/place audio handles.
#[derive(Resource)]
pub(super) struct RackSfx {
    take: Handle<AudioSource>,
    place: Handle<AudioSource>,
}

pub(super) fn load_rack_sfx(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(RackSfx {
        take: asset_server.load("audio/sfx/rack/take.ogg"),
        place: asset_server.load("audio/sfx/rack/place.ogg"),
    });
}

/// Detect rack prop entities that have been resolved by the
/// `PropPlacement` observer (they now have both `PropPlacement`
/// and `SceneRoot`) but don't yet have a `Rack` component.
pub(super) fn spawn_rack_slots(
    mut commands: Commands,
    new_racks: Query<(Entity, &PropPlacement, &Transform), (With<SceneRoot>, Without<Rack>)>,
) {
    for (rack_entity, placement, rack_transform) in &new_racks {
        if placement.kind != Prop::StorageRack01 {
            continue;
        }

        let mut slots: [Option<Entity>; SLOTS_PER_RACK] = [None; SLOTS_PER_RACK];

        for (i, &local_offset) in SLOT_OFFSETS.iter().enumerate() {
            let world_pos = rack_transform.translation
                + rack_transform.rotation * (local_offset * placement.scale);

            let slot_entity = commands
                .spawn((
                    RackSlot {
                        rack: rack_entity,
                        index: i,
                        item: None,
                        visual: None,
                    },
                    Interactable {
                        key: String::new(),
                        enabled: false,
                    },
                    Transform::from_translation(world_pos),
                    GlobalTransform::default(),
                ))
                .id();
            slots[i] = Some(slot_entity);
        }

        commands.entity(rack_entity).insert(Rack { slots });
    }
}

/// Refresh the `Interactable` prompt on each rack slot.
pub(super) fn update_slot_prompts(
    carrying: Res<Carrying>,
    mut slots: Query<(&RackSlot, &mut Interactable)>,
    localization: Option<Res<Localization>>,
) {
    for (slot, mut interactable) in &mut slots {
        let has_item = slot.item.is_some();
        let carrying_item = carrying.0.is_some();

        interactable.enabled = has_item || carrying_item;
        interactable.key = match (has_item, carrying_item) {
            (true, false) => {
                let name = slot_item_name(slot, localization.as_deref());
                format!("interact-rack-take?item={name}")
            }
            (false, true) => "interact-rack-place".into(),
            (true, true) => {
                let name = slot_item_name(slot, localization.as_deref());
                format!("interact-rack-swap?item={name}")
            }
            (false, false) => String::new(),
        };
    }
}

/// Disable non-rack interactables while the player is carrying.
/// Re-enables them when hands are empty.
pub(super) fn block_non_rack_interactions(
    carrying: Res<Carrying>,
    mut interactables: Query<&mut Interactable, Without<RackSlot>>,
) {
    if !carrying.is_changed() {
        return;
    }
    let holding = carrying.0.is_some();
    for mut i in &mut interactables {
        if holding {
            i.enabled = false;
        } else {
            // Re-enable. The owning systems (visitor button,
            // laptop, CCTV) will set their own enabled state on
            // the next frame — force-enabling here is a one-frame
            // blip that those systems immediately correct.
            i.enabled = true;
        }
    }
}

/// Handle the E-press on a rack slot: take, place, or swap.
fn on_slot_interact(
    trigger: On<Interact>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut carrying: ResMut<Carrying>,
    mut slots: Query<&mut RackSlot>,
    camera_q: Query<Entity, With<FpsCamera>>,
    sfx: Res<RackSfx>,
    game_data: Res<GameDataResource>,
) {
    let slot_entity = trigger.event().entity;
    let Ok(mut slot) = slots.get_mut(slot_entity) else {
        return;
    };

    let has_item = slot.item.is_some();
    let carrying_item = carrying.0.is_some();

    match (has_item, carrying_item) {
        (true, false) => {
            let instance = slot.item.take().unwrap();
            if let Some(vis) = slot.visual.take() {
                commands.entity(vis).despawn();
            }
            let visual =
                spawn_carried_visual(&mut commands, &mut meshes, &mut mats, &camera_q, &instance);
            carrying.0 = Some(CarriedItem { instance, visual });
            play_sfx(&mut commands, &sfx.take);
        }
        (false, true) => {
            let carried = carrying.0.take().unwrap();
            commands.entity(carried.visual).despawn();
            let vis = spawn_slot_visual(&mut commands, &game_data, slot_entity, &carried.instance);
            slot.visual = Some(vis);
            slot.item = Some(carried.instance);
            play_sfx(&mut commands, &sfx.place);
        }
        (true, true) => {
            let carried = carrying.0.take().unwrap();
            commands.entity(carried.visual).despawn();
            if let Some(vis) = slot.visual.take() {
                commands.entity(vis).despawn();
            }

            let shelf_instance = slot.item.take().unwrap();

            let new_vis =
                spawn_slot_visual(&mut commands, &game_data, slot_entity, &carried.instance);
            slot.visual = Some(new_vis);
            slot.item = Some(carried.instance);

            let new_carried_vis = spawn_carried_visual(
                &mut commands,
                &mut meshes,
                &mut mats,
                &camera_q,
                &shelf_instance,
            );
            carrying.0 = Some(CarriedItem {
                instance: shelf_instance,
                visual: new_carried_vis,
            });
            play_sfx(&mut commands, &sfx.take);
        }
        (false, false) => {}
    }
}

/// Attach the slot interact observer to newly-spawned RackSlot
/// entities.
pub(super) fn attach_slot_observers(
    mut commands: Commands,
    new_slots: Query<Entity, Added<RackSlot>>,
) {
    for entity in &new_slots {
        commands.entity(entity).observe(on_slot_interact);
    }
}

/// Gentle bob on the carried visual while the player moves.
pub(super) fn animate_carried_bob(
    carrying: Res<Carrying>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(carried) = &carrying.0 else {
        return;
    };
    let moving = keys.any_pressed([KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD]);
    let Ok(mut t) = transforms.get_mut(carried.visual) else {
        return;
    };
    if moving {
        let phase = time.elapsed_secs() * BOB_SPEED;
        t.translation.y = CARRY_OFFSET.y + phase.sin() * BOB_AMPLITUDE;
        t.translation.x = CARRY_OFFSET.x + (phase * 0.5).cos() * BOB_AMPLITUDE * 0.5;
    } else {
        t.translation.y = CARRY_OFFSET.y;
        t.translation.x = CARRY_OFFSET.x;
    }
}

/// Drop carried item with Q — spawns a slot-visual on the ground
/// (not on a rack) and clears the carry state. For now, the item
/// is lost — there's no floor-item pickup system yet.
pub(super) fn drop_carried(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut carrying: ResMut<Carrying>,
    camera_q: Query<&Transform, With<FpsCamera>>,
    sfx: Res<RackSfx>,
) {
    if !keys.just_pressed(KeyCode::KeyQ) {
        return;
    }
    let Some(carried) = carrying.0.take() else {
        return;
    };
    commands.entity(carried.visual).despawn();

    // Spawn the box on the floor in front of the player.
    if let Ok(cam) = camera_q.single() {
        let forward = cam.forward().as_vec3();
        let drop_pos = cam.translation + forward * 1.0;
        let drop_pos = Vec3::new(drop_pos.x, 0.0, drop_pos.z);
        commands.spawn(PropPlacement::new(Prop::Box01, drop_pos).no_collider());
    }

    play_sfx(&mut commands, &sfx.place);
    info!("dropped item: {}", carried.instance.def_id.as_str());
}

fn slot_item_name(slot: &RackSlot, l10n: Option<&Localization>) -> String {
    let id = slot
        .item
        .as_ref()
        .map(|i| i.def_id.as_str())
        .unwrap_or("item");
    match l10n {
        Some(l) => l10n_or(l, id, id),
        None => id.to_string(),
    }
}

fn play_sfx(commands: &mut Commands, handle: &Handle<AudioSource>) {
    commands.spawn((
        AudioPlayer(handle.clone()),
        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(RACK_SFX_VOLUME)),
    ));
}

fn spawn_carried_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    camera_q: &Query<Entity, With<FpsCamera>>,
    _instance: &ItemInstance,
) -> Entity {
    let Ok(camera) = camera_q.single() else {
        return Entity::PLACEHOLDER;
    };
    let mesh = meshes.add(Cuboid::new(0.25, 0.18, 0.25));
    let mat = mats.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.25, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });
    let child = commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(CARRY_OFFSET),
            Visibility::Visible,
        ))
        .id();
    commands.entity(camera).add_child(child);
    child
}

fn spawn_slot_visual(
    commands: &mut Commands,
    game_data: &GameDataResource,
    slot_entity: Entity,
    instance: &ItemInstance,
) -> Entity {
    let prop = game_data
        .0
        .items
        .get(&instance.def_id)
        .map(|def| def.data.category().shelf_prop())
        .unwrap_or(Prop::Box01);
    let child = commands
        .spawn((
            PropPlacement::new(prop, Vec3::ZERO).no_collider(),
            Transform::default(),
        ))
        .id();
    commands.entity(slot_entity).add_child(child);
    child
}

/// Starter items for testing.
const STARTER_ITEMS: &[&str] = &[
    "item_bandage",
    "item_bandage",
    "item_canned_food",
    "item_556_box",
];

/// One-shot system: populate empty rack slots with a few starter
/// items so the player has something to interact with.
pub(super) fn populate_starter_items(
    mut commands: Commands,
    game_data: Res<GameDataResource>,
    mut slots: Query<(Entity, &mut RackSlot)>,
    populated: Option<Res<RacksPopulated>>,
) {
    if populated.is_some() {
        return;
    }
    if slots.is_empty() {
        return;
    }

    let data = &game_data.0;
    let mut placed = 0;

    for &item_id in STARTER_ITEMS {
        let id = Id::new(item_id);
        let Some(def) = data.items.get(&id) else {
            warn!("starter item not found: {item_id}");
            continue;
        };

        let instance = ItemInstance::new(def);

        let Some((slot_entity, mut slot)) = slots.iter_mut().find(|(_, s)| s.item.is_none()) else {
            break;
        };

        let vis = spawn_slot_visual(&mut commands, &game_data, slot_entity, &instance);
        slot.item = Some(instance);
        slot.visual = Some(vis);
        placed += 1;
    }

    info!("populated {placed} starter items on rack slots");
    commands.insert_resource(RacksPopulated);
}

/// Flag so [`populate_starter_items`] only runs once.
#[derive(Resource)]
pub(super) struct RacksPopulated;

/// Drain items from `PlayerState.pending_items` onto the first
/// available rack slots. Quest consequences push items there;
/// this system moves them into the physical world each frame.
pub(super) fn drain_pending_to_racks(
    mut commands: Commands,
    game_data: Res<GameDataResource>,
    mut stash: ResMut<cordon_sim::resources::PlayerStash>,
    mut slots: Query<(Entity, &mut RackSlot)>,
) {
    while !stash.pending_items.is_empty() {
        let Some((slot_entity, mut slot)) = slots.iter_mut().find(|(_, s)| s.item.is_none()) else {
            break;
        };
        let Some(instance) = stash.pending_items.remove(0) else {
            break;
        };
        let vis = spawn_slot_visual(&mut commands, &game_data, slot_entity, &instance);
        slot.item = Some(instance);
        slot.visual = Some(vis);
    }
}
