//! Primitive value types used throughout the game.
//!
//! Small, self-contained types that represent a single concept:
//! a currency amount, a distance, a health value, etc. Used as
//! building blocks by the entity, item, and world modules.

mod condition;
mod credits;
mod distance;
mod duration;
mod environment;
mod experience;
mod hazard;
mod health;
mod id;
mod location;
mod rarity;
mod relation;
mod tier;
mod time;
mod uid;

pub use condition::Condition;
pub use credits::Credits;
pub use distance::Distance;
pub use duration::Duration;
pub use environment::Environment;
pub use experience::Experience;
pub use hazard::HazardType;
pub use health::Health;
pub use id::{Id, IdMarker};
pub use location::Location;
pub use rarity::Rarity;
pub use relation::Relation;
pub use tier::Tier;
pub use time::{Day, GameTime, Period};
pub use uid::Uid;
