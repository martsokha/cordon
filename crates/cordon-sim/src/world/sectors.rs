use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;
use cordon_core::world::area::Area;

/// Live state of an area in the world.
///
/// Tracks dynamic properties that change during gameplay:
/// faction control, danger modifiers from events, creature activity.
/// Base danger/reward values come from the area's config definition.
pub struct AreaState {
    pub id: Id<Area>,
    /// Which faction currently controls this area, if any.
    pub controlling_faction: Option<Id<Faction>>,
    /// Additive danger modifier from events/world state.
    pub danger_modifier: f32,
    /// Creature activity level (0.0–1.0). Affects danger.
    pub creature_activity: f32,
    /// Whether a hazard field is currently active.
    pub hazard_active: bool,
}

impl AreaState {
    pub fn new(id: Id<Area>) -> Self {
        Self {
            id,
            controlling_faction: None,
            danger_modifier: 0.0,
            creature_activity: 0.0,
            hazard_active: false,
        }
    }
}
