//! Laptop view: the Zone map with areas, bunker, and NPC dots.

mod environment;
pub(crate) mod fog;
mod input;
mod palette;
mod ui;
mod visuals;

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_fluent::prelude::*;
use cordon_core::entity::faction::RankScheme;
use cordon_core::entity::name::{NameFormat, NpcName};
use cordon_core::primitive::{HazardType, Tier};
use cordon_core::world::area::{AreaDef, AreaKind, SettlementRole};
use cordon_data::gamedata::GameDataResource;
use cordon_sim::behavior::{CombatTarget, FireState, MovementSpeed, MovementTarget, Vision};
use cordon_sim::components::{
    FactionId, NpcMarker, NpcNameComp, SquadFormation, SquadHomePosition, SquadMembership, Xp,
};

pub use self::palette::FactionPalette;
use crate::PlayingState;
use crate::locale::l10n_or;

pub struct LaptopPlugin;

impl Plugin for LaptopPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            input::InputPlugin,
            ui::UiPlugin,
            environment::EnvironmentPlugin,
            fog::FogPlugin,
            visuals::VisualsPlugin,
        ));
        app.insert_resource(SelectedNpc::default());
        app.add_systems(Startup, (setup_camera, init_npc_assets, init_relic_assets));

        // Build the faction palette as soon as game data is ready,
        // not on laptop entry — visuals must be ready to paint NPCs
        // that spawn while the player is still in the bunker.
        app.add_systems(
            OnEnter(crate::AppState::Playing),
            palette::build_faction_palette.run_if(not(resource_exists::<FactionPalette>)),
        );

        // First-time bootstrap of the static map geometry. Areas and
        // the bunker dot only need to be spawned once per session.
        app.add_systems(
            OnEnter(PlayingState::Laptop),
            (
                spawn_map.run_if(not(resource_exists::<MapSpawned>)),
                preload_relic_icons.run_if(not(resource_exists::<RelicIconAssets>)),
            ),
        );

        // Visual attachment must keep up with the sim regardless of
        // which view the player is currently looking at — `Added<T>`
        // only fires for one frame, so if these systems were gated
        // on `Laptop` state, NPCs and relics spawned in the bunker
        // would never receive their map visuals.
        app.add_systems(
            Update,
            (
                attach_npc_visuals.after(cordon_sim::plugin::SimSet::Spawn),
                attach_relic_visuals.after(cordon_sim::plugin::SimSet::Spawn),
            )
                .run_if(in_state(crate::AppState::Playing)),
        );

        // Interaction systems are only meaningful when the player is
        // looking at the map.
        app.add_systems(
            Update,
            (
                update_hover,
                handle_npc_click,
                update_npc_selection,
                deselect_or_exit,
            )
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

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

/// Shared mesh + material handles for world relics.
#[derive(Resource, Clone)]
pub struct RelicAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
}

/// Preloaded relic icon `Handle<Image>`s, keyed by item id. Filled
/// once on laptop entry by [`preload_relic_icons`] so the hover
/// system can hand the renderer a resolved handle without calling
/// `asset_server.load()` or formatting a path string on every tick.
#[derive(Resource, Default)]
pub struct RelicIconAssets {
    handles: HashMap<cordon_core::primitive::Id<cordon_core::item::Item>, Handle<Image>>,
}

impl RelicIconAssets {
    pub fn get(
        &self,
        id: &cordon_core::primitive::Id<cordon_core::item::Item>,
    ) -> Option<Handle<Image>> {
        self.handles.get(id).cloned()
    }
}

const COLOR_RELIC: Color = Color::srgb(0.3, 0.9, 1.0);

fn init_relic_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mesh = meshes.add(Circle::new(4.0));
    let material = materials.add(ColorMaterial::from_color(COLOR_RELIC));
    commands.insert_resource(RelicAssets { mesh, material });
}

