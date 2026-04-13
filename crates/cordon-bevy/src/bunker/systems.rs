use avian3d::prelude::*;
use bevy::camera::{ClearColorConfig, ImageRenderTarget, RenderTarget};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::ui::UiTargetCamera;

use super::cctv_material::CctvMaterial;
use super::components::*;
use super::geometry;
use super::resources::*;
use super::rooms;

/// Render texture resolution. Bumped to 512×288 so the visitor
/// sprite stays legible on the monitor; the shader applies the
/// scanlines/grain look on top of the higher-res sample.
pub(super) const CCTV_RESOLUTION: (u32, u32) = (512, 288);

pub(super) fn start_laptop_zoom(
    camera_q: Query<&Transform, With<FpsCamera>>,
    mut mode: ResMut<CameraMode>,
) {
    if let Ok(transform) = camera_q.single() {
        *mode = CameraMode::ZoomingToLaptop {
            saved_transform: *transform,
        };
    }
}

pub(super) fn start_free_look(
    mut mode: ResMut<CameraMode>,
    mut laptop_cam: Query<&mut Camera, With<crate::laptop::LaptopCamera>>,
    mut cursor_q: Query<&mut bevy::window::CursorOptions>,
) {
    let saved = match &*mode {
        CameraMode::AtLaptop { saved_transform }
        | CameraMode::ZoomingToLaptop { saved_transform } => Some(*saved_transform),
        _ => None,
    };
    if let Some(t) = saved {
        *mode = CameraMode::Returning(t);
        for mut cam in &mut laptop_cam {
            cam.is_active = false;
        }
        for mut cursor in &mut cursor_q {
            cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
            cursor.visible = false;
        }
    }
}

pub(super) fn animate_camera(
    // Bunker camera animation is player-facing, not sim state —
    // use real time so accelerating the sim doesn't speed up the
    // laptop-to-bunker return lerp.
    time: Res<Time<Real>>,
    mut mode: ResMut<CameraMode>,
    mut camera_q: Query<&mut Transform, With<FpsCamera>>,
    mut laptop_cam: Query<&mut Camera, (With<crate::laptop::LaptopCamera>, Without<FpsCamera>)>,
    mut cursor_q: Query<&mut bevy::window::CursorOptions>,
) {
    let dt = time.delta_secs();
    let factor = 1.0 - (-CAMERA_LERP_SPEED * dt).exp();

    let Ok(mut transform) = camera_q.single_mut() else {
        return;
    };

    match mode.clone() {
        CameraMode::Free => {}
        CameraMode::Returning(saved) => {
            transform.translation = transform.translation.lerp(saved.translation, factor);
            transform.rotation = transform.rotation.slerp(saved.rotation, factor);
            // The visitor-return case only changes rotation (the
            // player never moved), so a translation-only threshold
            // would flip back to Free on the very first frame
            // before the slerp had any visible effect. Check both.
            let pos_done = transform.translation.distance(saved.translation) < 0.01;
            let rot_done = transform.rotation.dot(saved.rotation).abs() > 0.9999;
            if pos_done && rot_done {
                *transform = saved;
                *mode = CameraMode::Free;
            }
        }
        CameraMode::ZoomingToLaptop { saved_transform } => {
            let target_rot = Transform::from_translation(LAPTOP_VIEW_POS)
                .looking_at(LAPTOP_VIEW_TARGET, Vec3::Y)
                .rotation;

            transform.translation = transform.translation.lerp(LAPTOP_VIEW_POS, factor);
            transform.rotation = transform.rotation.slerp(target_rot, factor);

            if transform.translation.distance(LAPTOP_VIEW_POS) < 0.01 {
                *mode = CameraMode::AtLaptop { saved_transform };
                for mut cam in &mut laptop_cam {
                    cam.is_active = true;
                }
                for mut cursor in &mut cursor_q {
                    cursor.grab_mode = bevy::window::CursorGrabMode::None;
                    cursor.visible = true;
                }
            }
        }
        CameraMode::AtLaptop { .. } => {
            let target_rot = Transform::from_translation(LAPTOP_VIEW_POS)
                .looking_at(LAPTOP_VIEW_TARGET, Vec3::Y)
                .rotation;
            transform.translation = LAPTOP_VIEW_POS;
            transform.rotation = target_rot;
        }
        CameraMode::LookingAt { target, .. } => {
            // Rotation only — player stays put. Smoothly slerp the
            // current rotation toward facing the visitor.
            let target_rot = Transform::from_translation(transform.translation)
                .looking_at(target, Vec3::Y)
                .rotation;
            transform.rotation = transform.rotation.slerp(target_rot, factor);
        }
        CameraMode::AtCctv { .. } => {
            // The CCTV camera takes over the window during fullscreen
            // mode. The FPS camera doesn't move; the cctv plugin's
            // `apply_cctv_fullscreen` system handles the swap.
        }
    }
}

