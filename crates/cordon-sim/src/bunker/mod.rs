//! Bunker-driven sim systems.
//!
//! Home for state and message handlers that live in cordon-sim
//! but are triggered by bunker interactions in cordon-app:
//!
//! - [`upgrades`] — laptop shop purchase flow.
//! - [`pills`] — [`PlayerPills`](pills::PlayerPills) dose tracking.
//!
//! Data stays on the sim side so quests and other systems can
//! read it without reaching across the app/sim boundary.

pub mod pills;
pub mod upgrades;
