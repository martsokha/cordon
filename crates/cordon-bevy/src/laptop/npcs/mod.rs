//! NPC map visuals: dot meshes, selection rings, faction-coloured
//! materials, attach-on-spawn hookup, and the click/selection
//! interaction loop.
//!
//! Split into three files:
//!
//! - `mod.rs` — plugin, asset init, `NpcDotInfo`, `attach_npc_visuals`
//! - `palette.rs` — `FactionPalette` (per-faction dot and corpse colors)
//! - `selection.rs` — `SelectedNpc`, `SelectionRing`, click handling

pub mod palette;
mod selection;

use bevy::prelude::*;
use bevy_fluent::prelude::*;
use cordon_core::entity::faction::RankScheme;
use cordon_core::entity::name::{NameFormat, NpcName};
use cordon_core::entity::squad::Formation;
use cordon_core::primitive::Experience;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::behavior::{CombatTarget, FireState, MovementSpeed, MovementTarget, Vision};
use cordon_sim::components::{FactionId, NpcMarker, SquadHomePosition, SquadMembership};

pub use self::palette::FactionPalette;
pub use self::selection::SelectedNpc;
use crate::PlayingState;
use crate::laptop::map::MapWorldEntity;
use crate::locale::l10n_or;

const COLOR_NPC_SELECTED: Color = Color::srgb(1.0, 0.9, 0.3);
const COLOR_NPC_SQUAD: Color = Color::srgb(0.7, 0.6, 0.25);

/// Shared mesh + outline material handles for NPC dots. The
/// per-faction default tints live in [`FactionPalette`]. Selection
/// state is shown via an outline ring child — `selected_ring_mesh`
/// for the focused NPC, `squad_ring_mesh` for their squadmates —
/// rather than by re-tinting the dot itself.
#[derive(Resource, Clone)]
pub struct NpcAssets {
    pub dot_mesh: Handle<Mesh>,
    pub selected_ring_mesh: Handle<Mesh>,
    pub squad_ring_mesh: Handle<Mesh>,
    pub selected_ring_mat: Handle<ColorMaterial>,
    pub squad_ring_mat: Handle<ColorMaterial>,
}

/// Per-NPC tooltip strings cached at spawn time so the hover
/// system doesn't have to re-resolve localized names on every
/// frame. Attached by `attach_npc_visuals`.
#[derive(Component, Clone)]
pub struct NpcDotInfo {
    pub faction_icon: String,
    pub name: String,
    pub faction: String,
    pub rank: String,
}

pub struct NpcsPlugin;

impl Plugin for NpcsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SelectedNpc::default());
        app.add_systems(Startup, init_npc_assets);

        // Faction palette builds on `OnEnter(Playing)` so NPCs
        // spawned during bunker state also get their faction
        // colors ready before the player ever opens the laptop.
        app.add_systems(
            OnEnter(crate::AppState::Playing),
            palette::build_faction_palette.run_if(not(resource_exists::<FactionPalette>)),
        );

        // Visual attachment must keep up with the sim regardless
        // of which view the player is currently looking at —
        // `Added<T>` only fires for one frame, so gating on
        // `Laptop` would miss NPCs spawned in the bunker.
        app.add_systems(
            Update,
            attach_npc_visuals
                .after(cordon_sim::plugin::SimSet::Spawn)
                .run_if(in_state(crate::AppState::Playing)),
        );

        app.add_systems(
            Update,
            (
                selection::handle_npc_click,
                selection::update_npc_selection,
                selection::deselect_or_exit,
            )
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

fn init_npc_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Dot radius is 6; rings sit at 8 (outer) with a 1.5px band
    // so they read as a crisp outline around the dot.
    let dot_mesh = meshes.add(Circle::new(6.0));
    let selected_ring_mesh = meshes.add(Annulus::new(7.5, 9.0));
    let squad_ring_mesh = meshes.add(Annulus::new(7.0, 8.2));
    let selected_ring_mat = materials.add(ColorMaterial::from_color(COLOR_NPC_SELECTED));
    let squad_ring_mat = materials.add(ColorMaterial::from_color(COLOR_NPC_SQUAD));
    commands.insert_resource(NpcAssets {
        dot_mesh,
        selected_ring_mesh,
        squad_ring_mesh,
        selected_ring_mat,
        squad_ring_mat,
    });
}

