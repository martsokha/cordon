//! Laptop view: the Zone map with areas, bunker, and NPC dots.

mod environment;
mod input;
mod ui;

use bevy::prelude::*;
use bevy_fluent::prelude::*;
use cordon_core::entity::faction::RankScheme;
use cordon_core::entity::name::{NameFormat, NpcName};
use cordon_core::primitive::{HazardType, Tier};
use cordon_core::world::area::AreaDef;
use cordon_data::gamedata::GameDataResource;

use cordon_sim::behavior::{CombatTarget, FireState, MovementSpeed, MovementTarget, Vision};
use cordon_sim::components::{
    FactionId, NpcMarker, NpcNameComp, SquadFormation, SquadHomePosition, SquadMembership, Xp,
};

use crate::PlayingState;
use crate::locale::{GameLocalization, l10n_or};

pub struct LaptopPlugin;

impl Plugin for LaptopPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            input::InputPlugin,
            ui::UiPlugin,
            environment::EnvironmentPlugin,
        ));
        app.insert_resource(SelectedNpc::default());
        app.add_systems(Startup, (setup_camera, init_npc_assets));
        app.add_systems(
            OnEnter(PlayingState::Laptop),
            spawn_map.run_if(not(resource_exists::<MapSpawned>)),
        );
        app.add_systems(
            Update,
            (
                attach_npc_visuals.after(cordon_sim::plugin::SimSet::Spawn),
                update_hover,
                handle_npc_click,
                update_npc_selection,
                deselect_or_exit,
            )
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

/// Shared mesh + material handles for NPC dots so the renderer can
/// batch all 1000 sprites into a single draw call instead of one per
/// NPC. The three materials cover the three colour states: default,
/// selected, and squadmate of selected.
#[derive(Resource, Clone)]
pub struct NpcAssets {
    pub dot_mesh: Handle<Mesh>,
    pub default_mat: Handle<ColorMaterial>,
    pub selected_mat: Handle<ColorMaterial>,
    pub squad_mat: Handle<ColorMaterial>,
}

fn init_npc_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let dot_mesh = meshes.add(Circle::new(6.0));
    let default_mat = materials.add(ColorMaterial::from_color(COLOR_NPC));
    let selected_mat = materials.add(ColorMaterial::from_color(COLOR_NPC_SELECTED));
    let squad_mat = materials.add(ColorMaterial::from_color(COLOR_NPC_SQUAD));
    commands.insert_resource(NpcAssets {
        dot_mesh,
        default_mat,
        selected_mat,
        squad_mat,
    });
}

pub use self::ui::MapWorldEntity;
use self::ui::map::{TooltipContent, cursor_world_pos};
use self::ui::{LaptopFont, spawn_ui};

#[derive(Component)]
struct Bunker;

#[derive(Component)]
struct AreaCircle;

#[derive(Component)]
struct AreaData(AreaTooltipInfo);

#[derive(Clone)]
struct AreaTooltipInfo {
    faction_icon: String,
    name: String,
    creatures: String,
    creatures_tier: Tier,
    radiation: String,
    radiation_tier: Tier,
    hazard_icon: String,
    hazard_image: Option<String>,
    hazard_count: u8,
    loot: String,
    loot_tier: Tier,
}

#[derive(Component, Clone)]
struct NpcDotInfo {
    faction_icon: String,
    name: String,
    faction: String,
    rank: String,
}

#[derive(Resource, Default)]
struct SelectedNpc(Option<Entity>);

const COLOR_AREA: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);
const COLOR_AREA_BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.25);
const COLOR_AREA_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.15);
const COLOR_NPC: Color = Color::srgb(0.7, 0.7, 0.7);
const COLOR_NPC_SELECTED: Color = Color::srgb(1.0, 0.9, 0.3);
const COLOR_NPC_SQUAD: Color = Color::srgb(0.7, 0.6, 0.25);

/// All laptop 2D entities render on layer 1 (not the main camera).
fn hazard_icon(h: &HazardType) -> &'static str {
    match h {
        HazardType::Chemical => "X",
        HazardType::Thermal => "*",
        HazardType::Electric => "~",
        HazardType::Gravitational => "O",
    }
}

