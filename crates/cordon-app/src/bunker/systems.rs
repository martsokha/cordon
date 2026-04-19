use std::f32::consts::{FRAC_PI_2, PI};

use avian3d::prelude::*;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::prelude::*;
use bevy::render::view::Msaa;
use bevy::ui::UiTargetCamera;

use super::camera::FpsCamera;
use super::cctv::MonitorPlacement;
use super::interaction::InteractPrompt;
use super::resources::*;
use super::rooms;
use crate::fonts::UiFont;

pub(super) fn spawn_bunker(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut effects: ResMut<Assets<bevy_hanabi::EffectAsset>>,
    upgrades: Res<cordon_sim::resources::PlayerUpgrades>,
    game_data: Res<cordon_data::gamedata::GameDataResource>,
    font: Res<UiFont>,
) {
    let pal = Palette::new(&mut mats, &asset_server);
    let l = Layout::new();

    commands.insert_resource(MonitorPlacement {
        pos: Vec3::new(-l.hw + 0.15, l.h - 0.8, l.trade_z - 0.1),
        target: Vec3::new(0.0, 1.4, 0.0),
    });

    const TABLE_TOP: f32 = 1.037;
    commands.insert_resource(LaptopPlacement {
        pos: Vec3::new(0.0, TABLE_TOP, l.desk_z() - 0.15),
        rot: Quat::from_rotation_y(PI) * Quat::from_rotation_y(-FRAC_PI_2),
    });

    let fps_camera_entity = spawn_camera(&mut commands, &l);
    super::lighting::spawn_lighting(&mut commands, &mut meshes, &mut mats, &l);

    {
        let mut ctx = RoomCtx {
            commands: &mut commands,
            meshes: &mut meshes,
            mats: &mut mats,
            effects: &mut effects,
            pal: &pal,
            l: &l,
            upgrades: &upgrades,
            game_data: &game_data,
        };
        rooms::corridor::spawn(&mut ctx);
        rooms::entry::spawn(&mut ctx);
        rooms::command::spawn(&mut ctx);
        rooms::armory::spawn(&mut ctx);
        rooms::kitchen::spawn(&mut ctx);
        rooms::quarters::spawn(&mut ctx);
        rooms::hall::spawn(&mut ctx);
        rooms::infirmary::spawn(&mut ctx);
        rooms::workshop::spawn(&mut ctx);
        rooms::pipes::spawn(&mut ctx);
        rooms::antechamber::spawn(&mut ctx);
    }

    spawn_ui(&mut commands, fps_camera_entity, &font.0);
    commands.insert_resource(BunkerSpawned);
}

fn spawn_camera(commands: &mut Commands, l: &Layout) -> Entity {
    commands
        .spawn((
            FpsCamera,
            super::input::controller::StepTracker::default(),
            Camera3d::default(),
            Collider::capsule(
                super::input::controller::PLAYER_RADIUS,
                super::input::controller::PLAYER_HEIGHT,
            ),
            Transform::from_xyz(
                0.0,
                super::input::controller::CAMERA_EYE_Y,
                l.desk_z() - 0.5,
            )
            .looking_at(Vec3::new(0.0, 1.2, l.front_z), Vec3::Y),
            bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
            // Subtle bloom on emissive surfaces.
            bevy::post_process::bloom::Bloom {
                intensity: 0.08,
                ..default()
            },
            // Fog -- dark haze for depth.
            bevy::pbr::DistanceFog {
                color: Color::srgba(0.04, 0.04, 0.05, 1.0),
                falloff: bevy::pbr::FogFalloff::Linear {
                    start: 8.0,
                    end: 16.0,
                },
                ..default()
            },
            // Contact shadows in corners / under props. Huge
            // readability win for a boxy concrete interior — the
            // tight wall/floor seams gain proper grounding without
            // more geometry or lights. Requires MSAA off (the
            // SSAO pass writes to depth/normal prepass textures
            // that don't support multisampling).
            ScreenSpaceAmbientOcclusion::default(),
            Msaa::Off,
            bevy::audio::SpatialListener::default(),
        ))
        .id()
}

fn spawn_ui(commands: &mut Commands, fps_camera_entity: Entity, font: &Handle<Font>) {
    // Interaction prompt (below crosshair), centred horizontally.
    commands
        .spawn((
            InteractPrompt,
            UiTargetCamera(fps_camera_entity),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Percent(55.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_child((
            Text::new(""),
            TextFont {
                font: font.clone(),
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        ));

    // Crosshair dot.
    commands
        .spawn((
            UiTargetCamera(fps_camera_entity),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Percent(50.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(50),
        ))
        .with_child((
            Text::new("•"),
            TextFont {
                font: font.clone(),
                font_size: 6.0,
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
        ));
}
