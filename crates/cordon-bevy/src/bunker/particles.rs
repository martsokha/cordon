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

use super::input::controller::FootstepScuffed;
use super::resources::Layout;
use crate::PlayingState;

#[derive(Component)]
struct DustEmitter;

/// Marks a transient (one-shot) emitter entity with the real time
/// at which it should be despawned. Prevents per-event emitters
/// (footstep scuffs, visitor swirls) from piling up for the life
/// of the session — one [`despawn_expired_emitters`] run per
/// frame sweeps them once their TTL is up.
#[derive(Component)]
struct EmitterTtl {
    despawn_at: f32,
}

/// Shared [`EffectAsset`] handles for per-event emitters built
/// once at startup. Storing them in a resource means every
/// footstep scuff and visitor swirl reuses the same compiled
/// effect rather than creating a fresh one (and leaking it) per
/// event.
///
/// `pub` because [`attach_visitor_arrival_swirl`] takes this by
/// reference — callers outside this module pick up the resource
/// themselves rather than having us reach into `Assets` twice.
#[derive(Resource, Clone)]
pub struct EventEffectAssets {
    footstep_scuff: Handle<EffectAsset>,
    visitor_swirl: Handle<EffectAsset>,
}

pub struct BunkerParticlesPlugin;

impl Plugin for BunkerParticlesPlugin {
    fn build(&self, app: &mut App) {
        // Room dust runs on bunker entry. Prop-attached effects
        // (kettle steam, visitor swirl) live in their spawn
        // call-sites alongside the prop they decorate.
        app.add_systems(
            OnEnter(PlayingState::Bunker),
            spawn_dust_emitters.run_if(not(resource_exists::<BunkerDustSpawned>)),
        );
        // Shared event effect assets, built once.
        app.add_systems(
            Startup,
            init_event_effect_assets.run_if(not(resource_exists::<EventEffectAssets>)),
        );
        // Footstep scuffs spawn short-lived emitters; a cleanup
        // pass reaps them once their particles have died.
        app.add_systems(
            Update,
            (spawn_footstep_scuffs, despawn_expired_emitters)
                .run_if(in_state(PlayingState::Bunker)),
        );
    }
}

/// Flag set after first dust spawn so re-entering the bunker
/// doesn't duplicate the emitters.
#[derive(Resource)]
struct BunkerDustSpawned;

/// Build the per-event effect assets once and stash their
/// handles. All footstep scuffs + visitor swirls share these.
fn init_event_effect_assets(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    commands.insert_resource(EventEffectAssets {
        footstep_scuff: effects.add(build_footstep_scuff_effect()),
        visitor_swirl: effects.add(build_visitor_swirl_effect()),
    });
}

/// Despawn any emitter whose TTL has elapsed. Real time (not
/// virtual) so cleanup cadence matches particle lifetime, which
/// is also measured in real time by Hanabi.
fn despawn_expired_emitters(
    mut commands: Commands,
    time: Res<Time<Real>>,
    q: Query<(Entity, &EmitterTtl)>,
) {
    let now = time.elapsed_secs();
    for (entity, ttl) in &q {
        if now >= ttl.despawn_at {
            commands.entity(entity).despawn();
        }
    }
}

/// Warm off-white — main corridor, kitchen, quarters.
const DUST_WARM: Vec4 = Vec4::new(0.95, 0.9, 0.8, 0.6);
/// Cool pale blue — T-junction (command/armory), lit by the CCTV
/// bank so cool tones read better there.
const DUST_COOL: Vec4 = Vec4::new(0.82, 0.88, 0.95, 0.55);