pub(super) fn enable_bunker_camera(mut camera_q: Query<&mut Camera, With<FpsCamera>>) {
    for mut cam in &mut camera_q {
        cam.is_active = true;
    }
}

/// Spawn the antechamber room, the CCTV camera, the bunker monitor,
/// and the shared image asset on first bunker entry.
pub(super) fn spawn_cctv(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    spawned: Option<Res<CctvImage>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut cctv_materials: ResMut<Assets<CctvMaterial>>,
) {
    if spawned.is_some() {
        return;
    }

    // Render-target image. The helper sets the right texture
    // descriptor (TEXTURE_BINDING | COPY_DST | RENDER_ATTACHMENT)
    // so the GPU can both render to it and sample it as a texture.
    let image = Image::new_target_texture(
        CCTV_RESOLUTION.0,
        CCTV_RESOLUTION.1,
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    let image_handle = images.add(image);
    commands.insert_resource(CctvImage(image_handle.clone()));

    rooms::antechamber::spawn(
        &mut commands,
        &mut meshes,
        &mut std_materials,
        &asset_server,
    );
    spawn_cctv_camera(&mut commands, image_handle.clone());
    spawn_cctv_monitor(
        &mut commands,
        &mut meshes,
        &mut std_materials,
        &mut cctv_materials,
        image_handle,
    );
}

/// Spawn the camera entity that renders the antechamber into our
/// shared image. Order is `-1` so it executes before the main
/// bunker camera each frame, ensuring the texture is fresh when
/// the bunker monitor samples it.
///
/// `RenderTarget` is a separate component in Bevy 0.18 (not a
/// field on `Camera`), so we attach it as a sibling. The fullscreen
/// toggle later swaps the component on the same entity via
/// `commands.entity(...).insert(...)`.
fn spawn_cctv_camera(commands: &mut Commands, image: Handle<Image>) {
    commands.spawn((
        CctvCamera,
        Camera3d::default(),
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.04, 0.04, 0.05)),
            ..default()
        },
        // Wide-angle perspective ≈ 80° vertical fov — sells the
        // CCTV "fisheye" feel, with the visitor centered and the
        // walls bowing in slightly.
        Projection::Perspective(PerspectiveProjection {
            fov: 1.4,
            ..default()
        }),
        RenderTarget::Image(ImageRenderTarget {
            handle: image,
            scale_factor: 1.0,
        }),
        Transform::from_translation(rooms::antechamber::CCTV_CAMERA_POS)
            .looking_at(rooms::antechamber::ANTECHAMBER_VISITOR_POS, Vec3::Y),
    ));
}

/// Spawn the bunker-side monitor: a small plane in the upper
/// corner of the desk room, tilted down toward where the player
/// would sit. The screen mesh uses [`CctvMaterial`] so the WGSL
/// shader can sample the camera feed and apply the scanlines /
/// vignette / phosphor look.
fn spawn_cctv_monitor(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    std_materials: &mut Assets<StandardMaterial>,
    cctv_materials: &mut Assets<CctvMaterial>,
    image: Handle<Image>,
) {
    // Bunker geometry constants from `room`: walls at
    // x = ±2.0, ceiling at h ≈ 2.4. The monitor sits in the
    // front-right corner just before the trade grate (z ≈ 1.5),
    // mounted high under the ceiling and tilted down toward the
    // player's chair. The bunker camera spawns facing world +z,
    // so the player's *screen-right* is world -x — hence the
    // negative x for "right".
    let monitor_pos = Vec3::new(-1.85, 2.15, 1.4);
    let monitor_target = Vec3::new(0.0, 1.4, 0.0);

    let screen_mat = cctv_materials.add(CctvMaterial {
        effect_strength: 1.0,
        _pad1: 0.0,
        _pad2: 0.0,
        _pad3: 0.0,
        feed: image,
    });

    // Spawn a small black bezel cuboid behind the screen so the
    // monitor reads as a real object on the wall, not a flat
    // floating glow patch. The screen plane is offset slightly
    // toward the player so it doesn't z-fight with the bezel.
    let bezel_mat = std_materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.06),
        perceptual_roughness: 0.4,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.36, 0.22, 0.05))),
        MeshMaterial3d(bezel_mat),
        Transform::from_translation(monitor_pos).looking_at(monitor_target, Vec3::Y),
    ));

    let dir = (monitor_target - monitor_pos).normalize_or_zero();
    // The screen rendering is mirrored L-R because the camera that
    // films the antechamber and the plane that displays the texture
    // sit on opposite sides of the same image, so the camera's
    // "right" lands on the player's "left" through naive UV
    // sampling. A negative-x scale on the monitor transform
    // mirrors the texture without touching shadows or winding.
    let mut monitor_transform =
        Transform::from_translation(monitor_pos + dir * 0.03).looking_at(monitor_target, Vec3::Y);
    monitor_transform.scale.x = -1.0;
    commands.spawn((
        CctvMonitor,
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(0.16, 0.09)))),
        MeshMaterial3d(screen_mat),
        monitor_transform,
    ));
}

