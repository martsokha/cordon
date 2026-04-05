//! Trade offers between the player and NPCs.

use serde::{Deserialize, Serialize};

use crate::item::Item;
use crate::item::def::Item as ItemMarker;
use crate::primitive::credits::Credits;
use crate::primitive::id::Id;
use crate::primitive::uid::Uid;

/// An item offered in a trade (from either side).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeItem {
    /// The item being traded.
    pub item: Item,
    /// The agreed-upon price.
    pub price: Credits,
}

/// A trade offer between the player and an NPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOffer {
    /// Runtime UID of the NPC involved in this trade.
    pub npc_id: Uid,
    /// Items the NPC is offering to sell.
    pub npc_selling: Vec<TradeItem>,
    /// Items the NPC wants to buy from the player.
    pub npc_buying: Vec<Id<ItemMarker>>,
}
