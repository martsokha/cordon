//! Footstep scuff: one-shot dust puff per step.

use bevy::prelude::*;
use bevy_hanabi::Gradient as HanabiGradient;
use bevy_hanabi::prelude::*;

use super::EmitterTtl;
use crate::bunker::input::controller::FootstepScuffed;

use super::EventEffectAssets;

const SCUFF_TTL_SECS: f32 = 0.7;

pub(super) fn spawn_footstep_scuffs(
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

pub(super) fn build_footstep_scuff_effect() -> EffectAsset {
    let spawner = SpawnerSettings::once(8.0.into());
    let writer = ExprWriter::new();

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        (writer.lit(0.3) + writer.rand(ScalarType::Float) * writer.lit(0.2)).expr(),
    );

    let init_pos = SetPositionCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Y).expr(),
        radius: writer.lit(0.04).expr(),
        dimension: ShapeDimension::Volume,
    };

    let init_vel = SetVelocityCircleModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        axis: writer.lit(Vec3::Y).expr(),
        speed: (writer.lit(0.3) + writer.rand(ScalarType::Float) * writer.lit(0.4)).expr(),
    };

    let mut module = writer.finish();
    let drag = LinearDragModifier::new(module.lit(5.0));

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
