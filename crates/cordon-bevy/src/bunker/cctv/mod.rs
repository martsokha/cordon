//! CCTV preview: a hidden antechamber rendered to a texture and
//! displayed on a monitor mounted in the corner of the bunker
//! ceiling.
//!
//! The antechamber is a small sealed room placed far below the
//! main bunker geometry (`y = -50`) so the player's main camera
//! can never accidentally see it. A dedicated [`CctvCamera`]
//! lives inside the antechamber, looking diagonally down at the
//! visitor's standing position. Its [`RenderTarget`] is an
//! [`Image`] asset whose handle is sampled by the bunker monitor
//! material — making the monitor a live feed.
//!
//! The visitor module spawns its `KnockingPreview` sprite at
//! [`ANTECHAMBER_VISITOR_POS`] when state becomes `Knocking`,
//! and despawns it when state leaves `Knocking`.
//!
//! Pressing **E** while looking at the bunker monitor toggles a
//! fullscreen view: the [`CctvCamera`] swaps its render target
//! from `Image` to `Window`, the main FPS camera goes inactive,
//! and the player can study the feed up close. Esc/E exits.

mod antechamber;
mod material;

use bevy::camera::{ClearColorConfig, ImageRenderTarget, RenderTarget};
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

pub use self::antechamber::ANTECHAMBER_VISITOR_POS;
use self::antechamber::CCTV_CAMERA_POS;
pub use self::material::CctvMaterial;
use super::{CameraMode, FpsCamera};
use crate::PlayingState;

pub struct CctvPlugin;

impl Plugin for CctvPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<CctvMaterial>::default());
        app.add_systems(OnEnter(PlayingState::Bunker), spawn_cctv);
        app.add_systems(
            Update,
            (
                ensure_fullscreen_plane,
                apply_cctv_fullscreen,
                follow_fps_camera,
            )
                .chain()
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}

/// Render texture resolution. Bumped to 512×288 so the visitor
/// sprite stays legible on the monitor; the shader applies the
/// scanlines/grain look on top of the higher-res sample.
const CCTV_RESOLUTION: (u32, u32) = (512, 288);

/// Marker for the CCTV camera entity.
#[derive(Component)]
pub struct CctvCamera;

/// Marker for the CCTV monitor mesh in the bunker corner.
#[derive(Component)]
pub struct CctvMonitor;

/// Marker for the fullscreen CCTV plane parented to the FPS camera.
/// Normally hidden; made visible while the player is in
/// [`CameraMode::AtCctv`] so the feed (plus shader effects) fills
/// the screen. Parented to the camera so it always sits directly
/// in front, regardless of camera movement.
#[derive(Component)]
struct CctvFullscreenPlane;

/// Resource holding the CCTV image handle so other systems (the
/// monitor material, the fullscreen-toggle) can refer to it
/// without re-querying the camera every frame.
#[derive(Resource, Clone)]
pub struct CctvImage(pub Handle<Image>);

/// Spawn the antechamber room, the CCTV camera, the bunker monitor,
/// and the shared image asset on first bunker entry.
fn spawn_cctv(
    mut commands: Commands,
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

    antechamber::spawn(&mut commands, &mut meshes, &mut std_materials);
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
        Transform::from_translation(CCTV_CAMERA_POS).looking_at(ANTECHAMBER_VISITOR_POS, Vec3::Y),
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
fn ensure_fullscreen_plane(
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
fn follow_fps_camera(
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
fn apply_cctv_fullscreen(
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