/// Spawn the fullscreen overlay plane once the CCTV image resource
/// exists. It uses the same `CctvMaterial` as the corner monitor so
/// the full CRT effect stack applies to fullscreen mode — the CCTV
/// camera stays rendering to the shared image as normal.
///
/// The plane isn't parented to the camera because Bevy camera
/// children don't always render reliably; instead we follow the FPS
/// camera's transform each frame in `follow_fps_camera`.
pub(super) fn ensure_fullscreen_plane(
    mut commands: Commands,
    cctv_image: Option<Res<CctvImage>>,
    mut cctv_materials: ResMut<Assets<CctvMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    existing: Query<(), With<CctvFullscreenPlane>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(cctv_image) = cctv_image else { return };

    // Effect strength 0.25: at fullscreen scale every CRT effect
    // reads much stronger than on the small corner monitor, so we
    // dial the shader way back to just a hint.
    let mat = cctv_materials.add(CctvMaterial {
        effect_strength: 0.25,
        _pad1: 0.0,
        _pad2: 0.0,
        _pad3: 0.0,
        feed: cctv_image.0.clone(),
    });

    // Plane sized to match the CCTV image aspect ratio (16:9) and
    // fit exactly inside the FPS camera's frustum at z = -0.2. With
    // the default 45° vertical FOV:
    //   height = 2 * 0.2 * tan(22.5°) ≈ 0.166
    //   width  = height * 16/9       ≈ 0.294
    // `Plane3d::new` takes *half-size*, so we halve those.
    commands.spawn((
        CctvFullscreenPlane,
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(0.147, 0.083)))),
        MeshMaterial3d(mat),
        Transform::default(),
        Visibility::Hidden,
    ));
}

/// Keep the fullscreen plane glued 0.2 units in front of the FPS
/// camera (facing it) while visible, and park it far out of the
/// bunker when hidden. The far-park is important: even hidden, a
/// PBR plane sitting in front of the camera can cast shadows or
/// otherwise influence lighting calculations, which showed up as
/// a diagonal crescent of darkness dragging across the bunker as
/// the player looked around.
pub(super) fn follow_fps_camera(
    camera_mode: Res<CameraMode>,
    fps_q: Query<&GlobalTransform, (With<FpsCamera>, Without<CctvFullscreenPlane>)>,
    mut plane_q: Query<&mut Transform, With<CctvFullscreenPlane>>,
) {
    let Ok(mut plane_tf) = plane_q.single_mut() else {
        return;
    };
    if !matches!(*camera_mode, CameraMode::AtCctv { .. }) {
        // Park far below the antechamber so it can't influence
        // anything in the bunker scene.
        *plane_tf = Transform::from_xyz(0.0, -200.0, 0.0);
        return;
    }
    let Ok(fps_tf) = fps_q.single() else {
        return;
    };
    let fps = fps_tf.compute_transform();
    let forward = fps.forward().as_vec3();
    let pos = fps.translation + forward * 0.2;
    // Face the camera: plane's +Z (its normal) should point back at
    // the camera, i.e. opposite the camera's forward.
    let look_target = pos - forward;
    *plane_tf = Transform::from_translation(pos).looking_at(look_target, Vec3::Y);
    // Mirror U so the feed orientation matches the corner monitor.
    plane_tf.scale.x = -1.0;
}

