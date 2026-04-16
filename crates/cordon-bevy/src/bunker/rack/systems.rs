//! Rack interaction systems: spawn slot entities when a rack
//! appears, handle take/place/swap on interact, and manage the
//! carried-item visual attached to the camera.

use bevy::prelude::*;
use cordon_core::item::ItemInstance;
use cordon_core::primitive::Id;
use cordon_data::gamedata::GameDataResource;

use super::components::*;
use crate::bunker::components::FpsCamera;
use crate::bunker::geometry::{Prop, PropPlacement};
use crate::bunker::interaction::{Interact, Interactable};

/// Local offset of the carried box relative to the camera.
/// Pushed well below and forward so it reads as "held at waist
/// level in front of the player" — not glued to the face.
const CARRY_OFFSET: Vec3 = Vec3::new(0.3, -0.65, -0.7);

/// Detect rack prop entities that have been resolved by the
/// `PropPlacement` observer (they now have both `PropPlacement`
/// and `SceneRoot`) but don't yet have a `Rack` component. This
/// ensures the observer has already set the correct `Transform`
/// so slot world positions are accurate.
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
                        prompt: String::new(),
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

/// Refresh the `Interactable` prompt on each rack slot based on
/// whether the slot is occupied and whether the player is carrying.
pub(super) fn update_slot_prompts(
    carrying: Res<Carrying>,
    mut slots: Query<(&RackSlot, &mut Interactable)>,
) {
    for (slot, mut interactable) in &mut slots {
        let has_item = slot.item.is_some();
        let carrying_item = carrying.0.is_some();

        interactable.enabled = has_item || carrying_item;
        interactable.prompt = match (has_item, carrying_item) {
            (true, false) => format!("[E] Take {}", slot_item_name(slot)),
            (false, true) => "[E] Place item".into(),
            (true, true) => format!("[E] Swap with {}", slot_item_name(slot)),
            (false, false) => String::new(),
        };
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
        }
        (false, true) => {
            let carried = carrying.0.take().unwrap();
            commands.entity(carried.visual).despawn();
            let vis = spawn_slot_visual(&mut commands, slot_entity, &carried.instance);
            slot.visual = Some(vis);
            slot.item = Some(carried.instance);
        }
        (true, true) => {
            let carried = carrying.0.take().unwrap();
            commands.entity(carried.visual).despawn();
            if let Some(vis) = slot.visual.take() {
                commands.entity(vis).despawn();
            }

            let shelf_instance = slot.item.take().unwrap();

            let new_vis = spawn_slot_visual(&mut commands, slot_entity, &carried.instance);
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

fn slot_item_name(slot: &RackSlot) -> &str {
    slot.item
        .as_ref()
        .map(|i| i.def_id.as_str())
        .unwrap_or("item")
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
    // Simple placeholder cube in camera-local space. Using a raw
    // mesh (not a GLB scene) avoids async-load timing issues and
    // the PropPlacement observer overriding the local offset.
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
        ))
        .id();
    commands.entity(camera).add_child(child);
    child
}

fn spawn_slot_visual(
    commands: &mut Commands,
    slot_entity: Entity,
    _instance: &ItemInstance,
) -> Entity {
    let child = commands
        .spawn((
            PropPlacement::new(Prop::Box01, Vec3::ZERO).no_collider(),
            Transform::default(),
        ))
        .id();
    commands.entity(slot_entity).add_child(child);
    child
}

/// Starter items for testing. Lists `(item_id, slot_count)` pairs
/// — each item gets placed on the next available rack slot.
const STARTER_ITEMS: &[&str] = &[
    "item_bandage",
    "item_bandage",
    "item_canned_food",
    "item_556_box",
];

/// One-shot system: populate empty rack slots with a few starter
/// items so the player has something to interact with. Runs once
/// by inserting a flag resource after filling.
pub(super) fn populate_starter_items(
    mut commands: Commands,
    game_data: Res<GameDataResource>,
    mut slots: Query<(Entity, &mut RackSlot)>,
    populated: Option<Res<RacksPopulated>>,
) {
    if populated.is_some() {
        return;
    }
    // Wait until at least one slot exists.
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

        // Find the next empty slot.
        let Some((slot_entity, mut slot)) = slots.iter_mut().find(|(_, s)| s.item.is_none()) else {
            break;
        };

        let vis = spawn_slot_visual(&mut commands, slot_entity, &instance);
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