fn faction_icon_str(faction: Option<&str>) -> &'static str {
    match faction {
        Some("garrison") => "[G]",
        Some("syndicate") => "[S]",
        Some("order") => "[O]",
        Some("collective") => "[C]",
        Some("mercenaries") => "[M]",
        Some("institute") => "[I]",
        Some("devoted") => "[D]",
        Some("drifters") => "[d]",
        _ => "[ ]",
    }
}

#[derive(Resource)]
struct MapSpawned;

#[derive(Component)]
pub struct LaptopCamera;

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        LaptopCamera,
        Camera2d,
        Camera {
            is_active: false,
            order: 1,
            ..default()
        },
        Transform::from_xyz(0.0, -100.0, 1000.0),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

fn tier_key(t: &Tier) -> &'static str {
    match t {
        Tier::VeryLow => "tier-verylow",
        Tier::Low => "tier-low",
        Tier::Medium => "tier-medium",
        Tier::High => "tier-high",
        Tier::VeryHigh => "tier-veryhigh",
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

fn format_npc_status(
    movement: &MovementTarget,
    combat: &CombatTarget,
    looting: bool,
    goal: &cordon_core::entity::squad::Goal,
) -> String {
    let doing = if combat.0.is_some() {
        "Fighting"
    } else if looting {
        "Looting"
    } else if movement.0.is_some() {
        "Walking"
    } else {
        "Idle"
    };
    let purpose = match goal {
        cordon_core::entity::squad::Goal::Idle => "idle",
        cordon_core::entity::squad::Goal::Patrol { .. } => "patrolling",
        cordon_core::entity::squad::Goal::Scavenge { .. } => "scavenging",
        cordon_core::entity::squad::Goal::Protect { .. } => "protecting",
        cordon_core::entity::squad::Goal::Find { .. } => "hunting",
        cordon_core::entity::squad::Goal::Deliver { .. } => "delivering",
    };
    format!("{doing} ({purpose})")
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

fn build_area_info(l10n: &Localization, area: &AreaDef) -> AreaTooltipInfo {
    let hazard_icon_str = area
        .danger
        .hazard
        .as_ref()
        .map(|h| hazard_icon(&h.kind).to_string())
        .unwrap_or_default();

    AreaTooltipInfo {
        faction_icon: faction_icon_str(area.default_faction.as_ref().map(|f| f.as_str()))
            .to_string(),
        name: l10n_or(
            l10n,
            &format!("area-{}", area.id.as_str()),
            area.id.as_str(),
        ),
        creatures: l10n_or(
            l10n,
            tier_key(&area.danger.creatures),
            &format!("{:?}", area.danger.creatures),
        ),
        creatures_tier: area.danger.creatures,
        radiation: l10n_or(
            l10n,
            tier_key(&area.danger.radiation),
            &format!("{:?}", area.danger.radiation),
        ),
        radiation_tier: area.danger.radiation,
        hazard_icon: hazard_icon_str,
        hazard_image: area.danger.hazard.as_ref().map(|h| match h.kind {
            HazardType::Chemical => "icons/hazards/chemical.png".to_string(),
            HazardType::Thermal => "icons/hazards/thermal.png".to_string(),
            HazardType::Electric => "icons/hazards/electric.png".to_string(),
            HazardType::Gravitational => "icons/hazards/gravitational.png".to_string(),
        }),
        hazard_count: area
            .danger
            .hazard
            .as_ref()
            .map(|h| match h.intensity {
                Tier::VeryLow => 1,
                Tier::Low => 2,
                Tier::Medium => 3,
                Tier::High => 4,
                Tier::VeryHigh => 5,
            })
            .unwrap_or(0),
        loot: l10n_or(
            l10n,
            tier_key(&area.loot_tier),
            &format!("{:?}", area.loot_tier),
        ),
        loot_tier: area.loot_tier,
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_map(
    game_data: Res<GameDataResource>,
    laptop_font: Res<LaptopFont>,
    _asset_server: Res<AssetServer>,
    l10n: Option<Res<GameLocalization>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let data = &game_data.0;
    let empty_l10n = Localization::default();
    let l10n = l10n.as_ref().map(|r| &r.0).unwrap_or(&empty_l10n);

    spawn_ui(&mut commands, &laptop_font.0);

    for area in data.areas.values() {
        let x = area.location.x;
        let y = area.location.y;
        let radius = area.radius.value();
        let info = build_area_info(l10n, area);

        let _area_entity = commands.spawn((
            MapWorldEntity,
            AreaCircle,
            AreaData(info),
            Mesh2d(meshes.add(Circle::new(radius))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_AREA))),
            Transform::from_xyz(x, y, 0.01),
        ));

        commands.spawn((
            MapWorldEntity,
            Mesh2d(meshes.add(Annulus::new(radius - 2.0, radius))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(COLOR_AREA_BORDER))),
            Transform::from_xyz(x, y, 0.1),
        ));
    }

    commands.spawn((
        MapWorldEntity,
        Bunker,
        Mesh2d(meshes.add(Circle::new(10.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::WHITE))),
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));

    info!("Laptop map: {} areas (NPCs spawned by cordon-sim)", data.areas.len());

    commands.insert_resource(MapSpawned);
}

#[allow(clippy::type_complexity)]
fn update_hover(
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<LaptopCamera>>,
    cam_proj: Query<&Projection, With<LaptopCamera>>,
    areas: Query<(&AreaData, &Transform, &MeshMaterial2d<ColorMaterial>), With<AreaCircle>>,
    npcs: Query<
        (
            &NpcDotInfo,
            &Transform,
            &MovementTarget,
            &CombatTarget,
            &SquadMembership,
            Option<&cordon_sim::behavior::LootState>,
        ),
        With<NpcMarker>,
    >,
    squad_goals: Query<&cordon_sim::components::SquadGoal>,
    mut mats: ResMut<Assets<ColorMaterial>>,
    mut tooltip: ResMut<TooltipContent>,
) {
    let Some(cursor) = cursor_world_pos(&windows, &cameras) else {
        *tooltip = TooltipContent::Hidden;
        return;
    };
    let scale = cam_proj
        .iter()
        .next()
        .and_then(|p| match p {
            Projection::Orthographic(o) => Some(o.scale),
            _ => None,
        })
        .unwrap_or(1.0);
    let npc_hit = 20.0 * scale;
    let area_hit = 50.0 * scale;

    // Check NPCs first (higher priority)
    type Hit<'a> = (
        &'a NpcDotInfo,
        &'a MovementTarget,
        &'a CombatTarget,
        &'a SquadMembership,
        bool,
        f32,
    );
    let mut closest_npc: Option<Hit<'_>> = None;
    for (info, transform, movement, combat, member, loot) in &npcs {
        let dist = cursor.distance(transform.translation.truncate());
        if dist < npc_hit && (closest_npc.is_none() || dist < closest_npc.as_ref().unwrap().5) {
            closest_npc = Some((info, movement, combat, member, loot.is_some(), dist));
        }
    }
    if let Some((info, movement, combat, member, looting, _)) = closest_npc {
        // Look up the member's squad goal for the status string.
        let goal = squad_goals
            .get(member.squad)
            .map(|g| g.0.clone())
            .unwrap_or(cordon_core::entity::squad::Goal::Idle);
        *tooltip = TooltipContent::Npc {
            faction_icon: info.faction_icon.clone(),
            name: info.name.clone(),
            faction: info.faction.clone(),
            rank: info.rank.clone(),
            status: format_npc_status(movement, combat, looting, &goal),
        };
        return;
    }

    // Reset all area colors, then highlight hovered
    let mut hovered_area = false;
    for (data, transform, mat_handle) in &areas {
        let dist = cursor.distance(transform.translation.truncate());
        if dist < area_hit && !hovered_area {
            if let Some(m) = mats.get_mut(&mat_handle.0) {
                m.color = COLOR_AREA_HOVER;
            }
            let i = &data.0;
            *tooltip = TooltipContent::Area {
                faction_icon: i.faction_icon.clone(),
                name: i.name.clone(),
                creatures: i.creatures.clone(),
                creatures_tier: i.creatures_tier,
                radiation: i.radiation.clone(),
                radiation_tier: i.radiation_tier,
                hazard_icon: i.hazard_icon.clone(),
                hazard_image: i.hazard_image.clone(),
                hazard_count: i.hazard_count,
                loot: i.loot.clone(),
                loot_tier: i.loot_tier,
            };
            hovered_area = true;
        } else if let Some(m) = mats.get_mut(&mat_handle.0) {
            m.color = COLOR_AREA;
        }
    }
    if hovered_area {
        return;
    }

    *tooltip = TooltipContent::Hidden;
}

