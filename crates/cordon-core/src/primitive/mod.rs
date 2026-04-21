//! Primitive value types used throughout the game.
//!
//! Small, self-contained types that represent a single concept:
//! a currency amount, a distance, a health pool, etc. Used as
//! building blocks by the entity, item, and world modules.

mod credits;
mod distance;
mod duration;
mod experience;
mod id;
mod location;
mod pool;
mod rank;
mod rarity;
mod relation;
mod resistances;
mod tier;
mod time;
mod uid;

pub use self::credits::Credits;
pub use self::distance::Distance;
pub use self::duration::Duration;
pub use self::experience::Experience;
pub use self::id::{Id, IdMarker};
pub use self::location::Location;
pub use self::pool::{Corruption, Health, Pool, PoolKind, Stamina};
pub use self::rank::Rank;
pub use self::rarity::Rarity;
pub use self::relation::{Relation, RelationDelta};
pub use self::resistances::Resistances;
pub use self::tier::Tier;
pub use self::time::{Day, GameTime};
pub use self::uid::Uid;
