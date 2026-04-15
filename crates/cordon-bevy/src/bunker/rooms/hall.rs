//! The straight-hall segment between T1 and T2. Wall space
//! along both sides is reserved for storage racks, populated as
//! the player installs the `racks` / `racks2` upgrades:
//!
//! - No rack upgrade → hall is empty.
//! - `upgrade_racks` installed → 2 racks (north pair).
//! - `upgrade_racks` + `upgrade_racks2` → 4 racks (both pairs).
//!
//! [`spawn`] handles the initial bunker build; it seeds whichever
//! tiers the player already has. [`sync_hall_racks`] reacts to
//! later installs by spawning the missing tier's pair.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use cordon_core::entity::bunker::Upgrade;
use cordon_core::primitive::Id;
use cordon_sim::plugin::prelude::Player;

use crate::bunker::geometry::*;
use crate::bunker::resources::{Layout, RoomCtx};

/// Half the centre-to-centre spacing between the two racks on a
/// wall when both rack upgrades are installed.
const RACK_OFFSET: f32 = 0.58;

/// Distance from the wall to the rack's lateral centre. Matches
/// the armory's 0.6 m inset for consistency.
const WALL_INSET: f32 = 0.6;

/// Which rack tier an entity belongs to, so [`sync_hall_racks`]
/// knows what's already present and what to spawn.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HallRackTier {
    /// Pair added by `upgrade_racks`.
    North,
    /// Pair added by `upgrade_racks2`.
    South,
}

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let has_racks = ctx.player.has_upgrade(&Id::<Upgrade>::new("upgrade_racks"));
    let has_racks2 = ctx
        .player
        .has_upgrade(&Id::<Upgrade>::new("upgrade_racks2"));

    if has_racks {
        spawn_pair(ctx.commands, ctx.l, HallRackTier::North);
    }
    if has_racks2 {
        spawn_pair(ctx.commands, ctx.l, HallRackTier::South);
    }
}

fn pair_offset(tier: HallRackTier) -> f32 {
    match tier {
        HallRackTier::North => RACK_OFFSET,
        HallRackTier::South => -RACK_OFFSET,
    }
}

/// Spawn one rack pair (left + right wall) and tag each with its
/// [`HallRackTier`]. The pair's `PropPlacement` components are
/// resolved asynchronously by the observer in `geometry`.
fn spawn_pair(commands: &mut Commands, l: &Layout, tier: HallRackTier) {
    let hall_cz = (l.tj2_north + l.tj1_south) / 2.0;
    let z = hall_cz + pair_offset(tier);
    for side in [-1.0, 1.0] {
        let x = side * (l.hw - WALL_INSET);
        let rot = Quat::from_rotation_y(-side * FRAC_PI_2);
        commands.spawn((
            PropPlacement::new(Prop::StorageRack01, Vec3::new(x, 0.0, z)).rotated(rot),
            tier,
        ));
    }
}

/// Watch the live player state and spawn rack pairs that aren't
/// already present. Lets rack upgrades installed *after* the
/// bunker was first built appear without a full respawn.
pub fn sync_hall_racks(
    mut commands: Commands,
    player: Res<Player>,
    existing: Query<&HallRackTier>,
) {
    if !player.is_changed() {
        return;
    }
    let l = Layout::new();

    let has_north_already = existing.iter().any(|t| *t == HallRackTier::North);
    let has_south_already = existing.iter().any(|t| *t == HallRackTier::South);

    if player.0.has_upgrade(&Id::<Upgrade>::new("upgrade_racks")) && !has_north_already {
        spawn_pair(&mut commands, &l, HallRackTier::North);
    }
    if player.0.has_upgrade(&Id::<Upgrade>::new("upgrade_racks2")) && !has_south_already {
        spawn_pair(&mut commands, &l, HallRackTier::South);
    }
}