/// Walk every relic `ItemDef` in the catalog and preload its icon
/// into a `Handle<Image>`, stored by item id in [`RelicIconAssets`].
///
/// Runs once on [`PlayingState::Laptop`] entry because that's the
/// first time both (a) the laptop UI is visible and (b) game data
/// is guaranteed loaded. 36 small PNGs is negligible to hold
/// resident.
fn preload_relic_icons(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game_data: Res<GameDataResource>,
) {
    let mut handles = HashMap::new();
    for (id, def) in &game_data.0.items {
        if matches!(def.data, cordon_core::item::ItemData::Relic(_)) {
            let path = format!("icons/relics/{}.png", id.as_str());
            handles.insert(id.clone(), asset_server.load(path));
        }
    }
    commands.insert_resource(RelicIconAssets { handles });
}

/// Attach map visuals to newly-spawned relic entities. The tooltip
/// payload is built on-demand by the hover system from the relic's
/// catalog def, so no per-entity info component is needed here.
fn attach_relic_visuals(
    assets: Res<RelicAssets>,
    new_relics: Query<Entity, Added<cordon_sim::components::RelicMarker>>,
    mut commands: Commands,
) {
    if new_relics.iter().next().is_none() {
        return;
    }
    for entity in &new_relics {
        commands.entity(entity).insert((
            MapWorldEntity,
            Mesh2d(assets.mesh.clone()),
            MeshMaterial2d(assets.material.clone()),
        ));
    }
}

pub use self::ui::MapWorldEntity;
use self::ui::map::{TooltipContent, cursor_world_pos};
use self::ui::{LaptopFont, spawn_ui};

#[derive(Component)]
struct Bunker;

#[derive(Component)]
struct AreaCircle {
    /// Visual radius in world units — used by hover detection so the
    /// clickable zone matches the rendered disk exactly.
    radius: f32,
    /// The disk's default tint. Stored here so the hover system can
    /// restore each area to its own colour after un-hovering —
    /// hardcoding one "reset" colour doesn't work once each area
    /// owns a fresh per-entity `ColorMaterial`.
    base_color: Color,
}

#[derive(Component)]
struct AreaData(AreaTooltipInfo);

#[derive(Clone)]
struct AreaTooltipInfo {
    faction_icon: String,
    name: String,
    /// Pre-localized archetype label (e.g. "Settlement", "Anomaly Field").
    kind_label: String,
    /// For Settlements: the role label ("Outpost"/"Market"). None otherwise.
    role: Option<String>,
    creatures: Option<(String, Tier)>,
    radiation: Option<(String, Tier)>,
    hazard_image: Option<String>,
    hazard_count: u8,
    loot: Option<(String, Tier)>,
}

#[derive(Component, Clone)]
struct NpcDotInfo {
    faction_icon: String,
    name: String,
    faction: String,
    rank: String,
}

#[derive(Resource, Default)]
pub(crate) struct SelectedNpc(pub Option<Entity>);

const COLOR_AREA: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);
const COLOR_AREA_BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.25);
const COLOR_AREA_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.15);
const COLOR_NPC_SELECTED: Color = Color::srgb(1.0, 0.9, 0.3);
const COLOR_NPC_SQUAD: Color = Color::srgb(0.7, 0.6, 0.25);

