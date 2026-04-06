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

pub use self::condition::Condition;
pub use self::credits::Credits;
pub use self::distance::Distance;
pub use self::duration::Duration;
pub use self::environment::Environment;
pub use self::experience::Experience;
pub use self::hazard::HazardType;
pub use self::health::Health;
pub use self::id::{Id, IdMarker};
pub use self::location::Location;
pub use self::rarity::Rarity;
pub use self::relation::Relation;
pub use self::tier::Tier;
pub use self::time::{Day, GameTime, Period};
pub use self::uid::Uid;
