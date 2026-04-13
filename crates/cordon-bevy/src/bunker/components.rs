use bevy::prelude::*;

#[derive(Component)]
pub struct FpsCamera;

#[derive(Component)]
pub struct LaptopObject;

#[derive(Component)]
pub struct InteractPrompt;

/// Marker for the small dome on the desk that admits a knocking
/// visitor. Spawned by the room module; clicked via mesh picking.
#[derive(Component)]
pub struct DoorButton;

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
