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

pub(super) type SpatialGrid = HashMap<(i32, i32), Vec<usize>>;

pub(super) fn build_spatial_grid(snapshot: &[NpcSnap], cell_size: f32) -> SpatialGrid {
    let mut grid: SpatialGrid = HashMap::with_capacity(snapshot.len() / 4 + 1);
    for (i, snap) in snapshot.iter().enumerate() {
        let cell = (
            (snap.pos.x / cell_size).floor() as i32,
            (snap.pos.y / cell_size).floor() as i32,
        );
        grid.entry(cell).or_default().push(i);
    }
    grid
}

pub(super) fn collect_nearby_cells(
    center: Vec2,
    radius: f32,
    grid: &SpatialGrid,
    cell_size: f32,
    out: &mut Vec<usize>,
) {
    let min_cx = ((center.x - radius) / cell_size).floor() as i32;
    let max_cx = ((center.x + radius) / cell_size).floor() as i32;
    let min_cy = ((center.y - radius) / cell_size).floor() as i32;
    let max_cy = ((center.y + radius) / cell_size).floor() as i32;
    for cy in min_cy..=max_cy {
        for cx in min_cx..=max_cx {
            if let Some(bucket) = grid.get(&(cx, cy)) {
                out.extend_from_slice(bucket);
            }
        }
    }
}