fn faction_icon_str(faction: Option<&str>) -> &'static str {
    match faction {
        Some("garrison") => "[G]",
        Some("syndicate") => "[S]",
        Some("institute") => "[I]",
        Some("devoted") => "[D]",
        Some("drifters") => "[d]",
        _ => "[?]",
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
    let tier_label =
        |t: Tier| -> (String, Tier) { (l10n_or(l10n, tier_key(&t), &format!("{:?}", t)), t) };
    let hazard_image = |h: &cordon_core::world::area::Hazard| -> String {
        match h.kind {
            HazardType::Chemical => "icons/hazards/chemical.png".to_string(),
            HazardType::Thermal => "icons/hazards/thermal.png".to_string(),
            HazardType::Electric => "icons/hazards/electric.png".to_string(),
            HazardType::Gravitational => "icons/hazards/gravitational.png".to_string(),
        }
    };
    let hazard_count = |t: Tier| -> u8 {
        match t {
            Tier::VeryLow => 1,
            Tier::Low => 2,
            Tier::Medium => 3,
            Tier::High => 4,
            Tier::VeryHigh => 5,
        }
    };

    let kind_key = match &area.kind {
        AreaKind::Settlement { .. } => "areakind-settlement",
        AreaKind::Wasteland { .. } => "areakind-wasteland",
        AreaKind::MutantLair { .. } => "areakind-mutant-lair",
        AreaKind::AnomalyField { .. } => "areakind-anomaly-field",
        AreaKind::Anchor { .. } => "areakind-anchor",
    };

    let role = match &area.kind {
        AreaKind::Settlement { role, .. } => Some({
            let key = match role {
                SettlementRole::Outpost => "settlement-role-outpost",
                SettlementRole::Market => "settlement-role-market",
            };
            l10n_or(l10n, key, key)
        }),
        _ => None,
    };

    let creatures = area.kind.creatures().map(tier_label);
    let radiation = area.kind.radiation().map(tier_label);
    let loot = area.kind.loot().map(tier_label);
    let (hazard_image, hazard_count_v) = match area.kind.hazard() {
        Some(h) => (Some(hazard_image(&h)), hazard_count(h.intensity)),
        None => (None, 0),
    };

    AreaTooltipInfo {
        faction_icon: faction_icon_str(area.kind.faction().map(|f| f.as_str())).to_string(),
        name: l10n_or(
            l10n,
            &format!("area-{}", area.id.as_str()),
            area.id.as_str(),
        ),
        kind_label: l10n_or(l10n, kind_key, kind_key),
        role,
        creatures,
        radiation,
        hazard_image,
        hazard_count: hazard_count_v,
        loot,
    }
}

/// Build a relic tooltip payload from its catalog def + relic data.
/// Called inline by the hover system — resolving strings on-demand
/// avoids duplicating them on every spawned relic entity, and means
/// localization updates apply immediately without rebuilding dots.
fn build_relic_tooltip(
    l10n: &Localization,
    icons: &RelicIconAssets,
    def: &cordon_core::item::ItemDef,
    data: &cordon_core::item::RelicData,
) -> TooltipContent {
    use cordon_core::item::{PassiveModifier, StatTarget};
    use cordon_core::primitive::{HazardType, Rarity};

    let name = l10n_or(l10n, &format!("item-{}", def.id.as_str()), def.id.as_str());
    let origin_key = match data.origin {
        HazardType::Chemical => "hazard-chemical",
        HazardType::Thermal => "hazard-thermal",
        HazardType::Electric => "hazard-electric",
        HazardType::Gravitational => "hazard-gravitational",
    };
    let origin = l10n_or(l10n, origin_key, origin_key);
    let rarity_key = match def.rarity {
        Rarity::Common => "rarity-common",
        Rarity::Uncommon => "rarity-uncommon",
        Rarity::Rare => "rarity-rare",
    };
    let rarity = l10n_or(l10n, rarity_key, rarity_key);

    let passives: Vec<String> = data
        .passive
        .iter()
        .map(|PassiveModifier { target, value }| {
            let (stat_key, fallback) = match target {
                StatTarget::MaxHealth => ("stat-max-health", "Max HP"),
                StatTarget::MaxStamina => ("stat-max-stamina", "Max Stamina"),
                StatTarget::MaxHunger => ("stat-max-hunger", "Max Hunger"),
                StatTarget::BallisticResistance => ("hazard-ballistic", "Ballistic"),
                StatTarget::RadiationResistance => ("hazard-radiation", "Radiation"),
                StatTarget::ChemicalResistance => ("hazard-chemical", "Chemical"),
                StatTarget::ThermalResistance => ("hazard-thermal", "Thermal"),
                StatTarget::ElectricResistance => ("hazard-electric", "Electric"),
                StatTarget::GravitationalResistance => ("hazard-gravitational", "Gravitational"),
            };
            let label = l10n_or(l10n, stat_key, fallback);
            let sign = if *value >= 0.0 { "+" } else { "" };
            format!("{label}: {sign}{value:.0}")
        })
        .collect();

    // Look up the preloaded handle. A missing icon (def id not in
    // the preload map) falls back to a default `Handle<Image>`,
    // which renders as Bevy's missing-asset placeholder.
    let icon = icons.get(&def.id).unwrap_or_default();

    TooltipContent::Relic {
        name,
        icon,
        origin,
        rarity,
        passives,
        triggered_count: data.triggered.len(),
    }
}