fn spawn_dust_emitters(mut commands: Commands, mut effects: ResMut<Assets<EffectAsset>>) {
    let l = Layout::new();

    // Main corridor: one long emitter spanning front_z → back_z.
    // The two T-junction openings are narrow cross-passages, not
    // separate volumes, so the corridor dust covers them.
    let corridor_min_z = l.back_z + 0.3;
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
        // Longer corridor, more particles. Old rate (25) over a
        // ~12 m span -> density ~2/m. Keep the same density here.
        40.0,
        DUST_WARM,
    );

    // T1 side rooms (kitchen + quarters).
    let t1_min_z = l.tj1_south + 0.3;
    let t1_max_z = l.tj1_north - 0.3;
    spawn_dust(
        &mut commands,
        &mut effects,
        "bunker_dust_kitchen",
        Vec3::new(l.kitchen_x_center(), l.hh(), (t1_min_z + t1_max_z) * 0.5),
        Vec3::new(
            (l.side_depth * 0.5) - 0.2,
            l.hh() - 0.2,
            (t1_max_z - t1_min_z) * 0.5,
        ),
        15.0,
        DUST_WARM,
    );
    spawn_dust(
        &mut commands,
        &mut effects,
        "bunker_dust_quarters",
        Vec3::new(l.quarters_x_center(), l.hh(), (t1_min_z + t1_max_z) * 0.5),
        Vec3::new(
            (l.side_depth * 0.5) - 0.2,
            l.hh() - 0.2,
            (t1_max_z - t1_min_z) * 0.5,
        ),
        15.0,
        DUST_WARM,
    );

    // T2 side rooms (infirmary + workshop). Cool tint to reinforce
    // the "clinical / industrial" atmosphere distinct from T1's
    // living quarters vibe.
    let t2_min_z = l.back_z + 0.3;
    let t2_max_z = l.tj2_north - 0.3;
    spawn_dust(
        &mut commands,
        &mut effects,
        "bunker_dust_infirmary",
        Vec3::new(l.infirmary_x_center(), l.hh(), (t2_min_z + t2_max_z) * 0.5),
        Vec3::new(
            (l.side_depth * 0.5) - 0.2,
            l.hh() - 0.2,
            (t2_max_z - t2_min_z) * 0.5,
        ),
        15.0,
        DUST_COOL,
    );
    spawn_dust(
        &mut commands,
        &mut effects,
        "bunker_dust_workshop",
        Vec3::new(l.workshop_x_center(), l.hh(), (t2_min_z + t2_max_z) * 0.5),
        Vec3::new(
            (l.side_depth * 0.5) - 0.2,
            l.hh() - 0.2,
            (t2_max_z - t2_min_z) * 0.5,
        ),
        15.0,
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

/// Particle lifetime upper bound for the scuff effect, in seconds.
/// Matches the max `init_lifetime` in [`build_footstep_scuff_effect`]
/// plus a small safety margin so cleanup never clips a live
/// particle. Keep these two in sync if either changes.
const SCUFF_TTL_SECS: f32 = 0.7;

/// One-shot dust scuff spawned at the player's feet per footstep.
/// Each event spawns a *new* short-lived emitter entity that is
/// marked with [`EmitterTtl`]; [`despawn_expired_emitters`] sweeps
/// it once its particles die, so per-event emitters don't
/// accumulate for the whole session.
///
/// The [`EffectAsset`] itself is shared via [`EventEffectAssets`]
/// — only the emitter entity is per-step, not the compiled effect.
///
/// Per-step emitters (rather than one shared emitter that gets
/// `reset()`) avoid the coalescing problem: two steps in rapid
/// succession would otherwise collapse into a single burst.
fn spawn_footstep_scuffs(
    mut commands: Commands,
    time: Res<Time<Real>>,
    assets: Res<EventEffectAssets>,
    mut steps: MessageReader<FootstepScuffed>,
) {
    let now = time.elapsed_secs();
    for ev in steps.read() {
        commands.spawn((
            Name::new("bunker_footstep_scuff"),
            ParticleEffect::new(assets.footstep_scuff.clone()),
            Transform::from_translation(ev.pos),
            EmitterTtl {
                despawn_at: now + SCUFF_TTL_SECS,
            },
        ));
    }
}

/// Tiny radial puff: ~8 particles splashing outward in the XZ
/// plane, short lifetime, quickly fading gray so the scuff reads
/// as dust kicked up by a boot and not a smoke bomb.
fn build_footstep_scuff_effect() -> EffectAsset {
    let spawner = SpawnerSettings::once(8.0.into());
    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        (writer.lit(0.3) + writer.rand(ScalarType::Float) * writer.lit(0.2)).expr(),
    );

    // Tiny disc on the ground plane so the burst has a visible
    // footprint at birth.
    let init_pos = SetPositionCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Y).expr(),
        radius: writer.lit(0.04).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Outward XZ velocity, 0.3–0.7 m/s radial.
    let init_vel = SetVelocityCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Y).expr(),
        speed: (writer.lit(0.3) + writer.rand(ScalarType::Float) * writer.lit(0.4)).expr(),
    };

    // Drag so particles settle instead of flying off across the
    // floor — "scuff and settle", not "spray". Drag value 5.0 is
    // eyeballed against the 0.3–0.7 m/s initial speed so particles
    // cover ~5–10 cm before stopping; retune this if speed changes.
    let mut module = writer.finish();
    let drag = LinearDragModifier::new(module.lit(5.0));

    // Alpha bumped from 0.45 peak so the scuff reads visibly when
    // the camera looks straight down at it (where billboards
    // stack and per-particle alpha compounds toward zero).
    let mut color_grad = HanabiGradient::new();
    color_grad.add_key(0.0, Vec4::new(0.5, 0.47, 0.42, 0.6));
    color_grad.add_key(0.6, Vec4::new(0.45, 0.42, 0.38, 0.35));
    color_grad.add_key(1.0, Vec4::new(0.4, 0.37, 0.33, 0.0));

    let mut size_grad = HanabiGradient::new();
    size_grad.add_key(0.0, Vec3::splat(0.02));
    size_grad.add_key(1.0, Vec3::splat(0.06));

    EffectAsset::new(32, spawner, module)
        .with_name("footstep_scuff")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .update(drag)
        .render(ColorOverLifetimeModifier {
            gradient: color_grad,
            ..default()
        })
        .render(SizeOverLifetimeModifier {
            gradient: size_grad,
            screen_space_size: false,
        })
}

