//! Visitor arrival swirl: one-shot dust burst at the door.

use bevy::prelude::*;
use bevy_hanabi::Gradient as HanabiGradient;
use bevy_hanabi::prelude::*;

use super::EventEffectAssets;

pub fn attach_visitor_arrival_swirl(
    commands: &mut Commands,
    assets: &EventEffectAssets,
    visitor_sprite: Entity,
) {
    let child = commands
        .spawn((
            Name::new("bunker_visitor_swirl"),
            ParticleEffect::new(assets.visitor_swirl.clone()),
            Transform::from_translation(Vec3::new(0.0, -0.6, 0.0)),
        ))
        .id();
    commands.entity(visitor_sprite).add_child(child);
}

pub(super) fn build_visitor_swirl_effect() -> EffectAsset {
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

    let mut module = writer.finish();
    let drag = LinearDragModifier::new(module.lit(3.5));

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
