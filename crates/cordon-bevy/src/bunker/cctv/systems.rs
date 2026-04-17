use bevy::camera::{ClearColorConfig, ImageRenderTarget, RenderTarget};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use super::components::*;
use super::materials::CctvMaterial;
use crate::bunker::components::FpsCamera;
use crate::bunker::interaction::{Interact, Interactable};
use crate::bunker::resources::CameraMode;

pub(crate) const CCTV_WIDTH: u32 = 512;
pub(crate) const CCTV_HEIGHT: u32 = 288;

pub(super) fn spawn_cctv(
    mut commands: Commands,
    spawned: Option<Res<CctvImage>>,
    placement: Option<Res<MonitorPlacement>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut cctv_materials: ResMut<Assets<CctvMaterial>>,
) {
    if spawned.is_some() {
        return;
    }
    let Some(placement) = placement else { return };

    let image =
        Image::new_target_texture(CCTV_WIDTH, CCTV_HEIGHT, TextureFormat::Rgba8UnormSrgb, None);
    let image_handle = images.add(image);
    commands.insert_resource(CctvImage(image_handle.clone()));

    spawn_cctv_camera(&mut commands, image_handle.clone());
    let monitor = super::bundles::spawn_monitor(
        &mut commands,
        &mut meshes,
        &mut std_materials,
        &mut cctv_materials,
        image_handle,
        placement.pos,
        placement.target,
    );
    commands
        .entity(monitor)
        .insert(Interactable {
            key: "interact-cctv".into(),
            enabled: true,
        })
        .observe(
            |_trigger: On<Interact>,
             mut camera_mode: ResMut<CameraMode>,
             cam_q: Query<&Transform, With<FpsCamera>>| {
                if let Ok(t) = cam_q.single() {
                    *camera_mode = CameraMode::AtCctv {
                        saved_transform: *t,
                    };
                }
            },
        );
}

fn spawn_cctv_camera(commands: &mut Commands, image: Handle<Image>) {
    commands.spawn((
        CctvCamera,
        Camera3d::default(),
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.04, 0.04, 0.05)),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 1.4,
            ..default()
        }),
        RenderTarget::Image(ImageRenderTarget {
            handle: image,
            scale_factor: 1.0,
        }),
        Transform::from_translation(crate::bunker::resources::CCTV_CAMERA_POS)
            .looking_at(crate::bunker::resources::ANTECHAMBER_VISITOR_POS, Vec3::Y),
    ));
}

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

    let mat = cctv_materials.add(CctvMaterial::new(cctv_image.0.clone(), 0.25));

    commands.spawn((
        CctvFullscreenPlane,
        Mesh3d(meshes.add(Plane3d::new(Vec3::Z, Vec2::new(0.147, 0.083)))),
        MeshMaterial3d(mat),
        Transform::default(),
        Visibility::Hidden,
    ));
}

pub(super) fn follow_fps_camera(
    camera_mode: Res<CameraMode>,
    fps_q: Query<&GlobalTransform, (With<FpsCamera>, Without<CctvFullscreenPlane>)>,
    mut plane_q: Query<&mut Transform, With<CctvFullscreenPlane>>,
) {
    let Ok(mut plane_tf) = plane_q.single_mut() else {
        return;
    };
    if !matches!(*camera_mode, CameraMode::AtCctv { .. }) {
        *plane_tf = Transform::from_xyz(0.0, -200.0, 0.0);
        return;
    }
    let Ok(fps_tf) = fps_q.single() else {
        return;
    };
    let fps = fps_tf.compute_transform();
    let forward = fps.forward().as_vec3();
    let pos = fps.translation + forward * 0.2;
    let look_target = pos - forward;
    *plane_tf = Transform::from_translation(pos).looking_at(look_target, Vec3::Y);
    plane_tf.scale.x = -1.0;
}

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
