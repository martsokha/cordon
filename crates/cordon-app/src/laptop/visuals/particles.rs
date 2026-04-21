//! GPU particle effects for combat.
//!
//! Two effects sit permanently in the scene and get repositioned +
//! [`reset()`](bevy_hanabi::EffectSpawner::reset) on every
//! [`ShotFired`]:
//!
//! - **Muzzle flash** at `ev.from` — a tight, short-lived burst of
//!   warm sparks pointing along the shot axis.
//! - **Impact spark** at `ev.to` — a wider, slightly longer burst
//!   splashing back along the shot axis.
//!
//! Shot direction is passed through [`EffectProperties`] as
//! `forward` (unit vector) and `tangent` (90°-rotated unit vector
//! in the XY plane). Velocity is built directly from those
//! properties in world space, so the emitter's own rotation does
//! not matter — the Z-rotation trick didn't work because Hanabi
//! expression literals like `Vec3::X` are evaluated in world space,
//! not emitter-local.

use bevy::prelude::*;
use bevy_hanabi::Gradient as HanabiGradient;
use bevy_hanabi::prelude::*;
use cordon_sim::plugin::SimSet;
use cordon_sim::plugin::prelude::{NpcDied, ShotFired};

use crate::PlayingState;

/// Z-level of combat VFX — sits above NPC dots (Z=10) so the
/// sparks read over the shooters and hit targets rather than
/// getting covered by their faction circles.
const VFX_Z: f32 = 11.0;

/// Property name carrying the shot's forward direction (unit vec,
/// Z=0). Same name on both effects so the fire system can set it
/// uniformly.
const PROP_FORWARD: &str = "forward";

/// Property name carrying the in-plane tangent (forward rotated 90°
/// CCW, Z=0). Used to spread particles perpendicular to the shot
/// axis.
const PROP_TANGENT: &str = "tangent";

#[derive(Component)]
struct MuzzleFlashEffect;

#[derive(Component)]
struct ImpactSparkEffect;

#[derive(Component)]
struct DeathBurstEffect;

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_effects);
        app.add_systems(
            Update,
            (
                fire_combat_vfx.after(SimSet::Combat),
                fire_death_vfx.after(SimSet::Death),
            )
                .run_if(in_state(PlayingState::Laptop)),
        );
    }
}

fn setup_effects(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    let muzzle = effects.add(build_muzzle_flash());
    commands.spawn((
        Name::new("vfx_muzzle_flash"),
        MuzzleFlashEffect,
        ParticleEffect::new(muzzle),
        EffectProperties::default(),
        Transform::from_xyz(0.0, 0.0, VFX_Z),
    ));

    let impact = effects.add(build_impact_spark());
    commands.spawn((
        Name::new("vfx_impact_spark"),
        ImpactSparkEffect,
        ParticleEffect::new(impact),
        EffectProperties::default(),
        Transform::from_xyz(0.0, 0.0, VFX_Z),
    ));

    let death = effects.add(build_death_burst());
    commands.spawn((
        Name::new("vfx_death_burst"),
        DeathBurstEffect,
        ParticleEffect::new(death),
        Transform::from_xyz(0.0, 0.0, VFX_Z),
    ));
}

