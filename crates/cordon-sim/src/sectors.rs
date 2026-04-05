use cordon_core::entity::faction::FactionId;
use cordon_core::world::sector::SectorId;

/// Live state of a sector in the world.
pub struct SectorState {
    pub id: SectorId,
    pub controlling_faction: Option<FactionId>,
    pub danger_modifier: f32,
    pub creature_activity: f32,
    pub hazard_active: bool,
}

impl SectorState {
    pub fn new(id: SectorId) -> Self {
        let controlling_faction = match id {
            SectorId::Scrapyard | SectorId::Hollows => Some(FactionId::Syndicate),
            SectorId::Depot => None, // contested
            _ => None,
        };

        Self {
            id,
            controlling_faction,
            danger_modifier: 0.0,
            creature_activity: 0.0,
            hazard_active: false,
        }
    }

    pub fn effective_danger(&self) -> f32 {
        let base = self.id.base_danger();
        (base + self.danger_modifier + self.creature_activity * 0.3).clamp(0.0, 1.0)
    }
}
