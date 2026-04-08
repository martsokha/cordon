//! Fog-of-war shader material.
//!
//! A single fullscreen quad over the map renders the fog using
//! `FogMaterial`. Each frame the fog system in `laptop::fog`
//! rebuilds the material's reveal-circle array and updates the
//! persistent scout-mask texture, then the fragment shader
//! paints:
//!
//! - **Currently visible** pixels (inside any reveal circle) →
//!   transparent, the underlying terrain shows through.
//! - **Previously scouted** pixels (scout-mask texel != 0) →
//!   grey memory wash over the terrain. The mask is a 256×256
//!   texture covering the whole map, updated in place as squads
//!   move, so memorised ground is capped only by texture
//!   resolution and grows monotonically forever.
//! - **Never seen** pixels → swirly animated dark cloud.

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

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FogMaterial {
    /// `.x` = number of active reveal circles in `reveals`,
    /// `.y` / `.z` / `.w` = unused padding.
    #[uniform(0)]
    pub counts: Vec4,
    /// Currently-active reveal circles. Each element is
    /// `(centre.x, centre.y, radius, _)` in world space.
    #[uniform(1)]
    pub reveals: [Vec4; MAX_REVEAL_CIRCLES],
    /// Persistent scout mask. `R8Unorm` texture where each texel
    /// is 0 (unscouted) or 1 (scouted), bilinear-filtered for
    /// smooth boundaries. Updated in place by
    /// [`crate::laptop::fog::mask::update_scout_mask`].
    #[texture(2)]
    #[sampler(3)]
    pub scout_mask: Handle<Image>,
}

impl FogMaterial {
    /// Build a fog material with a specific scout mask handle.
    /// The mask is created once by
    /// [`crate::laptop::fog::mask::init_scout_mask`] and reused
    /// as the shader input.
    pub fn new(scout_mask: Handle<Image>) -> Self {
        Self {
            counts: Vec4::ZERO,
            reveals: [Vec4::ZERO; MAX_REVEAL_CIRCLES],
            scout_mask,
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
/// looks up this entity to update the material's reveal array.
#[derive(Component)]
pub struct FogOverlay;

pub struct FogShaderPlugin;

impl Plugin for FogShaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<FogMaterial>::default());
    }
}

/// Spawn the fullscreen fog overlay. Mounted just below the
/// cloud layer (z = 4.5) so clouds still drift on top of the
/// fog, and above the terrain (z = 0..3) so it covers all the
/// map content.
///
/// Visibility is managed by
/// `crate::laptop::fog::sync::sync_fog_material`, which ties it
/// to the laptop+map tab state.
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    fog_mats: &mut ResMut<Assets<FogMaterial>>,
    scout_mask: Handle<Image>,
) {
    commands.spawn((
        FogOverlay,
        Mesh2d(meshes.add(Rectangle::new(5000.0, 5000.0))),
        MeshMaterial2d(fog_mats.add(FogMaterial::new(scout_mask))),
        Transform::from_xyz(0.0, 0.0, 4.5),
    ));
}
