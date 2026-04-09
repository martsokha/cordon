//! Per-faction display palette: color + pre-built `ColorMaterial`
//! handles for area disks, NPC dots, and corpse markers.
//!
//! Built once on laptop entry from the loaded `FactionDef` catalog so
//! all visuals share the same handles and the renderer can batch
//! draws by faction. Hex strings in the JSON are parsed to `Color`
//! here; parse failures fall back to a neutral grey.

use std::collections::HashMap;

use bevy::color::Srgba;
use bevy::prelude::*;
use cordon_core::entity::faction::Faction;
use cordon_core::primitive::Id;
use cordon_data::gamedata::GameDataResource;

/// Pre-built per-faction visuals.
#[derive(Resource, Clone)]
pub struct FactionPalette {
    /// Tinted material for NPC dots (full alpha).
    pub dot_mat: HashMap<Id<Faction>, Handle<ColorMaterial>>,
    /// Tinted material for the X-bar corpse marker.
    pub corpse_mat: HashMap<Id<Faction>, Handle<ColorMaterial>>,
    /// Fallback material when an entity has no known faction.
    pub fallback_dot: Handle<ColorMaterial>,
    pub fallback_corpse: Handle<ColorMaterial>,
}

impl FactionPalette {
    pub fn dot(&self, faction: &Id<Faction>) -> Handle<ColorMaterial> {
        self.dot_mat
            .get(faction)
            .cloned()
            .unwrap_or_else(|| self.fallback_dot.clone())
    }

    pub fn corpse(&self, faction: &Id<Faction>) -> Handle<ColorMaterial> {
        self.corpse_mat
            .get(faction)
            .cloned()
            .unwrap_or_else(|| self.fallback_corpse.clone())
    }
}

const FALLBACK: Color = Color::srgb(0.6, 0.6, 0.6);

fn parse_hex(hex: &str) -> Color {
    Srgba::hex(hex).map(Color::Srgba).unwrap_or(FALLBACK)
}

/// Build the palette from the loaded faction catalog. Idempotent —
/// safe to call once on `OnEnter(Laptop)`.
pub fn build_faction_palette(
    mut commands: Commands,
    game_data: Res<GameDataResource>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let cap = game_data.0.factions.len();
    let mut dot_mat: HashMap<Id<Faction>, Handle<ColorMaterial>> = HashMap::with_capacity(cap);
    let mut corpse_mat: HashMap<Id<Faction>, Handle<ColorMaterial>> = HashMap::with_capacity(cap);

    for (id, def) in &game_data.0.factions {
        let color = parse_hex(&def.color);

        // NPC dot: full alpha, slightly desaturated to keep the map
        // readable when many of them are clustered.
        dot_mat.insert(id.clone(), materials.add(ColorMaterial::from_color(color)));

        // Corpse X-bars: dimmed faction color so dead bodies are
        // recognizable but visually de-emphasized vs alive members.
        let s = color.to_srgba();
        let corpse_color = Color::srgba(s.red * 0.7, s.green * 0.7, s.blue * 0.7, 0.85);
        corpse_mat.insert(
            id.clone(),
            materials.add(ColorMaterial::from_color(corpse_color)),
        );
    }

    let fallback_dot = materials.add(ColorMaterial::from_color(FALLBACK));
    let fallback_corpse = materials.add(ColorMaterial::from_color(FALLBACK.with_alpha(0.85)));

    commands.insert_resource(FactionPalette {
        dot_mat,
        corpse_mat,
        fallback_dot,
        fallback_corpse,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_accepts_with_and_without_hash() {
        let a = parse_hex("#6B8C4D");
        let b = parse_hex("6B8C4D");
        assert_eq!(a.to_srgba(), b.to_srgba());
    }

    #[test]
    fn parse_hex_falls_back_on_garbage() {
        let c = parse_hex("not-a-color");
        let f = FALLBACK;
        assert_eq!(c.to_srgba(), f.to_srgba());
    }
}