fn handle_npc_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<LaptopCamera>>,
    dots: Query<(Entity, &Transform), With<NpcMarker>>,
    mut selected: ResMut<SelectedNpc>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor_world) = cursor_world_pos(&windows, &cameras) else {
        return;
    };

    let hit_radius = 20.0;
    let mut closest: Option<(Entity, f32)> = None;
    for (entity, transform) in &dots {
        let pos = transform.translation.truncate();
        let dist = pos.distance(cursor_world);
        if dist <= hit_radius && (closest.is_none() || dist < closest.unwrap().1) {
            closest = Some((entity, dist));
        }
    }

    match closest {
        Some((entity, _)) if selected.0 == Some(entity) => selected.0 = None,
        Some((entity, _)) => selected.0 = Some(entity),
        None => selected.0 = None,
    }
}

fn update_npc_selection(
    selected: Res<SelectedNpc>,
    npc_assets: Res<NpcAssets>,
    mut dots: Query<
        (Entity, &SquadMembership, &mut MeshMaterial2d<ColorMaterial>),
        With<NpcMarker>,
    >,
) {
    if !selected.is_changed() {
        return;
    }

    // Find the selected NPC's squad entity.
    let selected_squad = selected.0.and_then(|entity| {
        dots.iter()
            .find(|(e, _, _)| *e == entity)
            .map(|(_, m, _)| m.squad)
    });

    for (entity, member, mut mat_handle) in &mut dots {
        let new_mat = match (selected.0, selected_squad) {
            (Some(sel), _) if sel == entity => npc_assets.selected_mat.clone(),
            (_, Some(sq)) if member.squad == sq => npc_assets.squad_mat.clone(),
            _ => npc_assets.default_mat.clone(),
        };
        mat_handle.0 = new_mat;
    }
}

