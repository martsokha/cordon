//! Bunker ambient particles.
//!
//! Two kinds of emitters live here:
//!
//! 1. **Dust motes** — one per room, covering the room's AABB.
//!    Driven by [`Layout`] because every room is already sized
//!    against it; the plugin spawns these on entering the bunker.
//!
//! 2. **Prop-attached effects** — built through public helpers
//!    ([`attach_kettle_steam`]) that take the prop entity and
//!    parent the emitter to it. The emitter then moves + rotates
//!    with the prop automatically; no duplicated world
//!    coordinates.

use bevy::prelude::*;
use bevy_hanabi::Gradient as HanabiGradient;
use bevy_hanabi::prelude::*;

use super::resources::Layout;
use crate::PlayingState;

#[derive(Component)]
struct DustEmitter;

pub struct BunkerParticlesPlugin;

impl Plugin for BunkerParticlesPlugin {
    fn build(&self, app: &mut App) {
        // Room dust runs on bunker entry. Prop-attached effects
        // live in the room spawn functions alongside the props
        // they decorate, so they don't need a plugin hook here.
        app.add_systems(
            OnEnter(PlayingState::Bunker),
            spawn_dust_emitters.run_if(not(resource_exists::<BunkerDustSpawned>)),
        );
    }
}

/// Flag set after first dust spawn so re-entering the bunker
/// doesn't duplicate the emitters.
#[derive(Resource)]
struct BunkerDustSpawned;

/// Warm off-white — main corridor, kitchen, quarters.
const DUST_WARM: Vec4 = Vec4::new(0.95, 0.9, 0.8, 0.6);
/// Cool pale blue — T-junction (command/armory), lit by the CCTV
/// bank so cool tones read better there.
const DUST_COOL: Vec4 = Vec4::new(0.82, 0.88, 0.95, 0.55);

fn spawn_dust_emitters(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    let l = Layout::new();

    // Main corridor: front_z → tj_north, full corridor width.
    let corridor_min_z = l.tj_north + 0.2;
    let corridor_max_z = l.front_z - 0.3;
    spawn_dust(
        &mut commands,
        &mut effects,
        "bunker_dust_corridor",
        Vec3::new(0.0, l.hh(), (corridor_min_z + corridor_max_z) * 0.5),
        Vec3::new(
            l.hw - 0.2,
            l.hh() - 0.2,
            (corridor_max_z - corridor_min_z) * 0.5,
        ),
        25.0,
        DUST_WARM,
    );

    // Kitchen: left of -hw, from back_z forward by side_depth.
    let side_min_z = l.back_z + 0.3;
    let side_max_z = l.back_z + l.side_depth - 0.3;
    spawn_dust(
        &mut commands,
        &mut effects,
        "bunker_dust_kitchen",
        Vec3::new(
            l.kitchen_x_center(),
            l.hh(),
            (side_min_z + side_max_z) * 0.5,
        ),
        Vec3::new(
            (l.side_depth * 0.5) - 0.2,
            l.hh() - 0.2,
            (side_max_z - side_min_z) * 0.5,
        ),
        15.0,
        DUST_WARM,
    );

    // Quarters: right of +hw, same Z span as kitchen.
    spawn_dust(
        &mut commands,
        &mut effects,
        "bunker_dust_quarters",
        Vec3::new(
            l.quarters_x_center(),
            l.hh(),
            (side_min_z + side_max_z) * 0.5,
        ),
        Vec3::new(
            (l.side_depth * 0.5) - 0.2,
            l.hh() - 0.2,
            (side_max_z - side_min_z) * 0.5,
        ),
        15.0,
        DUST_WARM,
    );

    // T-junction (command/armory wing). Cool tint.
    spawn_dust(
        &mut commands,
        &mut effects,
        "bunker_dust_tjunction",
        Vec3::new(0.0, l.hh(), l.tj_center()),
        Vec3::new(l.hw - 0.2, l.hh() - 0.2, (l.tj_len() * 0.5) - 0.2),
        20.0,
        DUST_COOL,
    );

    commands.insert_resource(BunkerDustSpawned);
}

fn spawn_dust(
    commands: &mut Commands,
    effects: &mut Assets<EffectAsset>,
    name: &'static str,
    center: Vec3,
    half_extents: Vec3,
    rate: f32,
    color: Vec4,
) {
    let effect = effects.add(build_dust_effect(half_extents, rate, color));
    commands.spawn((
        Name::new(name),
        DustEmitter,
        ParticleEffect::new(effect),
        Transform::from_translation(center),
    ));
}