pub fn faction_icon_str(faction: Option<&str>) -> &'static str {
    match faction {
        Some("faction_garrison") => "[G]",
        Some("faction_syndicate") => "[S]",
        Some("faction_institute") => "[I]",
        Some("faction_devoted") => "[D]",
        Some("faction_drifters") => "[d]",
        _ => "[?]",
    }
}

fn rank_scheme_key(scheme: &RankScheme) -> &'static str {
    match scheme {
        RankScheme::Military => "military",
        RankScheme::Loose => "loose",
        RankScheme::Religious => "religious",
        RankScheme::Academic => "academic",
    }
}

fn resolve_npc_name(l10n: &Localization, name: &NpcName) -> String {
    let first = l10n_or(l10n, &name.first, &name.first);
    match (&name.format, &name.second) {
        (NameFormat::Alias, _) => first,
        (NameFormat::FirstSurname, Some(second)) => {
            let second = l10n_or(l10n, second, second);
            format!("{first} {second}")
        }
        (NameFormat::FirstAlias, Some(second)) => {
            let second = l10n_or(l10n, second, second);
            format!("{first} \"{second}\"")
        }
        _ => first,
    }
}

/// Attach laptop-side visuals to freshly-spawned NPC entities. Runs
/// after `spawn_population` every frame, but only does real work
/// for entities that were just given a `SquadMembership`.
fn attach_npc_visuals(
    game_data: Res<GameDataResource>,
    npc_assets: Res<NpcAssets>,
    palette: Res<FactionPalette>,
    l10n: Option<Res<Localization>>,
    squads: Query<(
        &SquadHomePosition,
        &Formation,
        &cordon_sim::components::SquadMembers,
    )>,
    new_npcs: Query<
        (Entity, &FactionId, &Experience, &NpcName, &SquadMembership),
        (With<NpcMarker>, Added<SquadMembership>),
    >,
    mut commands: Commands,
) {
    if new_npcs.iter().next().is_none() {
        return;
    }
    let data = &game_data.0;
    let empty_l10n = Localization::default();
    let l10n = l10n.as_deref().unwrap_or(&empty_l10n);

    for (entity, faction, xp, name, membership) in &new_npcs {
        let faction_str = faction.0.as_str();
        let faction_icon = faction_icon_str(Some(faction_str)).to_string();
        let faction_name = l10n_or(l10n, faction_str, faction_str);
        let name_display = resolve_npc_name(l10n, name);
        let rank = xp.npc_rank();
        let rank_title = data
            .faction(&faction.0)
            .map(|fdef| {
                let key = format!("rank-{}-{}", rank_scheme_key(&fdef.rank_scheme), rank.key());
                l10n_or(l10n, &key, &key)
            })
            .unwrap_or_else(|| format!("Rank {}", rank.key()));

        // Squad's home position + this member's slot offset, computed
        // from the *actual* squad size (not a hardcoded 5).
        let (home, slot_offset) = match squads.get(membership.squad) {
            Ok((home, formation, members)) => {
                let count = members.0.len().max(1);
                let offsets = formation.slot_offsets(count);
                let slot = (membership.slot as usize).min(offsets.len() - 1);
                (home.0, Vec2::new(offsets[slot][0], offsets[slot][1]))
            }
            Err(_) => (Vec2::ZERO, Vec2::ZERO),
        };
        let spawn_pos = home + slot_offset;

        let vision = Vision::for_npc(rank);

        commands.entity(entity).insert((
            MapWorldEntity,
            NpcDotInfo {
                faction_icon,
                name: name_display,
                faction: faction_name,
                rank: rank_title,
            },
            vision,
            MovementTarget::default(),
            MovementSpeed::default(),
            CombatTarget::default(),
            FireState::default(),
            Mesh2d(npc_assets.dot_mesh.clone()),
            MeshMaterial2d(palette.dot(&faction.0)),
            // z=10 keeps NPC dots (and the corpse X children that
            // ride the same transform) above the cloud layer at z=5.
            Transform::from_xyz(spawn_pos.x, spawn_pos.y, 10.0),
        ));
    }
}
