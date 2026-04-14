/// Area definitions loaded from config.
pub mod area;

/// Loot tables: per-area weighted drop tables.
pub mod loot;

/// Runner missions: plans, outcomes, and results.
pub mod mission;

/// Quests, events, triggers, and the shared condition/consequence
/// vocabulary they all use. Submodules are private — callers
/// import from the top-level `narrative` namespace.
pub mod narrative;

/// World-space position of the bunker on the laptop map.
///
/// The bunker is rendered at the origin in the map view; runners
/// and travelling quest NPCs that need to "head home" use this
/// constant as their target.
pub const BUNKER_MAP_POS: bevy::math::Vec2 = bevy::math::Vec2::ZERO;

/// Price calculation with condition-squared scaling.
pub mod price;

/// Trade offers between the player and NPCs.
pub mod trade;
