//! Fog-of-war shader material.
//!
//! A single fullscreen quad over the map renders the fog using
//! `FogMaterial`. Each frame the fog system in `laptop::fog`
//! rebuilds the material's reveal-circle and discovered-area
//! arrays based on player squad line-of-sight, then the fragment
//! shader paints:
//!
//! - **Currently visible** pixels (inside any reveal circle) →
//!   transparent, the underlying terrain shows through
//! - **Discovered but not currently in sight** pixels (inside any
//!   discovered area disk) → grey wash, "memory mode"
//! - **Never seen** pixels → swirly animated dark cloud
//!
//! Both arrays are fixed-size to keep the bind group simple. The
//! actual count for each is passed via `counts.x/.y` since WGSL
//! `array<vec4, N>` doesn't track length itself.

use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

/// Hard cap on simultaneous reveal circles. Player squads are
/// small (~3 squads × 5 members = 15 NPCs), plus one always-on
/// circle for the bunker, so 32 is comfortable headroom even if
/// squads later grow. WGSL uniform arrays are fixed-size: too
/// small breaks the shader, too large just wastes a few bytes.
pub const MAX_REVEAL_CIRCLES: usize = 32;

/// Hard cap on discovered disks (areas + memory trail combined).
/// The map has ~40 areas + we want ~200 trail breadcrumbs from
/// past squad walks, so 256 lets the player meaningfully chart
/// long routes. Each entry is 16 bytes → 4KB uniform total,
/// well under WGSL limits.
pub const MAX_DISCOVERED_AREAS: usize = 256;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FogMaterial {
    /// `.x` = number of active reveal circles in `reveals`,
    /// `.y` = number of active discovered disks in `discovered`,
    /// `.z` / `.w` = unused padding.
    #[uniform(0)]
    pub counts: Vec4,
    /// Currently-active reveal circles. Each element is
    /// `(centre.x, centre.y, radius, _)` in world space.
    #[uniform(1)]
    pub reveals: [Vec4; MAX_REVEAL_CIRCLES],
    /// Persistently-discovered area disks (memory mode). Same
    /// `(centre.x, centre.y, radius, _)` packing.
    #[uniform(2)]
    pub discovered: [Vec4; MAX_DISCOVERED_AREAS],
}

impl Default for FogMaterial {
    fn default() -> Self {
        Self {
            counts: Vec4::ZERO,
            reveals: [Vec4::ZERO; MAX_REVEAL_CIRCLES],
            discovered: [Vec4::ZERO; MAX_DISCOVERED_AREAS],
        }
    }
}

impl Material2d for FogMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/fog_of_war.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Marker for the fullscreen fog overlay quad. The fog system
/// looks up this entity to update the material's reveal arrays.
#[derive(Component)]
pub struct FogOverlay;

pub struct FogShaderPlugin;

impl Plugin for FogShaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<FogMaterial>::default());
    }
}

/// Spawn the fullscreen fog overlay. Mounted just below the cloud
/// layer (z = 4.5) so clouds still drift on top of the fog, and
/// above the terrain (z = 0..3) so it covers all the map content.
///
/// Visibility is managed by `laptop::fog::sync_fog_material`,
/// which ties it to the laptop+map tab state.
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    fog_mats: &mut ResMut<Assets<FogMaterial>>,
) {
    commands.spawn((
        FogOverlay,
        Mesh2d(meshes.add(Rectangle::new(5000.0, 5000.0))),
        MeshMaterial2d(fog_mats.add(FogMaterial::default())),
        Transform::from_xyz(0.0, 0.0, 4.5),
    ));
}