fn spawn_map(
    game_data: Res<GameDataResource>,
    laptop_font: Res<LaptopFont>,
    _asset_server: Res<AssetServer>,
    l10n: Option<Res<Localization>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let data = &game_data.0;
    let empty_l10n = Localization::default();
    let l10n = l10n.as_deref().unwrap_or(&empty_l10n);

    spawn_ui(&mut commands, &laptop_font.0);

    // Shared border material — safe to share because nothing ever
    // mutates the border color at runtime. Disk materials, by
    // contrast, each get their own handle (see the loop below)
    // because the hover system tints the *selected* disk, and a
    // shared handle would smear that tint across every area
    // sharing the material (all non-settlement areas, or all
    // settlements of the same faction).
    let border_mat = materials.add(ColorMaterial::from_color(COLOR_AREA_BORDER));

    for area in data.areas.values() {
        let x = area.location.x;
        let y = area.location.y;
        let radius = area.radius.value();
        let info = build_area_info(l10n, area);

        // All areas share the same neutral wash — no per-faction
        // tint on the map disk itself (faction is still readable
        // via the NPC dots standing inside it). Each area gets a
        // *fresh* ColorMaterial so the hover system can tint a
        // single disk without smearing across its neighbours.
        let base_color = COLOR_AREA;
        let area_material = materials.add(ColorMaterial::from_color(base_color));

        let area_entity = commands
            .spawn((
                MapWorldEntity,
                AreaCircle { radius, base_color },
                AreaData(info),
                Mesh2d(meshes.add(Circle::new(radius))),
                MeshMaterial2d(area_material),
                Transform::from_xyz(x, y, 0.01),
            ))
            .id();

        // Border annulus is a child of the disk so hierarchy
        // visibility propagation hides them together when fog of
        // war hides the area.
        let border = commands
            .spawn((
                MapWorldEntity,
                Mesh2d(meshes.add(Annulus::new(radius - 2.0, radius))),
                MeshMaterial2d(border_mat.clone()),
                // Child transform: local offset from the disk, which
                // already sits at (x, y, 0.01). The border just
                // needs a tiny z bump so it draws above the fill.
                Transform::from_xyz(0.0, 0.0, 0.09),
            ))
            .id();
        commands.entity(area_entity).add_child(border);
    }

    commands.spawn((
        MapWorldEntity,
        Bunker,
        Mesh2d(meshes.add(Circle::new(10.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::WHITE))),
        // Above the cloud layer (z=5) so the bunker stays visible.
        Transform::from_xyz(0.0, 0.0, 10.0),
    ));

    info!(
        "Laptop map: {} areas (NPCs spawned by cordon-sim)",
        data.areas.len()
    );

    commands.insert_resource(MapSpawned);
}

/// What the hover system currently believes the cursor is pointing
/// at. Persisted in a [`Local`] so we can detect *changes* and
/// only mutate `ColorMaterial.color`, `TooltipContent`, etc. when
/// the target actually changes — instead of rewriting everything
/// every frame and dirtying Bevy's change detection pipeline.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum HoverTarget {
    #[default]
    None,
    Area(Entity),
    Npc(Entity),
    Relic(Entity),
}

