use bevy::prelude::*;

use super::components::LaptopObject;
use super::material::LaptopMaterial;
use crate::PlayingState;
use crate::bunker::camera::FpsCamera;
use crate::bunker::interaction::{Interact, Interactable};
use crate::bunker::resources::{CameraMode, LaptopPlacement};
use crate::laptop::LaptopScreenImage;

/// Screen plane position in the laptop body's local space. The
/// `interior/Laptop.glb` lid sits centred on X with the top edge
/// tilted back from the hinge. Tune these if the plane ends up
/// floating off the glass.
const SCREEN_LOCAL_POS: Vec3 = Vec3::new(0.0, 0.14, -0.168);

/// Tilt of the screen plane around local X. 0 = vertical;
/// negative values tip the top backwards toward the hinge side
/// (negative Z). Matches the open-laptop lid angle.
const SCREEN_TILT: f32 = -0.19;

/// Half-extent of the screen plane on the laptop body, in
/// metres. Height derived from the `LAPTOP_RT_*` render-target
/// aspect (4:3) so sampled UVs don't stretch the image.
const SCREEN_HALF_EXTENT: Vec2 = Vec2::new(0.17, 0.17 * 4.0 / 5.0);

pub(super) fn spawn_laptop(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    placement: Option<Res<LaptopPlacement>>,
    screen_image: Option<Res<LaptopScreenImage>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut laptop_materials: ResMut<Assets<LaptopMaterial>>,
) {
    let Some(placement) = placement else { return };
    let Some(screen_image) = screen_image else {
        return;
    };
    let scene: Handle<Scene> = asset_server.load("models/interior/Laptop.glb#Scene0");

    let laptop = commands
        .spawn((
            LaptopObject,
            Interactable {
                key: "interact-laptop".into(),
                enabled: true,
            },
            SceneRoot(scene),
            Transform::from_translation(placement.pos).with_rotation(placement.rot),
        ))
        .observe(
            |_trigger: On<Interact>,
             mut camera_mode: ResMut<CameraMode>,
             cam_q: Query<&Transform, With<FpsCamera>>| {
                // Start the zoom animation. The
                // `promote_at_laptop_to_state` system flips the
                // `PlayingState` to `Laptop` once the zoom
                // finishes (CameraMode::AtLaptop), so the
                // fullscreen UI swap waits for the end of the
                // animation.
                if let Ok(t) = cam_q.single() {
                    *camera_mode = CameraMode::ZoomingToLaptop {
                        saved_transform: *t,
                    };
                }
            },
        )
        .id();

    // Screen plane as a child of the laptop body. Rotates
    // around its own centre — if you steepen SCREEN_TILT, the
    // bottom edge pulls back into the body. Counter that by
    // increasing SCREEN_LOCAL_POS.z (pull the whole plane
    // toward the player) or by raising SCREEN_LOCAL_POS.y so
    // the tilt doesn't dip the bottom into the lid.
    let screen_mat = laptop_materials.add(LaptopMaterial::new(screen_image.0.clone()));
    let screen_mesh = meshes.add(Plane3d::new(Vec3::Z, SCREEN_HALF_EXTENT));
    let screen_tf = Transform::from_translation(SCREEN_LOCAL_POS)
        .with_rotation(Quat::from_rotation_x(SCREEN_TILT));
    let screen = commands
        .spawn((Mesh3d(screen_mesh), MeshMaterial3d(screen_mat), screen_tf))
        .id();
    commands.entity(laptop).add_child(screen);

    commands.remove_resource::<LaptopPlacement>();
}

/// Promote `CameraMode::AtLaptop` to `PlayingState::Laptop` so
/// the fullscreen UI swap only happens at the end of the zoom
/// animation. Without this, the render-target swap fires
/// instantly on click and the player sees a jarring cut before
/// the zoom has played out.
pub(super) fn promote_at_laptop_to_state(
    mode: Res<CameraMode>,
    state: Res<State<PlayingState>>,
    mut next_state: ResMut<NextState<PlayingState>>,
) {
    if !mode.is_changed() {
        return;
    }
    if matches!(*mode, CameraMode::AtLaptop { .. }) && !matches!(state.get(), PlayingState::Laptop)
    {
        next_state.set(PlayingState::Laptop);
    }
}
