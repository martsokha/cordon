//! Kettle steam plume, parented to the kettle prop.

use bevy::prelude::*;
use bevy_hanabi::Gradient as HanabiGradient;
use bevy_hanabi::prelude::*;

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

fn build_kettle_steam_effect() -> EffectAsset {
    let spawner = SpawnerSettings::rate(35.0.into());
    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        (writer.lit(1.0) + writer.rand(ScalarType::Float) * writer.lit(0.5)).expr(),
    );

    let init_pos = SetPositionCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Y).expr(),
        radius: writer.lit(0.02).expr(),
        dimension: ShapeDimension::Volume,
    };

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
