//! Top-level Bevy resources owned by `cordon-sim`.
//!
//! These will be split further as the `World` struct is dissolved into
//! per-concern resources, but for now [`SimWorld`] is the single
//! container Bevy systems reach into for game time, RNG, and the
//! mutable world state that hasn't moved into ECS yet.

use std::collections::HashMap;

use bevy::prelude::*;
use cordon_core::entity::squad::Squad;
use cordon_core::primitive::Uid;

use crate::world::state::World;

/// Resource wrapping the simulation [`World`].
#[derive(Resource)]
pub struct SimWorld(pub World);

/// Maps stable squad uids to their current ECS entity. Maintained by
/// the spawn system and used by AI systems for the rare uid → entity
/// lookups (e.g. resolving `Goal::Protect { other }`).
#[derive(Resource, Default, Debug, Clone)]
pub struct SquadIdIndex(pub HashMap<Uid<Squad>, Entity>);