/// Big radial blood-burst at the corpse: ~30 particles in every
/// direction in the XY plane, slower and longer-lived than impact
/// sparks so the kill reads as a punctuation mark on the map.
fn build_death_burst() -> EffectAsset {
    let spawner = SpawnerSettings::once(30.0.into()).with_emit_on_start(false);
    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(0.55).expr());

    // Spawn on a small circle around the death point so the burst
    // starts with a visible footprint rather than a single pixel.
    let init_pos = SetPositionCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Z).expr(),
        radius: writer.lit(1.5).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Radial outward velocity in the XY plane, 15–45 map units/s.
    let init_vel = SetVelocityCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Z).expr(),
        speed: (writer.lit(15.0) + writer.rand(ScalarType::Float) * writer.lit(30.0)).expr(),
    };

    // Linear drag so particles ease to a stop instead of flying off
    // the map — reads as "splatter, settle" rather than "spray".
    let update_drag = LinearDragModifier::new(writer.lit(4.0).expr());

    let mut color_grad = HanabiGradient::new();
    color_grad.add_key(0.0, Vec4::new(0.45, 0.45, 0.45, 1.0));
    color_grad.add_key(0.35, Vec4::new(0.28, 0.28, 0.28, 0.95));
    color_grad.add_key(1.0, Vec4::new(0.08, 0.08, 0.08, 0.0));

    let mut size_grad = HanabiGradient::new();
    size_grad.add_key(0.0, Vec3::splat(3.5));
    size_grad.add_key(1.0, Vec3::splat(0.6));

    EffectAsset::new(512, spawner, writer.finish())
        .with_name("death_burst")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .update(update_drag)
        .render(ColorOverLifetimeModifier {
            gradient: color_grad,
            ..default()
        })
        .render(SizeOverLifetimeModifier {
            gradient: size_grad,
            screen_space_size: true,
        })
}

/// Tight warm-yellow burst pointing along `forward`.
fn build_muzzle_flash() -> EffectAsset {
    let spawner = SpawnerSettings::once(14.0.into()).with_emit_on_start(false);
    let writer = ExprWriter::new();

    let forward_prop = writer.add_property(PROP_FORWARD, Vec3::X.into());
    let tangent_prop = writer.add_property(PROP_TANGENT, Vec3::Y.into());
    let forward = writer.prop(forward_prop);
    let tangent = writer.prop(tangent_prop);

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(0.09).expr());
    let init_pos = SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());

    // Velocity: forward-biased with a little perpendicular spread.
    //   dir   = forward + tangent * spread * 0.35
    //   speed = 40..80
    //   v     = normalize(dir) * speed
    let spread = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let speed = writer.lit(40.0) + writer.rand(ScalarType::Float) * writer.lit(40.0);
    let dir = forward + tangent * spread * writer.lit(0.35);
    let velocity = dir.normalized() * speed;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    let mut color_grad = HanabiGradient::new();
    color_grad.add_key(0.0, Vec4::new(1.0, 0.92, 0.55, 1.0));
    color_grad.add_key(1.0, Vec4::new(1.0, 0.55, 0.15, 0.0));

    let mut size_grad = HanabiGradient::new();
    size_grad.add_key(0.0, Vec3::splat(3.0));
    size_grad.add_key(1.0, Vec3::splat(0.5));

    EffectAsset::new(256, spawner, writer.finish())
        .with_name("muzzle_flash")
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
            screen_space_size: true,
        })
}

/// Wider cool-white burst pointing along `-forward` (back toward
/// the shooter), with a broader perpendicular spread.
fn build_impact_spark() -> EffectAsset {
    let spawner = SpawnerSettings::once(18.0.into()).with_emit_on_start(false);
    let writer = ExprWriter::new();

    let forward_prop = writer.add_property(PROP_FORWARD, Vec3::X.into());
    let tangent_prop = writer.add_property(PROP_TANGENT, Vec3::Y.into());
    let forward = writer.prop(forward_prop);
    let tangent = writer.prop(tangent_prop);

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(0.22).expr());
    let init_pos = SetAttributeModifier::new(Attribute::POSITION, writer.lit(Vec3::ZERO).expr());

    //   dir   = -forward + tangent * spread * 0.8
    //   speed = 25..80
    let spread = writer.rand(ScalarType::Float) * writer.lit(2.0) - writer.lit(1.0);
    let speed = writer.lit(25.0) + writer.rand(ScalarType::Float) * writer.lit(55.0);
    let dir = forward * writer.lit(-1.0) + tangent * spread * writer.lit(0.8);
    let velocity = dir.normalized() * speed;
    let init_vel = SetAttributeModifier::new(Attribute::VELOCITY, velocity.expr());

    let mut color_grad = HanabiGradient::new();
    color_grad.add_key(0.0, Vec4::new(1.0, 0.95, 0.85, 1.0));
    color_grad.add_key(0.3, Vec4::new(1.0, 0.6, 0.25, 0.9));
    color_grad.add_key(1.0, Vec4::new(0.5, 0.15, 0.08, 0.0));

    let mut size_grad = HanabiGradient::new();
    size_grad.add_key(0.0, Vec3::splat(2.5));
    size_grad.add_key(1.0, Vec3::splat(0.4));

    EffectAsset::new(256, spawner, writer.finish())
        .with_name("impact_spark")
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
            screen_space_size: true,
        })
}

