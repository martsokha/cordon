//! World relic entity components.
//!
//! A relic on the map is a Bevy entity with a [`RelicMarker`]
//! plus an [`ItemInstance`] carrying the concrete item. When a
//! squad walks over it, the pickup system transfers the instance
//! into the leader's loadout and despawns the entity.
//!
//! `ItemInstance` derives `Component` directly in cordon-core,
//! so there's no wrapper here — the raw item is the component.

use bevy::prelude::*;
use cordon_core::primitive::Id;
use cordon_core::world::area::Area;

/// Marker that this entity is an uncollected relic sitting in
/// the world.
#[derive(Component, Debug, Clone, Copy)]
pub struct RelicMarker;

/// Which anomaly area this relic was spawned inside. Used by the
/// spawn system to enforce per-area carrying capacity without a
/// full spatial query.
///
/// The anchor is logical (by id), not spatial: if an area's
/// radius ever shrinks at runtime, relics spawned inside the
/// old radius stay anchored to that area even if their transform
/// is no longer within the new disk. Areas don't currently
/// resize, so this is a latent concern, not a bug.
#[derive(Component, Debug, Clone)]
pub struct RelicHome(pub Id<Area>);
