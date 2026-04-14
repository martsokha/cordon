//! Squad blackboard components.
//!
//! The two data channels through which the squad's three deciders
//! talk to its one mover:
//!
//! - [`MovementIntent`] — written by behavior-tree action leaves
//!   ([`super::behave`]), read by the formation driver
//!   ([`super::formation`]). Answers: "where does the squad want
//!   its centroid this frame?"
//! - [`EngagementTarget`] — written by the vision scanner
//!   ([`super::engagement`]), read by the formation driver and by
//!   trees that want to branch on "are we currently engaging?".
//!
//! Keeping these two in a dedicated file documents the contract
//! between writers and readers. Neither is a subplugin-specific
//! type; both are cross-cutting blackboards.

use bevy::prelude::*;

/// Where the squad wants its leader-anchored centroid to be this
/// frame. `Some(target)` = walk there; `None` = hold position (use
/// the leader's current transform as the centroid).
///
/// Replaces the old `SquadActivity::Move` variant. Absence replaces
/// `SquadActivity::Hold`.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MovementIntent(pub Option<Vec2>);

/// Hostile squad entity the engagement scanner has locked onto this
/// frame, if any. Written by `update_squad_engagement`; read by the
/// formation per-member pass (to chase a combat target when out of
/// weapon range) and by behavior trees (for branching on "we're
/// currently engaging").
///
/// Replaces the old `SquadActivity::Engage` variant.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct EngagementTarget(pub Option<Entity>);