/// Show or hide the fullscreen plane based on [`CameraMode`]
/// transitions. The CCTV camera and FPS camera both keep their
/// normal render targets — no swapping.
pub(super) fn apply_cctv_fullscreen(
    camera_mode: Res<CameraMode>,
    mut plane_q: Query<&mut Visibility, With<CctvFullscreenPlane>>,
) {
    if !camera_mode.is_changed() {
        return;
    }
    let Ok(mut vis) = plane_q.single_mut() else {
        return;
    };
    *vis = if matches!(*camera_mode, CameraMode::AtCctv { .. }) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

pub(super) fn spawn_bunker(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let pal = Palette::new(&mut mats);
    let l = Layout::new();

    let fps_camera_entity = spawn_camera(&mut commands, &l);
    spawn_lighting(&mut commands, &asset_server, &l);
    spawn_corridor(&mut commands, &mut meshes, &pal, &l);

    let mut ctx = RoomCtx {
        commands: &mut commands,
        asset_server: &asset_server,
        meshes: &mut meshes,
        mats: &mut mats,
        pal: &pal,
        l: &l,
    };
    rooms::entry::spawn(&mut ctx);
    rooms::command::spawn(&mut ctx);
    rooms::armory::spawn(&mut ctx);
    rooms::kitchen::spawn(&mut ctx);
    rooms::quarters::spawn(&mut ctx);
    drop(ctx);

    spawn_ui(&mut commands, fps_camera_entity);
    commands.insert_resource(BunkerSpawned);
}

fn spawn_camera(commands: &mut Commands, l: &Layout) -> Entity {
    commands
        .spawn((
            FpsCamera,
            Camera3d::default(),
            Collider::capsule(
                super::input::controller::PLAYER_RADIUS,
                super::input::controller::PLAYER_HEIGHT,
            ),
            Transform::from_xyz(0.0, 1.6, l.desk_z() - 0.5)
                .looking_at(Vec3::new(0.0, 1.2, l.front_z), Vec3::Y),
            bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface,
            // Subtle bloom on emissive surfaces.
            bevy::post_process::bloom::Bloom {
                intensity: 0.08,
                ..default()
            },
            // Fog — dark haze for depth.
            bevy::pbr::DistanceFog {
                color: Color::srgba(0.04, 0.04, 0.05, 1.0),
                falloff: bevy::pbr::FogFalloff::Linear {
                    start: 8.0,
                    end: 16.0,
                },
                ..default()
            },
        ))
        .id()
}

fn spawn_lighting(commands: &mut Commands, asset_server: &AssetServer, l: &Layout) {
    use geometry::LightFixture;

    let warm = Color::srgb(1.0, 0.82, 0.50);
    let cool = Color::srgb(0.85, 0.9, 1.0);
    let dim_cool = Color::srgb(0.8, 0.85, 0.95);
    let white = Color::srgb(0.95, 0.95, 1.0);
    let dim_warm = Color::srgb(1.0, 0.75, 0.45);
    let lamp_warm = Color::srgb(1.0, 0.70, 0.35);
    let screen_green = Color::srgb(0.4, 0.7, 0.4);

    let fixtures = [
        // Command post — ceiling lamp pulled 1m back from the desk
        // so it illuminates the room, not just the table surface.
        LightFixture::ceiling(0.0, l.desk_z() - 1.0, l.h, 120000.0, warm, true),
        LightFixture::desk(Vec3::new(0.4, 0.95, l.desk_z() - 0.15), 8000.0, warm),
        LightFixture::screen(Vec3::new(0.0, 1.1, l.desk_z()), 6000.0, screen_green),
        // Entry
        LightFixture::ceiling(0.0, 3.0, l.h, 50000.0, cool, false),
        // Armory + T-junction — single light between them.
        LightFixture::ceiling(0.0, l.tj_north - 0.5, l.h, 50000.0, dim_cool, false),
        // Kitchen
        LightFixture::ceiling(
            l.kitchen_x_center(),
            l.tj_center(),
            l.h,
            45000.0,
            white,
            false,
        ),
        // Quarters
        LightFixture::ceiling(
            l.quarters_x_center(),
            l.tj_center(),
            l.h,
            15000.0,
            dim_warm,
            false,
        ),
        LightFixture::standing(
            l.quarters_x_center(),
            l.tj_center() - 0.5,
            18000.0,
            lamp_warm,
        ),
    ];

    for fixture in &fixtures {
        fixture.spawn(commands, asset_server);
    }
}

fn spawn_corridor(commands: &mut Commands, meshes: &mut Assets<Mesh>, pal: &Palette, l: &Layout) {
    use std::f32::consts::{FRAC_PI_2, PI};

    use geometry::*;

    // Floor + ceiling.
    let main_center_z = (l.front_z + l.back_z) / 2.0;
    let main_floor_half = Vec2::new(l.hw, (l.front_z - l.back_z) / 2.0);
    spawn_floor_ceiling(
        commands,
        meshes,
        pal.concrete_dark.clone(),
        Vec3::new(0.0, 0.0, main_center_z),
        main_floor_half,
        l.h,
    );

    // Front wall.
    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(0.0, l.hh(), l.front_z),
        Quat::from_rotation_y(PI),
        Vec2::new(l.hw, l.hh()),
    );

    // Left wall + kitchen doorframe.
    {
        let len = l.front_z - l.tj_north;
        let cz = (l.front_z + l.tj_north) / 2.0;
        spawn_wall(
            commands,
            meshes,
            pal.concrete.clone(),
            Vec3::new(-l.hw, l.hh(), cz),
            Quat::from_rotation_y(FRAC_PI_2),
            Vec2::new(len / 2.0, l.hh()),
        );
    }
    spawn_doorframe_x(
        commands,
        meshes,
        pal.concrete.clone(),
        -l.hw,
        l.tj_center(),
        l.side_door_width,
        l.opening_h(),
    );
    {
        let door_n = l.tj_center() + l.side_door_width / 2.0;
        let len = (l.tj_north - door_n).abs();
        let cz = (l.tj_north + door_n) / 2.0;
        if len > 0.1 {
            spawn_wall(
                commands,
                meshes,
                pal.concrete.clone(),
                Vec3::new(-l.hw, l.hh(), cz),
                Quat::from_rotation_y(FRAC_PI_2),
                Vec2::new(len / 2.0, l.hh()),
            );
        }
    }
    {
        let door_s = l.tj_center() - l.side_door_width / 2.0;
        let len = (door_s - l.back_z).abs();
        let cz = (door_s + l.back_z) / 2.0;
        if len > 0.1 {
            spawn_wall(
                commands,
                meshes,
                pal.concrete.clone(),
                Vec3::new(-l.hw, l.hh(), cz),
                Quat::from_rotation_y(FRAC_PI_2),
                Vec2::new(len / 2.0, l.hh()),
            );
        }
    }

    // Right wall + bedroom doorframe.
    {
        let len = l.front_z - l.tj_north;
        let cz = (l.front_z + l.tj_north) / 2.0;
        spawn_wall(
            commands,
            meshes,
            pal.concrete.clone(),
            Vec3::new(l.hw, l.hh(), cz),
            Quat::from_rotation_y(-FRAC_PI_2),
            Vec2::new(len / 2.0, l.hh()),
        );
    }
    spawn_doorframe_x(
        commands,
        meshes,
        pal.concrete.clone(),
        l.hw,
        l.tj_center(),
        l.side_door_width,
        l.opening_h(),
    );
    {
        let door_n = l.tj_center() + l.side_door_width / 2.0;
        let len = (l.tj_north - door_n).abs();
        let cz = (l.tj_north + door_n) / 2.0;
        if len > 0.1 {
            spawn_wall(
                commands,
                meshes,
                pal.concrete.clone(),
                Vec3::new(l.hw, l.hh(), cz),
                Quat::from_rotation_y(-FRAC_PI_2),
                Vec2::new(len / 2.0, l.hh()),
            );
        }
    }
    {
        let door_s = l.tj_center() - l.side_door_width / 2.0;
        let len = (door_s - l.back_z).abs();
        let cz = (door_s + l.back_z) / 2.0;
        if len > 0.1 {
            spawn_wall(
                commands,
                meshes,
                pal.concrete.clone(),
                Vec3::new(l.hw, l.hh(), cz),
                Quat::from_rotation_y(-FRAC_PI_2),
                Vec2::new(len / 2.0, l.hh()),
            );
        }
    }

    // Back wall.
    spawn_wall(
        commands,
        meshes,
        pal.concrete.clone(),
        Vec3::new(0.0, l.hh(), l.back_z),
        Quat::IDENTITY,
        Vec2::new(l.hw, l.hh()),
    );
}

fn spawn_ui(commands: &mut Commands, fps_camera_entity: Entity) {
    commands.spawn((
        InteractPrompt,
        UiTargetCamera(fps_camera_entity),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(55.0),
            ..default()
        },
        Text::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Visibility::Hidden,
    ));
}
