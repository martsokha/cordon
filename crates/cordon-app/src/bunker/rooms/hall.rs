//! The straight-hall segment between T1 and T2. Wall space
//! along both sides is reserved for storage racks, populated from
//! the `HallRackPair` upgrade effect:
//!
//! - 0 pairs declared → hall is empty.
//! - 1 pair declared → 2 racks (north pair).
//! - 2 pairs declared → 4 racks (both pairs).
//!
//! Which specific upgrades grant `HallRackPair` is a data
//! concern — the room code just counts occurrences and spawns
//! pairs by index.
//!
//! [`spawn`] handles the initial bunker build; it seeds whichever
//! tiers the player already has. [`sync_hall_racks`] reacts to
//! later installs by spawning the missing tier's pair.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use cordon_core::entity::bunker::UpgradeEffect;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::resources::PlayerUpgrades;

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
///
/// Slot assignment is index-based: the first installed
/// `HallRackPair` effect fills `North`, the second fills `South`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HallRackTier {
    North,
    South,
}

impl HallRackTier {
    /// Tier for the Nth installed `HallRackPair` effect, or `None`
    /// if the hall can't fit another pair.
    fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::North),
            1 => Some(Self::South),
            _ => None,
        }
    }
}

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    let pair_count = ctx
        .upgrades
        .installed_effects(&ctx.game_data.0.upgrades)
        .filter(|e| matches!(e, UpgradeEffect::HallRackPair))
        .count();

    for i in 0..pair_count {
        let Some(tier) = HallRackTier::from_index(i) else {
            break;
        };
        spawn_pair(ctx.commands, ctx.l, tier);
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
    upgrades: Res<PlayerUpgrades>,
    game_data: Res<GameDataResource>,
    existing: Query<&HallRackTier>,
) {
    if !upgrades.is_changed() {
        return;
    }
    let l = Layout::new();

    let declared_pairs = upgrades
        .installed_effects(&game_data.0.upgrades)
        .filter(|e| matches!(e, UpgradeEffect::HallRackPair))
        .count();

    for i in 0..declared_pairs {
        let Some(tier) = HallRackTier::from_index(i) else {
            break;
        };
        if existing.iter().any(|t| *t == tier) {
            continue;
        }
        spawn_pair(&mut commands, &l, tier);
    }
}
