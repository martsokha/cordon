//! Trade-order bookkeeping: placement (credit deduction + queue)
//! and next-day delivery (spawn the supplier NPC as a visitor).
//!
//! Orders flow UI → sim → next-day visitor:
//!
//! 1. [`PlaceOrderRequest`] fires from the laptop Trade tab.
//! 2. [`place_orders`] checks unlock + stock + price + funds,
//!    deducts credits, appends a [`PendingOrder`], emits
//!    [`OrderPlaced`] (or [`OrderFailed`]).
//! 3. On day rollover, [`deliver_pending_orders`] drains the
//!    queue and spawns one delivery visitor per order, targeting
//!    the supplier's `delivery_yarn` node.
//!
//! The delivery yarn node calls `<<deliver_order>>` to transfer
//! the ordered item into the player's stash. Multiple orders
//! against the same supplier queue up as separate visitors.

use bevy::prelude::*;
use cordon_core::item::Item;
use cordon_core::primitive::Id;
use cordon_data::gamedata::GameDataResource;

use super::DayRolled;
use crate::quest::messages::{
    OrderFailed, OrderFailure, OrderPlaced, PlaceOrderRequest, SpawnNpcRequest,
};
use crate::resources::{
    GameClock, PendingOrder, PendingOrders, PlayerIdentity, PlayerSuppliers,
};

/// Consume [`PlaceOrderRequest`] messages and either queue the
/// order (emitting [`OrderPlaced`]) or reject it (emitting
/// [`OrderFailed`]).
pub fn place_orders(
    data: Res<GameDataResource>,
    clock: Res<GameClock>,
    suppliers: Res<PlayerSuppliers>,
    mut identity: ResMut<PlayerIdentity>,
    mut orders: ResMut<PendingOrders>,
    mut requests: MessageReader<PlaceOrderRequest>,
    mut placed_tx: MessageWriter<OrderPlaced>,
    mut failed_tx: MessageWriter<OrderFailed>,
) {
    for req in requests.read() {
        let Some(item_def) = data.0.items.get(&req.item) else {
            failed_tx.write(OrderFailed {
                item: req.item.clone(),
                supplier: req.supplier.clone(),
                reason: OrderFailure::UnknownItem,
            });
            continue;
        };
        let Some(template) = data.0.npc_templates.get(&req.supplier) else {
            failed_tx.write(OrderFailed {
                item: req.item.clone(),
                supplier: req.supplier.clone(),
                reason: OrderFailure::UnknownSupplier,
            });
            continue;
        };
        let Some(supplier_info) = template.supplier.as_ref() else {
            failed_tx.write(OrderFailed {
                item: req.item.clone(),
                supplier: req.supplier.clone(),
                reason: OrderFailure::SupplierDoesNotStock,
            });
            continue;
        };
        if !suppliers.is_unlocked(&req.supplier) {
            failed_tx.write(OrderFailed {
                item: req.item.clone(),
                supplier: req.supplier.clone(),
                reason: OrderFailure::SupplierLocked,
            });
            continue;
        }
        if !item_def.suppliers.contains(&req.supplier) {
            failed_tx.write(OrderFailed {
                item: req.item.clone(),
                supplier: req.supplier.clone(),
                reason: OrderFailure::SupplierDoesNotStock,
            });
            continue;
        }
        let price = supplier_info.price_for(item_def.base_price);
        if price != req.expected_price {
            failed_tx.write(OrderFailed {
                item: req.item.clone(),
                supplier: req.supplier.clone(),
                reason: OrderFailure::PriceMismatch,
            });
            continue;
        }
        if !identity.credits.can_afford(price) {
            failed_tx.write(OrderFailed {
                item: req.item.clone(),
                supplier: req.supplier.clone(),
                reason: OrderFailure::Insufficient,
            });
            continue;
        }

        identity.credits -= price;
        orders.push(PendingOrder {
            item: req.item.clone(),
            supplier: req.supplier.clone(),
            ordered_on: clock.0.day,
        });
        info!(
            "order placed: {} from {} for {}cr",
            req.item.as_str(),
            req.supplier.as_str(),
            price.value()
        );
        placed_tx.write(OrderPlaced {
            item: req.item.clone(),
            supplier: req.supplier.clone(),
            paid: price,
        });
    }
}

/// On day rollover: drain [`PendingOrders`] and dispatch one
/// delivery visit per *supplier* (not per order). Multiple orders
/// from the same supplier coalesce into a single visit carrying
/// all their items — suppliers are `unique: true`, so two spawn
/// requests for the same template in one frame would collapse
/// onto one entity anyway and the payload of the later request
/// would clobber the earlier.
///
/// The yarn node calls `<<deliver_order>>` once per item; the
/// delivery visitor stays at the counter until every item is
/// handed over.
pub fn deliver_pending_orders(
    data: Res<GameDataResource>,
    mut orders: ResMut<PendingOrders>,
    mut rolled: MessageReader<DayRolled>,
    mut spawn_tx: MessageWriter<SpawnNpcRequest>,
) {
    if rolled.read().next().is_none() {
        return;
    }
    let pending = orders.drain_for_delivery();
    if pending.is_empty() {
        return;
    }

    // Group orders by supplier while preserving placement order.
    let mut by_supplier: Vec<(Id<cordon_core::entity::npc::NpcTemplate>, Vec<Id<Item>>)> =
        Vec::new();
    for order in pending {
        if let Some(bucket) = by_supplier.iter_mut().find(|(s, _)| s == &order.supplier) {
            bucket.1.push(order.item);
        } else {
            by_supplier.push((order.supplier, vec![order.item]));
        }
    }

    for (supplier, items) in by_supplier {
        let Some(template) = data.0.npc_templates.get(&supplier) else {
            warn!(
                "delivery skipped: supplier `{}` not found in catalog",
                supplier.as_str()
            );
            continue;
        };
        let Some(supplier_info) = template.supplier.as_ref() else {
            warn!(
                "delivery skipped: supplier `{}` has no `supplier` block",
                supplier.as_str()
            );
            continue;
        };
        info!(
            "dispatching delivery: `{}` carrying {} item(s) → yarn `{}`",
            supplier.as_str(),
            items.len(),
            supplier_info.delivery_yarn
        );
        spawn_tx.write(SpawnNpcRequest {
            template: supplier,
            at: None,
            yarn_node: Some(supplier_info.delivery_yarn.clone()),
            delivery_items: items,
        });
    }
}
