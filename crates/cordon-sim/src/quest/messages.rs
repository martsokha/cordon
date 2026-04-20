//! All quest-related messages in one place.

use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::entity::npc::NpcTemplate;
use cordon_core::item::Item;
use cordon_core::primitive::{Credits, Experience, Id, RelationDelta};
use cordon_core::world::area::Area;
use cordon_core::world::narrative::{Decision, EndingCause, Intel, Quest};

/// Start a quest outside the regular trigger flow.
#[derive(Message, Debug, Clone)]
pub struct StartQuestRequest {
    pub quest: Id<Quest>,
}

/// Spawn a template NPC into the world.
#[derive(Message, Debug, Clone)]
pub struct SpawnNpcRequest {
    pub template: Id<NpcTemplate>,
    pub at: Option<Id<Area>>,
    pub yarn_node: Option<String>,
    /// When non-empty, the NPC is arriving as a trade delivery.
    /// The app-side visitor system carries the list through to
    /// the visitor's dialogue; `<<deliver_order>>` pops one item
    /// per call so a multi-order visit delivers each in turn.
    /// Empty for non-delivery spawns.
    pub delivery_items: Vec<Id<Item>>,
}

/// Dismiss a template NPC after dialogue (start return travel).
#[derive(Message, Debug, Clone)]
pub struct DismissTemplateNpc {
    pub entity: Entity,
    pub template: Id<NpcTemplate>,
}

/// Grant XP to a template NPC.
#[derive(Message, Debug, Clone)]
pub struct GiveNpcXpRequest {
    pub template: Id<NpcTemplate>,
    pub amount: Experience,
}

/// A faction standing changed via consequence.
#[derive(Message, Debug, Clone)]
pub struct StandingChanged {
    pub faction: Id<Faction>,
    pub delta: RelationDelta,
}

/// A quest was started.
#[derive(Message, Debug, Clone)]
pub struct QuestStarted {
    pub quest: Id<Quest>,
}

/// A quest advanced to a new stage (objective met, branch
/// resolved, talk completed).
#[derive(Message, Debug, Clone)]
pub struct QuestUpdated {
    pub quest: Id<Quest>,
}

/// A quest reached its outcome stage and completed.
#[derive(Message, Debug, Clone)]
pub struct QuestFinished {
    pub quest: Id<Quest>,
    pub success: bool,
}

/// A Talk stage's dialogue completed. Emitted by the cordon-app
/// Yarn bridge after copying flags; consumed by the drive system
/// to advance the quest stage.
#[derive(Message, Debug, Clone)]
pub struct TalkCompleted {
    pub quest: Id<Quest>,
    pub choice: Option<String>,
}

/// A player decision was recorded via
/// [`Consequence::RecordDecision`](cordon_core::world::narrative::Consequence::RecordDecision).
/// Consumed by the toast layer to surface a "this will have
/// consequences" beat and by anything else that reacts to the
/// moment of commitment (not just the resulting value).
#[derive(Message, Debug, Clone)]
pub struct DecisionRecorded {
    pub decision: Id<Decision>,
    pub value: String,
}

/// Emitted whenever a single intel entry is newly granted to the
/// player. Consumed by the toast layer to announce each piece of
/// intel individually and by anything else that reacts to new
/// intel becoming available (laptop, journal, etc.). Radio
/// broadcasts emit this on dialogue completion; other grant paths
/// (quest consequences, direct grants) should emit it alongside
/// their own state mutation.
#[derive(Message, Debug, Clone)]
pub struct IntelGranted {
    pub intel: Id<Intel>,
}

/// The run is over — transition to the ending slate. Emitted by
/// the [`Consequence::EndGame`] applier; consumed by cordon-app
/// to flip `AppState` to `Ending` and stash the `cause` for the
/// epitaph text.
#[derive(Message, Debug, Clone, Copy)]
pub struct EndGameRequest {
    pub cause: EndingCause,
}

/// The app-side Trade UI writes this when the player clicks an
/// "Order" button. The sim verifies the supplier is unlocked and
/// the player can afford the item, deducts credits, and appends
/// a [`PendingOrder`](crate::resources::PendingOrder) to be
/// delivered on the next day rollover. Failed orders emit
/// [`OrderFailed`] instead.
#[derive(Message, Debug, Clone)]
pub struct PlaceOrderRequest {
    pub item: Id<Item>,
    pub supplier: Id<NpcTemplate>,
    /// The price the UI showed the player. The sim sanity-checks
    /// this against its own price computation before charging, so
    /// a stale UI can't under-pay the world.
    pub expected_price: Credits,
}

/// Emitted when [`PlaceOrderRequest`] is accepted, credits are
/// deducted, and the order is queued. Consumed by the toast layer
/// for a "Order placed" beat.
#[derive(Message, Debug, Clone)]
pub struct OrderPlaced {
    pub item: Id<Item>,
    pub supplier: Id<NpcTemplate>,
    pub paid: Credits,
}

/// Why an order could not be placed. Carried as a field on
/// [`OrderFailed`]; not a `Message` on its own.
#[derive(Debug, Clone, Copy)]
pub enum OrderFailure {
    /// Supplier isn't in the player's unlocked set.
    SupplierLocked,
    /// Item isn't listed under this supplier's roster.
    SupplierDoesNotStock,
    /// Base price / multiplier changed since the UI last looked.
    PriceMismatch,
    /// Player doesn't have enough credits at charge time.
    Insufficient,
    /// Item id doesn't resolve in the catalog.
    UnknownItem,
    /// Supplier id doesn't resolve in the catalog.
    UnknownSupplier,
}

/// Rejection reason for a placed order.
#[derive(Message, Debug, Clone)]
pub struct OrderFailed {
    pub item: Id<Item>,
    pub supplier: Id<NpcTemplate>,
    pub reason: OrderFailure,
}
