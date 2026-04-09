//! Generic `(current, max)` pool primitive used for HP, stamina,
//! hunger, and any other bounded resource an NPC carries.
//!
//! A pool is parameterized by a marker type implementing [`PoolKind`]
//! so that an HP pool and a stamina pool are distinct types at
//! compile time. This lets Bevy query for `&Pool<Health>` and
//! `&Pool<Stamina>` as separate components without any runtime
//! branching.

use std::marker::PhantomData;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Marker trait for pool kinds.
///
/// Implementors are unit structs used only at the type level to
/// distinguish different kinds of pool (e.g., [`Health`], [`Stamina`],
/// [`Hunger`]). The trait carries per-kind constants so
/// [`Pool::full`] can produce a full pool at the right default
/// without the caller knowing the kind.
pub trait PoolKind: 'static + Send + Sync {
    /// Default maximum value for a fresh [`Pool::full`].
    const DEFAULT_MAX: u32 = 100;
}

/// Marker for a health pool.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize
)]
pub struct Health;
impl PoolKind for Health {}

/// Marker for a stamina pool.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize
)]
pub struct Stamina;
impl PoolKind for Stamina {}

/// Marker for a hunger pool.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize
)]
pub struct Hunger;
impl PoolKind for Hunger {}

/// Marker for an accumulated-radiation pool.
///
/// Unlike health / stamina / hunger which drain from full, a
/// radiation pool *accumulates* from zero: NPCs spawn with
/// `current = 0` and gain rads from contaminated areas, food,
/// or carried radioactive artifacts. Anti-rad pills and
/// radiation-scrubber relics drain the pool back down. Spawn
/// these with [`Pool::empty`] instead of [`Pool::full`].
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize
)]
pub struct Radiation;
impl PoolKind for Radiation {}

/// A bounded `(current, max)` resource.
///
/// The only way to mutate the inner fields is through methods that
/// enforce the invariant `current <= max`. `current` saturates at 0
/// on [`deplete`](Self::deplete) and caps at `max` on
/// [`restore`](Self::restore).
///
/// Serializes as `{"current": N, "max": M}`; the marker type isn't
/// persisted (the type annotation at the call site is the source of
/// truth for what kind of pool this is).
#[derive(
    Component,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize
)]
pub struct Pool<K: PoolKind> {
    current: u32,
    max: u32,
    #[serde(skip)]
    _marker: PhantomData<fn() -> K>,
}

impl<K: PoolKind> Pool<K> {
    /// Create a full pool at [`K::DEFAULT_MAX`]. For pools
    /// that drain from full (health, stamina, hunger).
    pub fn full() -> Self {
        Self::with_max(K::DEFAULT_MAX)
    }

    /// Create an empty pool at [`K::DEFAULT_MAX`]. For pools
    /// that accumulate from zero (radiation).
    pub fn empty() -> Self {
        Self {
            current: 0,
            max: K::DEFAULT_MAX,
            _marker: PhantomData,
        }
    }

    /// Create a full pool with an explicit max.
    pub fn with_max(max: u32) -> Self {
        Self {
            current: max,
            max,
            _marker: PhantomData,
        }
    }

    /// Create a pool with an explicit `(current, max)` pair. The
    /// `current` value is clamped to `[0, max]` to preserve the
    /// invariant.
    pub fn new(current: u32, max: u32) -> Self {
        Self {
            current: current.min(max),
            max,
            _marker: PhantomData,
        }
    }

    /// Current pool value.
    pub fn current(&self) -> u32 {
        self.current
    }

    /// Maximum pool value.
    pub fn max(&self) -> u32 {
        self.max
    }

    /// Current value as a `0.0..=1.0` ratio. Returns `0.0` when
    /// `max == 0` to avoid division by zero.
    pub fn ratio(&self) -> f32 {
        if self.max == 0 {
            0.0
        } else {
            self.current as f32 / self.max as f32
        }
    }

    /// `true` if the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.current == 0
    }

    /// `true` if the pool is at max.
    pub fn is_full(&self) -> bool {
        self.current == self.max
    }

    /// Subtract `amount` from `current`, saturating at 0.
    pub fn deplete(&mut self, amount: u32) {
        self.current = self.current.saturating_sub(amount);
    }

    /// Add `amount` to `current`, capped at `max`.
    pub fn restore(&mut self, amount: u32) {
        self.current = self.current.saturating_add(amount).min(self.max);
    }

    /// Set a new maximum. If the new max is below the current value,
    /// `current` is clamped down to match.
    pub fn set_max(&mut self, new_max: u32) {
        self.max = new_max;
        if self.current > new_max {
            self.current = new_max;
        }
    }
}

impl<K: PoolKind> Default for Pool<K> {
    fn default() -> Self {
        Self::full()
    }
}