/// Drive both emitters from `ShotFired`. For every shot, set the
/// `forward` and `tangent` properties (unit vectors in the map's XY
/// plane), teleport the emitter to the shot endpoint, and reset the
/// spawner for a one-shot burst.
fn fire_combat_vfx(
    mut shots: MessageReader<ShotFired>,
    mut muzzle_q: Query<
        (&mut Transform, &mut EffectProperties, &mut EffectSpawner),
        (With<MuzzleFlashEffect>, Without<ImpactSparkEffect>),
    >,
    mut impact_q: Query<
        (&mut Transform, &mut EffectProperties, &mut EffectSpawner),
        (With<ImpactSparkEffect>, Without<MuzzleFlashEffect>),
    >,
) {
    // EffectSpawner is inserted in PostUpdate on the frame the
    // emitter entity is spawned, so on the very first frame these
    // queries may be empty.
    let Ok((mut muzzle_tx, mut muzzle_props, mut muzzle_spawner)) = muzzle_q.single_mut() else {
        shots.clear();
        return;
    };
    let Ok((mut impact_tx, mut impact_props, mut impact_spawner)) = impact_q.single_mut() else {
        shots.clear();
        return;
    };

    for ev in shots.read() {
        let delta = ev.to - ev.from;
        let len = delta.length();
        if len < 0.5 {
            continue;
        }
        let forward2 = delta / len;
        let forward = Vec3::new(forward2.x, forward2.y, 0.0);
        // 90° CCW in the XY plane: (x, y) -> (-y, x).
        let tangent = Vec3::new(-forward2.y, forward2.x, 0.0);

        muzzle_tx.translation = Vec3::new(ev.from.x, ev.from.y, VFX_Z);
        muzzle_props.set(PROP_FORWARD, forward.into());
        muzzle_props.set(PROP_TANGENT, tangent.into());
        muzzle_spawner.reset();

        impact_tx.translation = Vec3::new(ev.to.x, ev.to.y, VFX_Z);
        impact_props.set(PROP_FORWARD, forward.into());
        impact_props.set(PROP_TANGENT, tangent.into());
        impact_spawner.reset();
    }
}

/// Read `NpcDied` and fire the radial blood burst at the corpse's
/// position. Only the last death this frame gets a visible burst —
/// same coalescing caveat as the combat path. Rare in practice
/// given kill cadence.
fn fire_death_vfx(
    mut deaths: MessageReader<NpcDied>,
    npc_transforms: Query<&Transform, Without<DeathBurstEffect>>,
    mut burst_q: Query<(&mut Transform, &mut EffectSpawner), With<DeathBurstEffect>>,
) {
    let Ok((mut burst_tx, mut burst_spawner)) = burst_q.single_mut() else {
        deaths.clear();
        return;
    };
    for ev in deaths.read() {
        let Ok(corpse_tx) = npc_transforms.get(ev.entity) else {
            // Entity already despawned this frame. Rare (death and
            // despawn are separate systems) but don't crash on it.
            continue;
        };
        burst_tx.translation = Vec3::new(corpse_tx.translation.x, corpse_tx.translation.y, VFX_Z);
        burst_spawner.reset();
    }
}
