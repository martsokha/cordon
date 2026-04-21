//! Rack storage components: the rack itself, per-slot markers,
//! and the player's carried-item state.

use bevy::prelude::*;
use cordon_core::item::ItemInstance;

/// Number of shelves (slots) on a single StorageRack01 prop.
pub const SLOTS_PER_RACK: usize = 3;

/// Local-space Y offsets for each shelf surface, measured from
/// the rack's feet-centre origin. Eyeballed from StorageRack01's
/// AABB (0 → 2.55 m tall, 3 shelves roughly evenly spaced).
pub const SLOT_OFFSETS: [Vec3; SLOTS_PER_RACK] = [
    Vec3::new(0.0, 0.65, 0.0),
    Vec3::new(0.0, 1.30, 0.0),
    Vec3::new(0.0, 1.85, 0.0),
];

/// Attached to the rack prop entity. Tracks which slot entities
/// belong to this rack.
#[derive(Component)]
#[allow(dead_code)]
pub struct Rack {
    pub slots: [Option<Entity>; SLOTS_PER_RACK],
}

/// One interaction point on a rack shelf. Spawned as a child of
/// the rack entity at the shelf's world position. Carries the
/// item (if occupied) and an `Interactable` for the prompt.
#[derive(Component)]
pub struct RackSlot {
    #[allow(dead_code)]
    pub rack: Entity,
    #[allow(dead_code)]
    pub index: usize,
    pub item: Option<ItemInstance>,
    /// The visual prop entity for this slot's item, if any.
    pub visual: Option<Entity>,
}

/// What the player is currently carrying. None = empty hands.
#[derive(Resource, Default)]
pub struct Carrying(pub Option<CarriedItem>);

pub struct CarriedItem {
    pub instance: ItemInstance,
    /// The visual prop entity parented to the FPS camera.
    pub visual: Entity,
}
