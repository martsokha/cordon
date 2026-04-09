//! Per-NPC snapshot + uniform grid used by the engagement scanner.
//!
//! The engagement system snapshots every alive NPC into a flat
//! [`NpcSnap`] array, then bins those indices into a coarse 2D
//! [`SpatialGrid`] keyed on `(cell_x, cell_y)`. Squad-vision scans
//! enumerate only the cells overlapping their members' vision radii
//! instead of the full population.

use std::collections::HashMap;

use bevy::prelude::*;

pub(super) struct NpcSnap {
    pub entity: Entity,
    pub squad: Entity,
    pub pos: Vec2,
    pub vision: f32,
}

/// Uniform-grid spatial index. Stored in a `Local` on the engagement
/// system so we reuse the hashmap capacity and per-bucket vecs across
/// ticks instead of reallocating every frame.
#[derive(Default)]
pub(super) struct SpatialGrid {
    cells: HashMap<(i32, i32), Vec<usize>>,
}

impl SpatialGrid {
    /// Rebuild the grid from a fresh snapshot. Preserves the hashmap
    /// capacity and per-bucket vec capacity by clearing instead of
    /// reallocating.
    pub fn rebuild(&mut self, snapshot: &[NpcSnap], cell_size: f32) {
        for bucket in self.cells.values_mut() {
            bucket.clear();
        }
        for (i, snap) in snapshot.iter().enumerate() {
            let key = (
                (snap.pos.x / cell_size).floor() as i32,
                (snap.pos.y / cell_size).floor() as i32,
            );
            self.cells.entry(key).or_default().push(i);
        }
    }

    /// Push every snapshot index whose cell overlaps the `radius`
    /// disk around `center` into `out`. Cells are *coarse* — callers
    /// still have to filter by actual distance afterwards.
    pub fn collect_nearby(&self, center: Vec2, radius: f32, cell_size: f32, out: &mut Vec<usize>) {
        let min_cx = ((center.x - radius) / cell_size).floor() as i32;
        let max_cx = ((center.x + radius) / cell_size).floor() as i32;
        let min_cy = ((center.y - radius) / cell_size).floor() as i32;
        let max_cy = ((center.y + radius) / cell_size).floor() as i32;
        for cy in min_cy..=max_cy {
            for cx in min_cx..=max_cx {
                if let Some(bucket) = self.cells.get(&(cx, cy)) {
                    out.extend_from_slice(bucket);
                }
            }
        }
    }
}
