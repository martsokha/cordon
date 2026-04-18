use bevy::prelude::*;

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
pub(crate) struct CctvFullscreenPlane;

/// Resource holding the CCTV image handle so other systems (the
/// monitor material, the fullscreen-toggle) can refer to it
/// without re-querying the camera every frame.
#[derive(Resource, Clone)]
pub struct CctvImage(pub Handle<Image>);

/// Where the CCTV monitor should be placed in the bunker. Inserted
/// by the bunker orchestrator so the CCTV module doesn't need to
/// know about Layout dimensions.
#[derive(Resource)]
pub struct MonitorPlacement {
    pub pos: Vec3,
    pub target: Vec3,
}
