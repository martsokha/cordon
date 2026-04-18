//! Low-level geometry helpers: prop placement, tiled mesh builders,
//! and structural spawn functions (walls, floors, grates, stairs).

mod meshes;
mod placement;

pub use meshes::*;
pub use placement::*;

pub use super::props::Prop;
use super::resources::RoomCtx;

/// Real-world size of one texture tile on concrete surfaces, in
/// metres. Applied uniformly to walls, floors, ceilings, door-
/// frames and stairs so adjacent surfaces read as one continuous
/// concrete pour.
pub const TILE_SIZE: f32 = 2.5;

impl<'a, 'w, 's> RoomCtx<'a, 'w, 's> {
    pub fn prop<'c>(
        &'c mut self,
        kind: Prop,
        pos: bevy::prelude::Vec3,
    ) -> bevy::ecs::system::EntityCommands<'c> {
        self.commands.spawn(PropPlacement::new(kind, pos))
    }

    pub fn prop_rot<'c>(
        &'c mut self,
        kind: Prop,
        pos: bevy::prelude::Vec3,
        rot: bevy::prelude::Quat,
    ) -> bevy::ecs::system::EntityCommands<'c> {
        self.commands
            .spawn(PropPlacement::new(kind, pos).rotated(rot))
    }

    pub fn prop_scaled<'c>(
        &'c mut self,
        kind: Prop,
        pos: bevy::prelude::Vec3,
        rot: bevy::prelude::Quat,
        scale: f32,
    ) -> bevy::ecs::system::EntityCommands<'c> {
        self.commands
            .spawn(PropPlacement::new(kind, pos).rotated(rot).scaled(scale))
    }

    pub fn prop_placement<'c>(
        &'c mut self,
        placement: PropPlacement,
    ) -> bevy::ecs::system::EntityCommands<'c> {
        self.commands.spawn(placement)
    }

    pub fn wall(
        &mut self,
        center: bevy::prelude::Vec3,
        rot: bevy::prelude::Quat,
        half_size: bevy::prelude::Vec2,
        mat: &bevy::prelude::Handle<bevy::prelude::StandardMaterial>,
    ) {
        spawn_wall(
            self.commands,
            self.meshes,
            mat.clone(),
            center,
            rot,
            half_size,
        );
    }

    pub fn floor_ceiling(
        &mut self,
        center: bevy::prelude::Vec3,
        half_size: bevy::prelude::Vec2,
        ceiling_y: f32,
        mat: &bevy::prelude::Handle<bevy::prelude::StandardMaterial>,
    ) {
        spawn_floor_ceiling(
            self.commands,
            self.meshes,
            mat.clone(),
            center,
            half_size,
            ceiling_y,
        );
    }

    pub fn grate_bars(
        &mut self,
        x_min: f32,
        x_max: f32,
        z: f32,
        height: f32,
        spacing: f32,
        mat: &bevy::prelude::Handle<bevy::prelude::StandardMaterial>,
    ) {
        spawn_grate_bars(
            self.commands,
            self.meshes,
            mat.clone(),
            x_min,
            x_max,
            z,
            height,
            spacing,
        );
    }

    pub fn doorframe_x(
        &mut self,
        x: f32,
        center_z: f32,
        width: f32,
        opening_h: f32,
        mat: &bevy::prelude::Handle<bevy::prelude::StandardMaterial>,
    ) {
        spawn_doorframe_x(
            self.commands,
            self.meshes,
            mat.clone(),
            x,
            center_z,
            width,
            opening_h,
        );
    }

    pub fn stairs(
        &mut self,
        start_z: f32,
        width: f32,
        steps: u32,
        mat: &bevy::prelude::Handle<bevy::prelude::StandardMaterial>,
    ) {
        spawn_stairs(
            self.commands,
            self.meshes,
            mat.clone(),
            start_z,
            width,
            steps,
        );
    }

    pub fn decor_box(
        &mut self,
        pos: bevy::prelude::Vec3,
        size: bevy::prelude::Vec3,
        mat: &bevy::prelude::Handle<bevy::prelude::StandardMaterial>,
    ) {
        spawn_box(
            self.commands,
            self.meshes,
            mat.clone(),
            pos,
            size,
            TILE_SIZE,
        );
    }
}
