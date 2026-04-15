//! Upgrade purchasing flow.
//!
//! The bunker laptop's Upgrades tab (in cordon-bevy) dispatches
//! [`BuyUpgrade`] messages; [`apply_buy_upgrade`] consumes them,
//! validates cost + prereq + duplicate-install, deducts credits,
//! and pushes the upgrade id onto `player.upgrades`.
//!
//! Side-effects (storage capacity, fog bypass, visual rack
//! spawning) are handled by existing systems that watch
//! `player.upgrades` — this module only touches economy + the
//! installed list.

use bevy::prelude::*;
use cordon_core::entity::bunker::Upgrade;
use cordon_core::primitive::Id;
use cordon_data::gamedata::GameDataResource;

use crate::resources::Player;

/// Request to purchase and install the named upgrade. Emitted by
/// the laptop UI; handled by [`apply_buy_upgrade`].
#[derive(Message, Debug, Clone)]
pub struct BuyUpgrade {
    pub upgrade: Id<Upgrade>,
}

/// Possible failure modes a buy request can hit. Returned via
/// [`BuyUpgradeOutcome`] so the UI can surface feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuyUpgradeFailure {
    UnknownUpgrade,
    AlreadyInstalled,
    MissingPrerequisite,
    Unaffordable,
}

/// One outcome per handled [`BuyUpgrade`] request. The UI watches
/// these to show errors ("Not enough credits", "Install X first")
/// and refresh the upgrade list on success.
#[derive(Message, Debug, Clone)]
pub struct BuyUpgradeOutcome {
    pub upgrade: Id<Upgrade>,
    pub result: Result<(), BuyUpgradeFailure>,
}

/// Drain `BuyUpgrade` messages and process each. Runs every
/// frame (cheap — the message stream is almost always empty).
pub fn apply_buy_upgrade(
    mut requests: MessageReader<BuyUpgrade>,
    mut outcomes: MessageWriter<BuyUpgradeOutcome>,
    mut player: ResMut<Player>,
    game_data: Res<GameDataResource>,
) {
    for request in requests.read() {
        let id = &request.upgrade;
        let data = &game_data.0;

        let Some(def) = data.upgrades.get(id) else {
            outcomes.write(BuyUpgradeOutcome {
                upgrade: id.clone(),
                result: Err(BuyUpgradeFailure::UnknownUpgrade),
            });
            continue;
        };

        if player.0.has_upgrade(id) {
            outcomes.write(BuyUpgradeOutcome {
                upgrade: id.clone(),
                result: Err(BuyUpgradeFailure::AlreadyInstalled),
            });
            continue;
        }

        // Prereqs: every required upgrade must already be installed.
        let missing_prereq = def
            .requires
            .iter()
            .any(|req| !player.0.has_upgrade(req));
        if missing_prereq {
            outcomes.write(BuyUpgradeOutcome {
                upgrade: id.clone(),
                result: Err(BuyUpgradeFailure::MissingPrerequisite),
            });
            continue;
        }

        if player.0.credits.value() < def.cost.value() {
            outcomes.write(BuyUpgradeOutcome {
                upgrade: id.clone(),
                result: Err(BuyUpgradeFailure::Unaffordable),
            });
            continue;
        }

        // All checks passed — charge the cost and install.
        player.0.credits -= def.cost;
        player.0.upgrades.push(id.clone());
        player.0.recompute_storage_capacity(&data.upgrades);

        info!(
            "upgrade installed: `{}` ({})",
            id.as_str(),
            def.cost
        );
        outcomes.write(BuyUpgradeOutcome {
            upgrade: id.clone(),
            result: Ok(()),
        });
    }
}
