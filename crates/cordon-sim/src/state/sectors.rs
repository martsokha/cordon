use cordon_core::primitive::id::Id;

/// Live state of a sector in the world.
///
/// Tracks dynamic properties that change during gameplay:
/// faction control, danger modifiers from events, creature activity.
/// Base danger/reward values come from the sector's config definition.
pub struct SectorState {
    pub id: Id,
    /// Which faction currently controls this sector, if any.
    pub controlling_faction: Option<Id>,
    /// Additive danger modifier from events/world state.
    pub danger_modifier: f32,
    /// Creature activity level (0.0–1.0). Affects danger.
    pub creature_activity: f32,
    /// Whether a hazard field is currently active.
    pub hazard_active: bool,
}

impl SectorState {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            controlling_faction: None,
            danger_modifier: 0.0,
            creature_activity: 0.0,
            hazard_active: false,
        }
    }
}