fn update_hover(
    mut last_target: Local<HoverTarget>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<LaptopCamera>>,
    cam_proj: Query<&Projection, With<LaptopCamera>>,
    game_data: Res<GameDataResource>,
    relic_icons: Option<Res<RelicIconAssets>>,
    l10n: Option<Res<Localization>>,
    areas: Query<(
        Entity,
        &AreaCircle,
        &AreaData,
        &Transform,
        &MeshMaterial2d<ColorMaterial>,
        &Visibility,
    )>,
    npcs: Query<
        (
            Entity,
            &NpcDotInfo,
            &Transform,
            &MovementTarget,
            &CombatTarget,
            &SquadMembership,
            Option<&cordon_sim::behavior::LootState>,
            &Visibility,
        ),
        With<NpcMarker>,
    >,
    relics: Query<
        (
            Entity,
            &cordon_sim::components::RelicItem,
            &Transform,
            &Visibility,
        ),
        With<cordon_sim::components::RelicMarker>,
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
    let relic_hit = 12.0 * scale;

    // ---- Pass 1: find the new hover target. Priority is relic >
    // NPC > area, so relics tucked inside dense anomaly zones stay
    // tooltip-reachable even when NPCs cross over the same pixel.
    let mut new_target = HoverTarget::None;

    let mut closest_relic: Option<(Entity, f32)> = None;
    for (entity, _item, transform, vis) in &relics {
        if matches!(vis, Visibility::Hidden) {
            continue;
        }
        let dist = cursor.distance(transform.translation.truncate());
        if dist < relic_hit && closest_relic.is_none_or(|(_, d)| dist < d) {
            closest_relic = Some((entity, dist));
        }
    }
    if let Some((entity, _)) = closest_relic {
        new_target = HoverTarget::Relic(entity);
    }

    if matches!(new_target, HoverTarget::None) {
        let mut closest_npc: Option<(Entity, f32)> = None;
        for (entity, _info, transform, _mvt, _cmb, _mem, _loot, vis) in &npcs {
            if matches!(vis, Visibility::Hidden) {
                continue;
            }
            let dist = cursor.distance(transform.translation.truncate());
            if dist < npc_hit && closest_npc.is_none_or(|(_, d)| dist < d) {
                closest_npc = Some((entity, dist));
            }
        }
        if let Some((entity, _)) = closest_npc {
            new_target = HoverTarget::Npc(entity);
        }
    }

    if matches!(new_target, HoverTarget::None) {
        for (entity, circle, _data, transform, _mat, vis) in &areas {
            if matches!(vis, Visibility::Hidden) {
                continue;
            }
            let dist = cursor.distance(transform.translation.truncate());
            if dist < circle.radius {
                new_target = HoverTarget::Area(entity);
                break;
            }
        }
    }

    // ---- Early out: target unchanged → don't touch the tooltip or
    // any materials. This is the hot path most frames. Writes to
    // `ResMut<TooltipContent>` or `ColorMaterial.color` would mark
    // them as changed and wake Bevy's whole change-detection and
    // render-upload pipeline each frame; skipping them here is most
    // of the performance win.
    if *last_target == new_target {
        return;
    }

    // ---- Target changed. Repaint the *old* hovered area back to
    // its base colour (if the previous target was an area), and
    // paint the *new* area with the hover colour (if the new target
    // is an area). All other areas keep whatever colour they
    // already had — no mass rewrite.
    if let HoverTarget::Area(prev) = *last_target
        && let Ok((_, circle, _, _, mat_handle, _)) = areas.get(prev)
        && let Some(m) = mats.get_mut(&mat_handle.0)
    {
        m.color = circle.base_color;
    }
    if let HoverTarget::Area(curr) = new_target
        && let Ok((_, _, _, _, mat_handle, _)) = areas.get(curr)
        && let Some(m) = mats.get_mut(&mat_handle.0)
    {
        m.color = COLOR_AREA_HOVER;
    }

    // ---- Build the new tooltip content. This runs only on the
    // transition frame, so the string allocations here are cheap.
    *tooltip = match new_target {
        HoverTarget::None => TooltipContent::Hidden,
        HoverTarget::Relic(entity) => {
            // Resolve relic def + icon, falling back to Hidden if
            // anything is missing (shouldn't happen but we don't
            // want to stick the tooltip on stale state).
            let mut out = TooltipContent::Hidden;
            if let Ok((_, item, _, _)) = relics.get(entity)
                && let Some(def) = game_data.0.items.get(&item.0.def_id)
                && let cordon_core::item::ItemData::Relic(relic_data) = &def.data
                && let Some(icons) = relic_icons.as_deref()
            {
                let empty_l10n = Localization::default();
                let l10n = l10n.as_deref().unwrap_or(&empty_l10n);
                out = build_relic_tooltip(l10n, icons, def, relic_data);
            }
            out
        }
        HoverTarget::Npc(entity) => {
            let mut out = TooltipContent::Hidden;
            if let Ok((_, info, _, movement, combat, member, loot, _)) = npcs.get(entity) {
                let goal = squad_goals
                    .get(member.squad)
                    .map(|g| g.0.clone())
                    .unwrap_or(cordon_core::entity::squad::Goal::Idle);
                out = TooltipContent::Npc {
                    faction_icon: info.faction_icon.clone(),
                    name: info.name.clone(),
                    faction: info.faction.clone(),
                    rank: info.rank.clone(),
                    status: format_npc_status(movement, combat, loot.is_some(), &goal),
                };
            }
            out
        }
        HoverTarget::Area(entity) => {
            let mut out = TooltipContent::Hidden;
            if let Ok((_, _, data, _, _, _)) = areas.get(entity) {
                let i = &data.0;
                out = TooltipContent::Area {
                    faction_icon: i.faction_icon.clone(),
                    name: i.name.clone(),
                    kind_label: i.kind_label.clone(),
                    role: i.role.clone(),
                    creatures: i.creatures.clone(),
                    radiation: i.radiation.clone(),
                    hazard_image: i.hazard_image.clone(),
                    hazard_count: i.hazard_count,
                    loot: i.loot.clone(),
                };
            }
            out
        }
    };

    *last_target = new_target;
}