/// Attach a one-shot arrival swirl as a child of the visitor
/// sprite. Reads as the door opening and a gust of dust rolling
/// in around the visitor when they step into the bunker.
///
/// Parenting to the sprite means the emitter co-despawns when the
/// visitor sprite does (dialogue end / dismissal), so we don't
/// leak emitter entities across visitors. The [`EffectAsset`] is
/// shared via [`EventEffectAssets`] so each admit reuses the same
/// compiled effect.
pub fn attach_visitor_arrival_swirl(
    commands: &mut Commands,
    assets: &EventEffectAssets,
    visitor_sprite: Entity,
) {
    let child = commands
        .spawn((
            Name::new("bunker_visitor_swirl"),
            ParticleEffect::new(assets.visitor_swirl.clone()),
            // Drop below the sprite centre so the swirl pools at
            // the visitor's feet, not their chest.
            Transform::from_translation(Vec3::new(0.0, -0.6, 0.0)),
        ))
        .id();
    commands.entity(visitor_sprite).add_child(child);
}

/// ~40 warm off-white particles exploding outward in the XZ
/// plane, short lifetime.
fn build_visitor_swirl_effect() -> EffectAsset {
    let spawner = SpawnerSettings::once(40.0.into());
    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        (writer.lit(0.8) + writer.rand(ScalarType::Float) * writer.lit(0.4)).expr(),
    );

    let init_pos = SetPositionCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Y).expr(),
        radius: writer.lit(0.15).expr(),
        dimension: ShapeDimension::Volume,
    };

    let init_vel = SetVelocityCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Y).expr(),
        speed: (writer.lit(0.6) + writer.rand(ScalarType::Float) * writer.lit(0.7)).expr(),
    };

    // Drag value 3.5 eyeballed against 0.6–1.3 m/s initial speed
    // so the swirl covers ~20–30 cm before settling; retune
    // alongside speed if you change either.
    let mut module = writer.finish();
    let drag = LinearDragModifier::new(module.lit(3.5));

    // Warm off-white — reads as "outside air rolling in", not
    // "corpse gray dust".
    let mut color_grad = HanabiGradient::new();
    color_grad.add_key(0.0, Vec4::new(1.0, 0.95, 0.85, 0.0));
    color_grad.add_key(0.2, Vec4::new(1.0, 0.95, 0.85, 0.55));
    color_grad.add_key(1.0, Vec4::new(0.9, 0.85, 0.75, 0.0));

    let mut size_grad = HanabiGradient::new();
    size_grad.add_key(0.0, Vec3::splat(0.04));
    size_grad.add_key(1.0, Vec3::splat(0.18));

    EffectAsset::new(128, spawner, module)
        .with_name("visitor_swirl")
        .init(init_pos)
        .init(init_vel)
        .init(init_age)
        .init(init_lifetime)
        .update(drag)
        .render(ColorOverLifetimeModifier {
            gradient: color_grad,
            ..default()
        })
        .render(SizeOverLifetimeModifier {
            gradient: size_grad,
            screen_space_size: false,
        })
}