/// Continuous low-rate dust emitter filling an AABB in world
/// space. Each mote is a ~5 mm camera-facing quad — small enough
/// to read as a speck, not a sheet of paper.
fn build_dust_effect(half_extents: Vec3, rate: f32, color: Vec4) -> EffectAsset {
    let spawner = SpawnerSettings::rate(rate.into());
    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        (writer.lit(6.0) + writer.rand(ScalarType::Float) * writer.lit(4.0)).expr(),
    );

    //   pos = (rand3 * 2 - 1) * half_extents
    let rand_vec = writer.rand(VectorType::VEC3F);
    let pos = (rand_vec * writer.lit(2.0) - writer.lit(Vec3::ONE)) * writer.lit(half_extents);
    let init_pos = SetAttributeModifier::new(Attribute::POSITION, pos.expr());

    // Very slow downward drift with tiny lateral jitter.
    let jitter_x = writer.rand(ScalarType::Float) * writer.lit(0.04) - writer.lit(0.02);
    let jitter_z = writer.rand(ScalarType::Float) * writer.lit(0.04) - writer.lit(0.02);
    let fall = writer.lit(-0.02) - writer.rand(ScalarType::Float) * writer.lit(0.02);
    let velocity = writer.lit(Vec3::X) * jitter_x
        + writer.lit(Vec3::Y) * fall
        + writer.lit(Vec3::Z) * jitter_z;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    let mut color_grad = HanabiGradient::new();
    color_grad.add_key(0.0, Vec4::new(color.x, color.y, color.z, 0.0));
    color_grad.add_key(0.15, color);
    color_grad.add_key(0.85, color);
    color_grad.add_key(1.0, Vec4::new(color.x, color.y, color.z, 0.0));

    // World-space size in meters (not screen-space — that made
    // particles look like giant sheets of paper when near the
    // camera in 3D). 5 mm reads as a point speck at any distance.
    let mut size_grad = HanabiGradient::new();
    size_grad.add_key(0.0, Vec3::splat(0.005));
    size_grad.add_key(1.0, Vec3::splat(0.005));

    EffectAsset::new(512, spawner, writer.finish())
        .with_name("bunker_dust")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .render(ColorOverLifetimeModifier {
            gradient: color_grad,
            ..default()
        })
        .render(SizeOverLifetimeModifier {
            gradient: size_grad,
            screen_space_size: false,
        })
}

/// Attach a rising steam plume as a child of `kettle`. The emitter
/// sits at `local_spout_offset` in the kettle's local frame, so it
/// follows the kettle if it ever moves (and keeps the math in one
/// place — callers don't need to recompute the world position).
pub fn attach_kettle_steam(
    commands: &mut Commands,
    effects: &mut Assets<EffectAsset>,
    kettle: Entity,
    local_spout_offset: Vec3,
) {
    let effect = effects.add(build_kettle_steam_effect());
    let child = commands
        .spawn((
            Name::new("bunker_kettle_steam"),
            ParticleEffect::new(effect),
            Transform::from_translation(local_spout_offset),
        ))
        .id();
    commands.entity(kettle).add_child(child);
}

/// Plume with volume: particles spawn across a small disc (not a
/// single point) so the column has a real cross-section and reads
/// as 3D from any viewpoint. Rises straight up with small lateral
/// jitter — no swirl.
fn build_kettle_steam_effect() -> EffectAsset {
    let spawner = SpawnerSettings::rate(35.0.into());
    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        (writer.lit(1.0) + writer.rand(ScalarType::Float) * writer.lit(0.5)).expr(),
    );

    // 2 cm radius disc around the spout axis, filled (not rim).
    // Gives the plume a round cross-section so the silhouette
    // reads as a 3D column instead of a flat sheet.
    let init_pos = SetPositionCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Y).expr(),
        radius: writer.lit(0.02).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Upward buoyancy + small lateral jitter. No swirl: particles
    // rise mostly vertically and expand via size gradient.
    let jitter_x = writer.rand(ScalarType::Float) * writer.lit(0.06) - writer.lit(0.03);
    let jitter_z = writer.rand(ScalarType::Float) * writer.lit(0.06) - writer.lit(0.03);
    let rise = writer.lit(0.14) + writer.rand(ScalarType::Float) * writer.lit(0.08);
    let velocity = writer.lit(Vec3::X) * jitter_x
        + writer.lit(Vec3::Y) * rise
        + writer.lit(Vec3::Z) * jitter_z;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    let mut color_grad = HanabiGradient::new();
    color_grad.add_key(0.0, Vec4::new(0.95, 0.97, 1.0, 0.0));
    color_grad.add_key(0.15, Vec4::new(0.95, 0.97, 1.0, 0.25));
    color_grad.add_key(0.6, Vec4::new(0.92, 0.94, 0.98, 0.2));
    color_grad.add_key(1.0, Vec4::new(0.9, 0.92, 0.95, 0.0));

    let mut size_grad = HanabiGradient::new();
    size_grad.add_key(0.0, Vec3::splat(0.025));
    size_grad.add_key(1.0, Vec3::splat(0.14));

    EffectAsset::new(512, spawner, writer.finish())
        .with_name("kettle_steam")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .render(ColorOverLifetimeModifier {
            gradient: color_grad,
            ..default()
        })
        .render(SizeOverLifetimeModifier {
            gradient: size_grad,
            screen_space_size: false,
        })
}
