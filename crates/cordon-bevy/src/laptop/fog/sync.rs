//! Fog shader uniform sync.
//!
//! Packs the reveal + discovered caches built by `apply` and
//! `trail` into the `FogMaterial` uniform arrays each frame so
//! the WGSL shader can render the correct cut-throughs.

use bevy::prelude::*;

use super::{DiscoveredDisks, FogEnabled, FogReveals, MemoryTrail};
use crate::PlayingState;
use crate::laptop::environment::fog::{
    FogMaterial, FogOverlay, MAX_DISCOVERED_AREAS, MAX_REVEAL_CIRCLES,
};
use crate::laptop::ui::LaptopTab;

/// Push the cached reveal/discovered arrays into the fog overlay
/// material so the WGSL shader can render the right cut-throughs
/// each frame. Runs after `apply_fog` in the same chain so it
/// always sees the freshest data.
///
/// When fog is disabled (cheat mode), we publish a synthetic
/// reveal so the shader's "currently visible" mask covers the
/// whole map and effectively disables the overlay.
pub(super) fn sync_fog_material(
    fog_reveals: Res<FogReveals>,
    discovered_disks: Res<DiscoveredDisks>,
    memory_trail: Res<MemoryTrail>,
    fog_enabled: Res<FogEnabled>,
    state: Res<State<PlayingState>>,
    active_tab: Res<LaptopTab>,
    mut overlay_q: Query<(&MeshMaterial2d<FogMaterial>, &mut Visibility), With<FogOverlay>>,
    mut fog_mats: ResMut<Assets<FogMaterial>>,
) {
    let map_visible = *state.get() == PlayingState::Laptop && *active_tab == LaptopTab::Map;
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
        // Cheat mode: one giant reveal disk that covers the whole
        // map. Cheaper than threading a separate "fog disabled"
        // branch through the shader.
        mat.counts = Vec4::new(1.0, 0.0, 0.0, 0.0);
        mat.reveals[0] = Vec4::new(0.0, 0.0, 1_000_000.0, 0.0);
        return;
    }

    // Pack reveal circles into the fixed-size uniform array.
    // Slots beyond the active count are ignored by the shader
    // (it breaks the loop at `i >= counts.x`), but we still zero
    // them so stale data can't leak through if the count
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

    // Pack discovered area disks first, then breadcrumb trail
    // points, into the same uniform array. The shader doesn't
    // care which is which — both contribute to the memory wash.
    let n_areas = discovered_disks.0.len().min(MAX_DISCOVERED_AREAS);
    let trail_room = MAX_DISCOVERED_AREAS - n_areas;
    let n_trail = memory_trail.points.len().min(trail_room);
    let n_disc = n_areas + n_trail;

    for i in 0..MAX_DISCOVERED_AREAS {
        let value = if i < n_areas {
            let (c, r) = discovered_disks.0[i];
            Vec4::new(c.x, c.y, r, 0.0)
        } else if i < n_disc {
            let trail_idx = i - n_areas;
            // Walk from the *back* of the deque so the most
            // recent breadcrumbs always make the cut when the
            // trail is longer than the remaining slots.
            let from_back = memory_trail.points.len() - 1 - trail_idx;
            let (c, r) = memory_trail.points[from_back];
            Vec4::new(c.x, c.y, r, 0.0)
        } else {
            Vec4::ZERO
        };
        mat.discovered[i] = value;
    }

    mat.counts = Vec4::new(n_reveals as f32, n_disc as f32, 0.0, 0.0);
}
