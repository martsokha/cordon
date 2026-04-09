//! Raster-grid scout mask.
//!
//! A 256×256 single-channel texture covering the 5000×5000 map.
//! Each texel is "scouted" (> 0) or "not yet scouted" (0). Once
//! a texel is set it's never cleared — memory is monotonic.
//!
//! Every frame, [`update_scout_mask`] reads the current set of
//! reveal circles (already computed by
//! [`super::apply::apply_fog`]) and stamps every texel the
//! circles overlap into the texture. The fog shader samples the
//! texture at each fragment's world-space position and uses the
//! sampled value to decide whether the pixel is "memorized",
//! cutting the fog overlay back to reveal the terrain.
//!
//! Bilinear filtering on the texture sampler gives a smooth
//! boundary between scouted and unscouted regions at normal
//! zoom. If you zoom in far enough to see individual texels the
//! grid will be visible — at 256×256 each texel is ~20 world
//! units wide.

use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::FogReveals;

/// Width/height of the mask in texels. At 1024, each texel
/// covers `MAP_EXTENT * 2.0 / MASK_SIZE` ≈ 4.9 world units —
/// fine enough that the bilinear-filtered boundary reads as a
/// smooth curve at normal zoom instead of a visible grid.
///
/// Texture footprint: 1024 × 1024 × 1 byte = 1 MB. Trivial.
pub const MASK_SIZE: u32 = 1024;

/// Half-extent of the playable map in world units. The mask
/// covers `[-MAP_EXTENT, MAP_EXTENT]` in both x and y.
const MAP_EXTENT: f32 = 2500.0;

/// Persistent scout mask. Texels start at 0 and get stamped to
/// 255 ("fully scouted") as player squads walk over them.
///
/// The texture handle is stable for the life of the session;
/// only its pixel buffer changes. `dirty` tracks whether any
/// texel flipped this frame so we can skip the GPU re-upload
/// when the squad is standing still and nothing new has been
/// scouted.
#[derive(Resource, Debug, Clone)]
pub struct ScoutMask {
    pub handle: Handle<Image>,
    pub dirty: bool,
}

pub struct ScoutMaskPlugin;

impl Plugin for ScoutMaskPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_scout_mask);
    }
}

fn init_scout_mask(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Single-channel 8-bit texture, zero-initialised. `R8Unorm`
    // returns values in [0.0, 1.0] to the shader — we stamp 255
    // into bytes to mean "fully scouted", and bilinear filtering
    // gives us a smooth 0..1 gradient at texel boundaries.
    let size = Extent3d {
        width: MASK_SIZE,
        height: MASK_SIZE,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0u8],
        TextureFormat::R8Unorm,
        RenderAssetUsages::all(),
    );
    // Linear filtering across texel edges so the scouted →
    // unscouted boundary reads as a soft gradient rather than a
    // hard grid line.
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());

    let handle = images.add(image);
    commands.insert_resource(ScoutMask {
        handle,
        dirty: false,
    });
}

/// Convert a world coordinate to a mask texel index, clamped to
/// the valid range `[0, MASK_SIZE - 1]`. Returns `None` when the
/// coordinate is far enough outside the map that sampling would
/// be meaningless — the squad movement systems clamp NPCs to
/// `MAP_BOUND`, so this is belt-and-suspenders for edge cases.
fn world_to_texel(world: Vec2) -> Option<(u32, u32)> {
    // Normalise to `[0, 1]` first, then scale to the texel grid.
    let u = (world.x + MAP_EXTENT) / (MAP_EXTENT * 2.0);
    let v = (world.y + MAP_EXTENT) / (MAP_EXTENT * 2.0);
    if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
        return None;
    }
    let tx = (u * MASK_SIZE as f32) as u32;
    let ty = (v * MASK_SIZE as f32) as u32;
    Some((tx.min(MASK_SIZE - 1), ty.min(MASK_SIZE - 1)))
}

/// Stamp every reveal circle from [`FogReveals`] into the mask.
/// Only marks texels that aren't already scouted, so the dirty
/// flag only flips when genuinely new ground is covered.
pub(super) fn update_scout_mask(
    fog_reveals: Res<FogReveals>,
    mut mask: ResMut<ScoutMask>,
    mut images: ResMut<Assets<Image>>,
) {
    if fog_reveals.0.is_empty() {
        return;
    }

    // We only want to touch the image asset if something actually
    // changed this frame — otherwise `get_mut` marks it as dirty
    // and Bevy re-uploads the whole texture to the GPU. First
    // compute the candidate texel set without the asset borrow.
    let Some(image) = images.get(&mask.handle) else {
        return;
    };
    let Some(data) = image.data.as_ref() else {
        return;
    };

    // Pre-compute in world-space how many texels each world unit
    // covers, so we can expand each reveal circle into a texel
    // bounding box and iterate only the candidate cells.
    let texels_per_unit = MASK_SIZE as f32 / (MAP_EXTENT * 2.0);

    // Collect the (x, y) coords of texels that need to flip from
    // 0 → 255. Bounded by the reveal set size; for ~15 NPCs with
    // radius ~130 that's at most ~15 × 55 ≈ 800 texels per frame.
    let mut to_flip: Vec<(u32, u32)> = Vec::new();
    for &(centre, radius) in &fog_reveals.0 {
        let r_sq = radius * radius;
        // Bounding box in texel space.
        let min = centre - Vec2::splat(radius);
        let max = centre + Vec2::splat(radius);
        let (min_tx, min_ty) = match world_to_texel(min) {
            Some(t) => t,
            None => continue,
        };
        let (max_tx, max_ty) = match world_to_texel(max) {
            Some(t) => t,
            None => continue,
        };
        for ty in min_ty..=max_ty {
            for tx in min_tx..=max_tx {
                // Texel centre in world space.
                let wx = (tx as f32 + 0.5) / texels_per_unit - MAP_EXTENT;
                let wy = (ty as f32 + 0.5) / texels_per_unit - MAP_EXTENT;
                let dx = wx - centre.x;
                let dy = wy - centre.y;
                if dx * dx + dy * dy > r_sq {
                    continue;
                }
                let idx = (ty * MASK_SIZE + tx) as usize;
                if data[idx] == 0 {
                    to_flip.push((tx, ty));
                }
            }
        }
    }

    if to_flip.is_empty() {
        return;
    }

    // Something genuinely changed — now we can grab the mut
    // borrow, write the texels, and let Bevy re-upload.
    let Some(image) = images.get_mut(&mask.handle) else {
        return;
    };
    let Some(data) = image.data.as_mut() else {
        return;
    };
    for (tx, ty) in to_flip {
        let idx = (ty * MASK_SIZE + tx) as usize;
        data[idx] = 255;
    }
    mask.dirty = true;
}
