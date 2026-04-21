//! Command post zone: desk, laptop, chair, bookshelves, props.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use cordon_core::entity::bunker::UpgradeEffect;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::resources::PlayerUpgrades;

use crate::bunker::geometry::*;
use crate::bunker::interaction::Interactable;
use crate::bunker::resources::{Layout, RadioPlacement, RoomCtx};
use crate::bunker::visitor::DoorButton;

/// Marker for the `Radio04` prop that the `listening_device`
/// upgrade unlocks. Separate marker from other radios so
/// [`sync_command_listening_device`] can tell whether the prop is
/// already present without touching the other radio or the desk.
#[derive(Component, Debug, Clone, Copy)]
pub struct ListeningRadio;

/// World-space placement for the listening-device radio. Kept as
/// a module constant so both the initial spawn and the sync
/// system produce the same pose.
fn listening_radio_transform(l: &Layout) -> Transform {
    Transform::from_translation(Vec3::new(0.55, 0.0, l.desk_z() - 0.2))
        .with_rotation(Quat::from_rotation_y(FRAC_PI_2))
}

fn has_listening_device(upgrades: &PlayerUpgrades, data: &GameDataResource) -> bool {
    upgrades
        .installed_effects(&data.0.upgrades)
        .any(|e| matches!(e, UpgradeEffect::ListeningDevice))
}

pub fn spawn(ctx: &mut RoomCtx<'_, '_, '_>) {
    // Divider grate.
    let metal = ctx.pal.metal.clone();
    ctx.grate_bars(
        -ctx.l.hw,
        -ctx.l.hole_half,
        ctx.l.divider_z,
        ctx.l.h,
        0.12,
        &metal,
    );
    ctx.grate_bars(
        ctx.l.hole_half,
        ctx.l.hw,
        ctx.l.divider_z,
        ctx.l.h,
        0.12,
        &metal,
    );

    // Wire grid hung on the east grate panel, centred on the
    // solid panel (between the opening and the side wall) and
    // raised so the grid's base sits well off the floor.
    let grate_panel_center_x = (ctx.l.hole_half + ctx.l.hw) / 2.0;
    ctx.prop_rot(
        Prop::Shelf01Grid,
        Vec3::new(grate_panel_center_x, 0.3, ctx.l.divider_z + 0.02),
        Quat::from_rotation_y(-FRAC_PI_2),
    );

    // Dinner table as the command desk. Table top at y = 1.037.
    const TABLE_TOP: f32 = 1.037;
    ctx.prop(Prop::WoodenDinnerTable, Vec3::new(0.0, 0.0, ctx.l.desk_z()));
    ctx.prop(Prop::WoodenChair, Vec3::new(0.0, 0.0, ctx.l.desk_z() - 0.5));
    ctx.prop(Prop::Mug, Vec3::new(-0.35, TABLE_TOP, ctx.l.desk_z() - 0.1));
    // Counter radio: the radio module owns the spawn (like
    // laptop). Pulled ~18 cm in from the +x (left) edge so the
    // body sits solidly on the desk instead of hanging off it.
    ctx.commands.insert_resource(RadioPlacement {
        pos: Vec3::new(0.75, TABLE_TOP, ctx.l.desk_z()),
        rot: Quat::from_rotation_y(PI),
    });

    // Door button — sits on the table surface. A flat cylinder
    // so it reads as a push button at a glance rather than a
    // mystery ball. Starts unlit; the visitor module's
    // `update_button_glow` flips `emissive` on this material when
    // someone is knocking.
    const BUTTON_RADIUS: f32 = 0.035;
    const BUTTON_HEIGHT: f32 = 0.015;
    let button_mesh = ctx.meshes.add(Cylinder::new(BUTTON_RADIUS, BUTTON_HEIGHT));
    let button_mat = ctx.mats.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.05, 0.05),
        perceptual_roughness: 0.4,
        metallic: 0.2,
        emissive: LinearRgba::BLACK,
        ..default()
    });
    ctx.commands.spawn((
        DoorButton,
        Interactable {
            key: "interact-door".into(),
            enabled: false,
        },
        Mesh3d(button_mesh),
        MeshMaterial3d(button_mat),
        // Centre sits half the cylinder height above the desk
        // surface so it rests flush instead of hovering.
        Transform::from_xyz(
            0.375,
            TABLE_TOP + BUTTON_HEIGHT / 2.0,
            ctx.l.desk_z() - 0.25,
        ),
    ));
    // Bin between the table legs.
    ctx.prop_scaled(
        Prop::Bin,
        Vec3::new(-0.4, 0.0, ctx.l.desk_z()),
        Quat::IDENTITY,
        0.6,
    );
    // Chunky tube-style radio on the floor beside the chair,
    // same side as the counter radio (+x) so the two read as a
    // matched pair. Tucked well under the table. Only present
    // if the `listening_device` upgrade is installed — it's the
    // visual half of the decryption feature (see
    // `UpgradeEffect::ListeningDevice`). `sync_command_listening_device`
    // keeps this in sync if the upgrade is installed mid-run.
    if has_listening_device(ctx.upgrades, ctx.game_data) {
        let tf = listening_radio_transform(ctx.l);
        ctx.commands.spawn((
            ListeningRadio,
            PropPlacement::new(Prop::Radio04, tf.translation).rotated(tf.rotation),
            DespawnOnExit(crate::AppState::Playing),
        ));
    }

    // East wall: two full-height bookshelves along the
    // command-post z-span.
    let shelf_north_z = ctx.l.trade_z - 0.978;
    let shelf_south_z = ctx.l.divider_z + 0.978;
    for z in [shelf_north_z, shelf_south_z] {
        ctx.prop_rot(
            Prop::Bookshelf,
            Vec3::new(ctx.l.hw - 0.25, 0.0, z),
            Quat::from_rotation_y(-FRAC_PI_2),
        );
    }

    // West wall: a row of lockers running south from just under
    // the CCTV toward the divider grate, mirroring the entry
    // room's locker bank. Fronts face into the room. Starts
    // 30 cm south of the trade grate to leave breathing room
    // around the grate frame.
    for i in 0..5 {
        ctx.prop_rot(
            Prop::Locker,
            Vec3::new(-ctx.l.hw + 0.3, 0.0, ctx.l.trade_z - 0.3 - 0.5 * i as f32),
            Quat::from_rotation_y(FRAC_PI_2),
        );
    }
    // Rug in front of the desk.
    ctx.prop(Prop::Rug, Vec3::new(0.0, 0.0, ctx.l.desk_z() - 0.3));
}

/// Reactive spawn: if the player installs the `listening_device`
/// upgrade after the bunker was first built, add the `Radio04`
/// prop without a full bunker rebuild. Mirrors
/// [`sync_hall_racks`](super::hall::sync_hall_racks).
///
/// We don't despawn on uninstall — the game doesn't expose
/// uninstall, and the `DespawnOnExit(AppState::Playing)` on the
/// spawn handles the run-reset path.
pub fn sync_command_listening_device(
    mut commands: Commands,
    upgrades: Res<PlayerUpgrades>,
    game_data: Res<GameDataResource>,
    existing: Query<(), With<ListeningRadio>>,
) {
    if !upgrades.is_changed() {
        return;
    }
    if !existing.is_empty() {
        return;
    }
    if !has_listening_device(&upgrades, &game_data) {
        return;
    }
    let tf = listening_radio_transform(&Layout::new());
    commands.spawn((
        ListeningRadio,
        PropPlacement::new(Prop::Radio04, tf.translation).rotated(tf.rotation),
        DespawnOnExit(crate::AppState::Playing),
    ));
}