fn deselect_or_exit(
    keys: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedNpc>,
    mut next_state: ResMut<NextState<PlayingState>>,
) {
    if keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::Escape) {
        if selected.0.is_some() && keys.just_pressed(KeyCode::Escape) {
            selected.0 = None;
        } else {
            *next_state = NextState::Pending(PlayingState::Bunker);
        }
    }
}

/// Attach laptop-side visuals to freshly-spawned NPC entities. Runs
/// after `spawn_population` every frame, but only does real work for
/// entities that were just given a `SquadMembership`.
#[allow(clippy::type_complexity)]
fn attach_npc_visuals(
    game_data: Res<GameDataResource>,
    npc_assets: Res<NpcAssets>,
    l10n: Option<Res<GameLocalization>>,
    squads: Query<(
        &SquadHomePosition,
        &SquadFormation,
        &cordon_sim::components::SquadMembers,
    )>,
    new_npcs: Query<
        (Entity, &FactionId, &Xp, &NpcNameComp, &SquadMembership),
        (With<NpcMarker>, Added<SquadMembership>),
    >,
    mut commands: Commands,
) {
    if new_npcs.iter().next().is_none() {
        return;
    }
    let data = &game_data.0;
    let empty_l10n = Localization::default();
    let l10n = l10n.as_ref().map(|r| &r.0).unwrap_or(&empty_l10n);

    for (entity, faction, xp, name, membership) in &new_npcs {
        let faction_str = faction.0.as_str();
        let faction_icon = faction_icon_str(Some(faction_str)).to_string();
        let faction_name = l10n_or(l10n, &format!("faction-{}", faction_str), faction_str);
        let name_display = resolve_npc_name(l10n, &name.0);
        let rank = xp.rank();
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
                let offsets = formation.0.slot_offsets(count);
                let slot = (membership.slot as usize).min(offsets.len() - 1);
                (home.0, Vec2::new(offsets[slot][0], offsets[slot][1]))
            }
            Err(_) => (Vec2::ZERO, Vec2::ZERO),
        };
        let spawn_pos = home + slot_offset;

        let is_military = matches!(faction_str, "garrison" | "order" | "mercenaries");
        let vision = Vision::for_npc(rank, is_military);

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
            MeshMaterial2d(npc_assets.default_mat.clone()),
            Transform::from_xyz(spawn_pos.x, spawn_pos.y, 0.5),
        ));
    }
}
