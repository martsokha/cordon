use bevy::prelude::*;

#[derive(Resource)]
pub struct BunkerSpawned;

#[derive(Resource)]
pub struct MovementLocked;

/// When present as a resource, all interactions are blocked. The
/// visitor module inserts this while a visitor is inside the bunker
/// so the player can't escape mid-conversation.
#[derive(Resource)]
pub struct InteractionLocked;

#[derive(Resource)]
pub struct LaptopPlacement {
    pub pos: Vec3,
    pub rot: Quat,
}

pub const ANTECHAMBER_VISITOR_POS: Vec3 = Vec3::new(0.0, -49.75, -49.5);
pub(crate) const CCTV_CAMERA_POS: Vec3 = Vec3::new(-1.85, -47.9, -48.15);

/// Camera zoomed to laptop. desk_z=1.0, laptop at y≈1.05.
pub(crate) const LAPTOP_VIEW_POS: Vec3 = Vec3::new(0.0, 1.35, 0.5);
pub(crate) const LAPTOP_VIEW_TARGET: Vec3 = Vec3::new(0.0, 1.10, 1.12);
pub(crate) const CAMERA_LERP_SPEED: f32 = 6.0;

#[derive(Resource, Clone)]
pub enum CameraMode {
    Free,
    ZoomingToLaptop {
        saved_transform: Transform,
    },
    AtLaptop {
        saved_transform: Transform,
    },
    Returning(Transform),
    /// Smoothly turn (rotation only) to face a world-space point.
    /// Used while a visitor is inside the bunker. The position is
    /// untouched — the player stays where they were standing.
    LookingAt {
        target: Vec3,
        saved_transform: Transform,
    },
    /// Player is studying the CCTV feed in fullscreen. The CCTV
    /// camera takes over the window and the FPS camera goes
    /// inactive until the player presses E or Esc.
    AtCctv {
        saved_transform: Transform,
    },
}

/// Shared material handles used across every room. Four colors —
/// everything the bunker renders is currently tinted by one of these.
///
/// This is deliberately a small, fixed palette. When the bunker needs
/// real visual variety (stained concrete zones, metal accent walls,
/// weathered vs. clean wood, ...) this is the first place to extend
/// or replace — adding a handful more variants here is cheaper than
/// switching to per-material lookups per call site.
pub(crate) struct Palette {
    pub concrete: Handle<StandardMaterial>,
    pub concrete_dark: Handle<StandardMaterial>,
    pub wood: Handle<StandardMaterial>,
    pub metal: Handle<StandardMaterial>,
}

impl Palette {
    pub(crate) fn new(mats: &mut Assets<StandardMaterial>) -> Self {
        Self {
            concrete: mats.add(StandardMaterial {
                base_color: Color::srgb(0.14, 0.13, 0.12),
                perceptual_roughness: 0.95,
                ..default()
            }),
            concrete_dark: mats.add(StandardMaterial {
                base_color: Color::srgb(0.10, 0.10, 0.09),
                perceptual_roughness: 0.95,
                ..default()
            }),
            wood: mats.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.16, 0.10),
                perceptual_roughness: 0.85,
                ..default()
            }),
            metal: mats.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.28, 0.26),
                perceptual_roughness: 0.5,
                metallic: 0.6,
                ..default()
            }),
        }
    }
}

/// Bundle of references every per-room spawner needs. Collapses what
/// used to be 6 positional arguments into `ctx: &mut RoomCtx`, which
/// makes adding a new room mechanical and keeps call sites uniform.
pub(crate) struct RoomCtx<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub asset_server: &'a AssetServer,
    pub meshes: &'a mut Assets<Mesh>,
    pub mats: &'a mut Assets<StandardMaterial>,
    pub pal: &'a Palette,
    pub l: &'a Layout,
}

/// Bunker dimensions. Only stores the primary constants; derived
/// values are computed via methods so nothing can go stale.
pub(crate) struct Layout {
    /// Ceiling height.
    pub h: f32,
    /// Main corridor half-width (x extent from center).
    pub hw: f32,
    /// Z of the front wall (stairs / entrance).
    pub front_z: f32,
    /// Z of the trade grate.
    pub trade_z: f32,
    /// Z of the office ↔ armory divider grate.
    pub divider_z: f32,
    /// Half-width of the grate opening.
    pub hole_half: f32,
    /// Z of the north edge of the T-junction.
    pub tj_north: f32,
    /// Z of the back wall (south edge of corridor + side rooms).
    pub back_z: f32,
    /// How far each side room extends from the corridor wall.
    pub side_depth: f32,
    /// Width of the side-room doorframe openings.
    pub side_door_width: f32,
}

impl Layout {
    pub(crate) fn new() -> Self {
        Self {
            h: 2.4,
            hw: 2.05,
            front_z: 5.0,
            trade_z: 1.5,
            divider_z: -2.25,
            hole_half: 0.6,
            tj_north: -4.63,
            back_z: -7.63,
            side_depth: 3.0,
            side_door_width: 1.6,
        }
    }

    pub fn hh(&self) -> f32 {
        self.h / 2.0
    }

    /// Height of a walkable doorframe opening. Leaves a 0.3 m air gap
    /// above for the lintel so the frame reads as a real doorway
    /// rather than a ceiling-height hole.
    pub fn opening_h(&self) -> f32 {
        self.h - 0.3
    }

    pub fn desk_z(&self) -> f32 {
        self.trade_z - 0.5
    }

    pub fn tj_center(&self) -> f32 {
        (self.tj_north + self.back_z) / 2.0
    }

    pub fn tj_len(&self) -> f32 {
        self.tj_north - self.back_z
    }

    /// Kitchen (left): furthest x.
    pub fn kitchen_x_min(&self) -> f32 {
        -(self.hw + self.side_depth)
    }

    /// Kitchen (left): center x.
    pub fn kitchen_x_center(&self) -> f32 {
        (self.kitchen_x_min() + (-self.hw)) / 2.0
    }

    /// Quarters (right): furthest x.
    pub fn quarters_x_max(&self) -> f32 {
        self.hw + self.side_depth
    }

    /// Quarters (right): center x.
    pub fn quarters_x_center(&self) -> f32 {
        (self.hw + self.quarters_x_max()) / 2.0
    }
}
