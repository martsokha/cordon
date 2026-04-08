//! Static map geometry: area disks, area borders, bunker marker.
//!
//! The map mesh is spawned once on `OnEnter(PlayingState::Laptop)`
//! from [`GameDataResource`]. Everything that lives on the map but
//! has its own lifecycle (NPCs, relics, anomalies, fog, visuals)
//! lives in sibling modules. This module only owns the immutable
//! "where is what" geometry plus a small set of shared types:
//!
//! - [`AreaCircle`] — per-area component holding radius + base colour
//! - [`AreaData`] / [`AreaTooltipInfo`] — pre-resolved tooltip strings
//! - [`Bunker`] — marker for the bunker dot
//! - [`MapSpawned`] — once-per-session latch
//! - [`MapWorldEntity`] — re-exported from `ui`; tags anything that
//!   should hide on non-map tabs
//!
//! Tooltip payload helpers live in the `tooltip` submodule so
//! neither the map spawner nor the hover system has to care about
//! localization mechanics.

pub mod relics;
pub mod tooltip;

use bevy::prelude::*;
use bevy_fluent::prelude::*;
use cordon_core::primitive::Tier;
use cordon_data::gamedata::GameDataResource;

use self::tooltip::build_area_info;
use crate::PlayingState;
pub use crate::laptop::ui::MapWorldEntity;
use crate::laptop::ui::{LaptopFont, spawn_ui};

const COLOR_AREA: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);
const COLOR_AREA_BORDER: Color = Color::srgba(1.0, 1.0, 1.0, 0.25);
pub(crate) const COLOR_AREA_HOVER: Color = Color::srgba(1.0, 1.0, 1.0, 0.15);

#[derive(Component)]
pub struct Bunker;

#[derive(Component)]
pub struct AreaCircle {
    /// Visual radius in world units — used by hover detection so
    /// the clickable zone matches the rendered disk exactly.
    pub radius: f32,
    /// The disk's default tint. Stored here so the hover system
    /// can restore each area to its own colour after un-hovering
    /// — hardcoding one "reset" colour doesn't work once each
    /// area owns a fresh per-entity `ColorMaterial`.
    pub base_color: Color,
}

#[derive(Component)]
pub struct AreaData(pub AreaTooltipInfo);

#[derive(Clone)]
pub struct AreaTooltipInfo {
    pub faction_icon: String,
    pub name: String,
    /// Pre-localized archetype label (e.g. "Settlement",
    /// "Anomaly Field").
    pub kind_label: String,
    /// For Settlements: the role label ("Outpost" / "Market").
    pub role: Option<String>,
    pub creatures: Option<(String, Tier)>,
    pub radiation: Option<(String, Tier)>,
    pub hazard_image: Option<String>,
    pub hazard_count: u8,
    pub loot: Option<(String, Tier)>,
}

#[derive(Resource)]
pub struct MapSpawned;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(relics::RelicsPlugin);
        app.add_systems(
            OnEnter(PlayingState::Laptop),
            spawn_map.run_if(not(resource_exists::<MapSpawned>)),
        );
    }
}

fn spawn_map(
    game_data: Res<GameDataResource>,
    laptop_font: Res<LaptopFont>,
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

        // Area disks and borders live *above* the fog overlay
        // (z > 4.5) so that once discovered they render cleanly
        // on top of the mist without the shader darkening them.
        // The fog only hides/darkens terrain, not markers —
        // markers are gated by the `Visibility` component that
        // `apply_fog` toggles on the `RevealedAreas` latch.
        let area_entity = commands
            .spawn((
                MapWorldEntity,
                AreaCircle { radius, base_color },
                AreaData(info),
                Mesh2d(meshes.add(Circle::new(radius))),
                MeshMaterial2d(area_material),
                Transform::from_xyz(x, y, 8.0),
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
                // Child transform: local offset from the disk so
                // the border draws just above the fill.
                Transform::from_xyz(0.0, 0.0, 0.08),
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
