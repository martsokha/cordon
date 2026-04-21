//! Fog shader uniform sync.
//!
//! Packs the reveal-circle cache built by `apply` into the
//! `FogMaterial` uniform array each frame and toggles the
//! overlay's visibility with the laptop tab state. Scouted-
//! terrain memory is in a separate texture updated by
//! `mask::update_scout_mask`; nothing to sync here for that.

use bevy::prelude::*;

use super::{FogEnabled, FogReveals};
use crate::laptop::environment::fog::{FogMaterial, FogOverlay, MAX_REVEAL_CIRCLES};
use crate::laptop::ui::LaptopTab;

/// Push the cached reveal circles into the fog overlay material
/// so the WGSL shader can render the right cut-throughs. Runs
/// after `apply_fog` in the same chain so it always sees the
/// freshest data.
///
/// When fog is disabled (F3 cheat), we publish a synthetic
/// reveal that covers the whole map, effectively disabling the
/// overlay without a separate shader branch.
pub(super) fn sync_fog_material(
    fog_reveals: Res<FogReveals>,
    fog_enabled: Res<FogEnabled>,
    active_tab: Res<LaptopTab>,
    mut overlay_q: Query<(&MeshMaterial2d<FogMaterial>, &mut Visibility), With<FogOverlay>>,
    mut fog_mats: ResMut<Assets<FogMaterial>>,
) {
    // Fog is gated on the Map tab, not on `PlayingState::Laptop`:
    // the laptop UI camera renders into the desk projection
    // continuously, so fog needs to be visible there too. The
    // map tab is always the default, so this effectively means
    // "always on" until the player opens another tab.
    let map_visible = *active_tab == LaptopTab::Map;
    let Ok((handle, mut overlay_vis)) = overlay_q.single_mut() else {
        return;
    };
    // Change-guarded visibility so the write doesn't dirty the
    // render pipeline every frame while nothing's changed.
    let target_vis = if map_visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if *overlay_vis != target_vis {
        *overlay_vis = target_vis;
    }
    if !map_visible {
        return;
    }
    let Some(mat) = fog_mats.get_mut(&handle.0) else {
        return;
    };

    if !fog_enabled.enabled {
        // Cheat mode: one giant reveal disk that covers the
        // whole map. Cheaper than threading a separate "fog
        // disabled" branch through the shader.
        mat.counts = Vec4::new(1.0, 0.0, 0.0, 0.0);
        mat.reveals[0] = Vec4::new(0.0, 0.0, 1_000_000.0, 0.0);
        return;
    }

    // Pack reveal circles into the fixed-size uniform array.
    // Slots beyond the active count are ignored by the shader
    // (it breaks the loop at `i >= counts.x`), but we still
    // zero them so stale data can't leak through if the count
    // somehow advances unchanged.
    let n_reveals = fog_reveals.0.len().min(MAX_REVEAL_CIRCLES);
    for (i, slot) in mat.reveals.iter_mut().enumerate() {
        *slot = if i < n_reveals {
            let (c, r) = fog_reveals.0[i];
            Vec4::new(c.x, c.y, r, 0.0)
        } else {
            Vec4::ZERO
        };
    }

    mat.counts = Vec4::new(n_reveals as f32, 0.0, 0.0, 0.0);
}
