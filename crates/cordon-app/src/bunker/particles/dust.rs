//! Room dust motes: one emitter per room volume.

use bevy::prelude::*;
use bevy_hanabi::Gradient as HanabiGradient;
use bevy_hanabi::prelude::*;

use crate::bunker::resources::Layout;

#[derive(Component)]
struct DustEmitter;

#[derive(Resource)]
pub(super) struct BunkerDustSpawned;

const DUST_WARM: Vec4 = Vec4::new(0.65, 0.60, 0.52, 0.4);
const DUST_COOL: Vec4 = Vec4::new(0.52, 0.58, 0.65, 0.35);

pub(super) fn spawn_dust_emitters(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let l = Layout::new();

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
        5.0,
        DUST_WARM,
    );

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
        2.0,
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
        2.0,
        DUST_WARM,
    );

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
        2.0,
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
        2.0,
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

fn build_dust_effect(half_extents: Vec3, rate: f32, color: Vec4) -> EffectAsset {
    let spawner = SpawnerSettings::rate(rate.into());
    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        (writer.lit(6.0) + writer.rand(ScalarType::Float) * writer.lit(4.0)).expr(),
    );
    let rand_vec = writer.rand(VectorType::VEC3F);
    let pos = (rand_vec * writer.lit(2.0) - writer.lit(Vec3::ONE)) * writer.lit(half_extents);
    let init_pos = SetAttributeModifier::new(Attribute::POSITION, pos.expr());

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

    let mut size_grad = HanabiGradient::new();
    size_grad.add_key(0.0, Vec3::splat(0.004));
    size_grad.add_key(1.0, Vec3::splat(0.004));

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
