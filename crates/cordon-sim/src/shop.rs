//! Upgrade purchasing flow.
//!
//! The bunker laptop's Upgrades tab (in cordon-bevy) dispatches
//! [`BuyUpgrade`] messages; [`apply_buy_upgrade`] consumes them,
//! validates cost + prereq + duplicate-install, deducts credits,
//! and pushes the upgrade id onto `player.upgrades`.
//!
//! Side-effects (fog bypass, visual rack spawning, etc.) are
//! handled by systems that query `player.installed_effects(...)`
//! — this module only touches economy + the installed list.

use bevy::prelude::*;
use cordon_core::entity::bunker::{Upgrade, UpgradeDef};
use cordon_core::entity::player::PlayerState;
use cordon_core::primitive::Id;
use cordon_data::catalog::GameData;
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
        let result = validate_purchase(&player.0, &game_data.0, id).map(|def| {
            player.0.credits -= def.cost;
            player.0.upgrades.push(id.clone());
            info!("upgrade installed: `{}` ({})", id.as_str(), def.cost);
        });
        outcomes.write(BuyUpgradeOutcome {
            upgrade: id.clone(),
            result,
        });
    }
}

/// Check that `id` names a known upgrade the player can legally
/// install right now (not already installed, prereqs satisfied,
/// affordable). On success returns the matching [`UpgradeDef`] so
/// the caller can charge the cost without a second lookup.
fn validate_purchase<'a>(
    player: &PlayerState,
    data: &'a GameData,
    id: &Id<Upgrade>,
) -> Result<&'a UpgradeDef, BuyUpgradeFailure> {
    let def = data
        .upgrades
        .get(id)
        .ok_or(BuyUpgradeFailure::UnknownUpgrade)?;
    if player.has_upgrade(id) {
        return Err(BuyUpgradeFailure::AlreadyInstalled);
    }
    if def.requires.iter().any(|req| !player.has_upgrade(req)) {
        return Err(BuyUpgradeFailure::MissingPrerequisite);
    }
    if player.credits.value() < def.cost.value() {
        return Err(BuyUpgradeFailure::Unaffordable);
    }
    Ok(def)
}
