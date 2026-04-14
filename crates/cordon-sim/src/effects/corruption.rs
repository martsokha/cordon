//! Passive corruption gain from standing inside anomaly areas.
//!
//! Every minute, each NPC takes a corruption delta proportional
//! to the corruption tier of every [`AnomalyField`] or [`Anchor`]
//! whose disk currently contains its position. Multiple
//! overlapping areas stack additively — an NPC standing in two
//! medium-tier zones takes both rates.
//!
//! Tier → rate per minute:
//!
//! | Tier      | Rate (corruption/min) |
//! |---|---|
//! | VeryLow  | 0.5 |
//! | Low      | 1.0 |
//! | Medium   | 2.0 |
//! | High     | 4.0 |
//! | VeryHigh | 8.0 |
//!
//! Why a single per-minute system rather than per-area triggers:
//! the area count is small (~10 anomaly disks) and the NPC count
//! is also small (~hundreds at peak), so an O(N×A) sweep is
//! cheap and avoids needing a spatial index or per-NPC "currently
//! inside" tracking. Skipping the work entirely on frames where
//! no minute rolled over keeps the steady-state cost near zero.
//!
//! [`AnomalyField`]: cordon_core::world::area::AreaKind::AnomalyField
//! [`Anchor`]: cordon_core::world::area::AreaKind::Anchor

use bevy::prelude::*;
use cordon_core::item::ResourceTarget;
use cordon_core::primitive::{GameTime, Tier};
use cordon_data::gamedata::GameDataResource;

use super::apply_pool_delta;
use crate::behavior::combat::NpcPoolChanged;
use crate::behavior::death::Dead;
use crate::entity::npc::{CorruptionPool, HealthPool, NpcMarker, StaminaPool};
use crate::resources::GameClock;

/// Corruption gained per minute, per anomaly tier.
fn rate_per_minute(tier: Tier) -> f32 {
    match tier {
        Tier::VeryLow => 0.5,
        Tier::Low => 1.0,
        Tier::Medium => 2.0,
        Tier::High => 4.0,
        Tier::VeryHigh => 8.0,
    }
}

/// Tracks the last minute we processed corruption ticks for, so
/// frames that cross 0 minute boundaries do nothing.
#[derive(Default)]
pub(crate) struct LastTick(Option<GameTime>);

/// Apply corruption-area passive gain to every alive NPC, scaled
/// by the number of in-game minutes elapsed since the last tick.
pub(crate) fn area_corruption_tick(
    clock: Res<GameClock>,
    data: Res<GameDataResource>,
    mut last: Local<LastTick>,
    mut pool_changed: MessageWriter<NpcPoolChanged>,
    mut npcs: Query<
        (
            Entity,
            &Transform,
            &mut HealthPool,
            &mut StaminaPool,
            &mut CorruptionPool,
        ),
        (With<NpcMarker>, Without<Dead>),
    >,
) {
    let now = clock.0;
    let last_tick = match last.0 {
        Some(t) => t,
        None => {
            // First frame: prime the clock without applying anything.
            last.0 = Some(now);
            return;
        }
    };
    let minutes = now.minutes_since(last_tick);
    if minutes == 0 {
        return;
    }
    last.0 = Some(now);

    // Pre-collect anomaly disks so we don't walk the area catalog
    // per NPC. Each entry is `(center, radius², rate_per_minute)`.
    // Squared radius lets the inner loop skip a sqrt.
    let disks: Vec<(Vec2, f32, f32)> = data
        .0
        .areas
        .values()
        .filter_map(|area| {
            if !area.kind.is_anomaly() {
                return None;
            }
            let tier = area.kind.corruption()?;
            let r = area.radius.value();
            Some((
                Vec2::new(area.location.x, area.location.y),
                r * r,
                rate_per_minute(tier),
            ))
        })
        .collect();
    if disks.is_empty() {
        return;
    }

    let minutes_f = minutes as f32;
    for (entity, transform, mut hp, mut stamina, mut corruption) in npcs.iter_mut() {
        let pos = transform.translation.truncate();
        let mut rate = 0.0_f32;
        for (center, r_sq, per_min) in &disks {
            if pos.distance_squared(*center) <= *r_sq {
                rate += per_min;
            }
        }
        if rate <= 0.0 {
            continue;
        }
        let delta = rate * minutes_f;
        apply_pool_delta(
            entity,
            ResourceTarget::Corruption,
            delta,
            &mut hp,
            &mut stamina,
            &mut corruption,
            &mut pool_changed,
        );
    }
}