fn handle_npc_click(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<LaptopCamera>>,
    dots: Query<(Entity, &Transform, &Visibility), With<NpcMarker>>,
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
    for (entity, transform, vis) in &dots {
        // Fog-hidden NPCs aren't clickable.
        if matches!(vis, Visibility::Hidden) {
            continue;
        }
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

/// Marker for the ring child entity that draws the selection /
/// squadmate outline around an NPC dot. Keeping it as its own
/// component lets the selection system find and despawn the ring
/// without touching anything else parented to the NPC.
#[derive(Component)]
struct SelectionRing;

fn update_npc_selection(
    selected: Res<SelectedNpc>,
    npc_assets: Res<NpcAssets>,
    mut commands: Commands,
    dots: Query<(Entity, &SquadMembership, Option<&Children>), With<NpcMarker>>,
    rings: Query<Entity, With<SelectionRing>>,
) {
    if !selected.is_changed() {
        return;
    }

    // Despawn all existing rings first. Rings are children of their
    // NPC, so despawning the ring entity is enough — the NPC stays.
    for ring in &rings {
        commands.entity(ring).despawn();
    }

    // Nothing selected → nothing to draw.
    let Some(selected_entity) = selected.0 else {
        return;
    };

    // Find the selected NPC's squad so we can mark its squadmates.
    let Some(selected_squad) = dots
        .iter()
        .find(|(e, _, _)| *e == selected_entity)
        .map(|(_, m, _)| m.squad)
    else {
        return;
    };

    // Spawn a ring under each matching NPC. The focused NPC gets the
    // thicker "selected" ring; squadmates get the thinner one. Rings
    // sit at local z = 0.5 so they render just above the dot (which
    // is at 0) but below any later overlay.
    for (entity, member, _) in &dots {
        let (mesh, mat) = if entity == selected_entity {
            (
                npc_assets.selected_ring_mesh.clone(),
                npc_assets.selected_ring_mat.clone(),
            )
        } else if member.squad == selected_squad {
            (
                npc_assets.squad_ring_mesh.clone(),
                npc_assets.squad_ring_mat.clone(),
            )
        } else {
            continue;
        };
        let ring = commands
            .spawn((
                SelectionRing,
                Mesh2d(mesh),
                MeshMaterial2d(mat),
                Transform::from_xyz(0.0, 0.0, 0.5),
            ))
            .id();
        commands.entity(entity).add_child(ring);
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
fn attach_npc_visuals(
    game_data: Res<GameDataResource>,
    npc_assets: Res<NpcAssets>,
    palette: Res<FactionPalette>,
    l10n: Option<Res<Localization>>,
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
    let l10n = l10n.as_deref().unwrap_or(&empty_l10n);

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

        let is_military = matches!(faction_str, "garrison");
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
            MeshMaterial2d(palette.dot(&faction.0)),
            // z=10 keeps NPC dots (and the corpse X children that
            // ride the same transform) above the cloud layer at z=5.
            Transform::from_xyz(spawn_pos.x, spawn_pos.y, 10.0),
        ));
    }
}
