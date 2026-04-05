/// Consequences and conditions shared by quests and events.
pub mod consequence;

/// Zone events: surges, faction wars, raids, and more.
pub mod event;

/// Runner missions: plans, outcomes, and results.
pub mod mission;

/// Price calculation with condition-squared scaling.
pub mod price;

/// Quest definitions, stages, and runtime state.
pub mod quest;

/// Area definitions loaded from config.
pub mod area;

/// Loot tables: per-area weighted drop tables.
pub mod loot;

/// Day/phase time system.
pub mod time;

/// Trade offers between the player and NPCs.
pub mod trade;
