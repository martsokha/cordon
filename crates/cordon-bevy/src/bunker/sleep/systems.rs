//! Sleep transition: sofa interaction, fade-to-black, time
//! acceleration, fade-back-in.

use bevy::prelude::*;

use avian3d::schedule::{Physics, PhysicsTime};

use crate::bunker::geometry::{Prop, PropPlacement};
use crate::bunker::interaction::{Interact, Interactable};
use crate::bunker::resources::{InteractionLocked, MovementLocked};
use cordon_sim::resources::GameClock;

const SLEEP_HOURS: u32 = 8;
const FADE_SECS: f32 = 0.5;

/// Marker on the sofa entity.
#[derive(Component)]
pub(super) struct SleepTarget;

/// Marker on the fullscreen fade overlay.
#[derive(Component)]
pub(super) struct SleepOverlay;

/// Sleep state machine.
#[derive(Resource, Default)]
pub(super) enum SleepState {
    #[default]
    Awake,
    FadingOut {
        timer: f32,
        target_minutes: u64,
    },
    Sleeping {
        target_minutes: u64,
    },
    FadingIn {
        timer: f32,
    },
}

/// Find WideSofa props and attach an Interactable.
pub(super) fn attach_sleep_target(
    mut commands: Commands,
    sofas: Query<(Entity, &PropPlacement), (With<SceneRoot>, Without<SleepTarget>)>,
) {
    for (entity, placement) in &sofas {
        if placement.kind != Prop::WideSofa {
            continue;
        }
        commands.entity(entity).insert((
            SleepTarget,
            Interactable {
                prompt: "[E] Sleep".into(),
                enabled: true,
            },
        ));
    }
}

/// Wire the observer onto newly-tagged sleep targets.
pub(super) fn attach_observer(
    mut commands: Commands,
    new: Query<Entity, Added<SleepTarget>>,
) {
    for entity in &new {
        commands.entity(entity).observe(on_sleep);
    }
}

fn on_sleep(
    _trigger: On<Interact>,
    mut commands: Commands,
    state: Res<SleepState>,
    clock: Res<GameClock>,
) {
    if !matches!(*state, SleepState::Awake) {
        return;
    }

    let target = clock.0.total_minutes() + (SLEEP_HOURS as u64 * 60);

    commands.insert_resource(SleepState::FadingOut {
        timer: 0.0,
        target_minutes: target,
    });
    commands.insert_resource(MovementLocked);
    commands.insert_resource(InteractionLocked);

    commands.spawn((
        SleepOverlay,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        GlobalZIndex(100),
    ));

    info!(
        "sleep started at {} — target +{} hours",
        clock.0.time_str(),
        SLEEP_HOURS,
    );
}

/// Per-frame driver for the sleep state machine. Uses real time
/// for fades so they take 0.5s regardless of sim speed.
pub(super) fn drive_sleep_transition(
    mut commands: Commands,
    mut state: ResMut<SleepState>,
    real_time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut physics_time: ResMut<Time<Physics>>,
    clock: Res<GameClock>,
    mut overlay_q: Query<&mut BackgroundColor, With<SleepOverlay>>,
    overlay_entities: Query<Entity, With<SleepOverlay>>,
) {
    let dt = real_time.delta_secs();

    match *state {
        SleepState::Awake => {}

        SleepState::FadingOut {
            ref mut timer,
            target_minutes,
        } => {
            *timer += dt;
            let alpha = (*timer / FADE_SECS).min(1.0);
            for mut bg in &mut overlay_q {
                bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
            }
            if *timer >= FADE_SECS {
                physics_time.pause();
                virtual_time.set_relative_speed_f64(50.0);
                *state = SleepState::Sleeping { target_minutes };
            }
        }

        SleepState::Sleeping { target_minutes } => {
            for mut bg in &mut overlay_q {
                bg.0 = Color::srgba(0.0, 0.0, 0.0, 1.0);
            }
            if clock.0.total_minutes() >= target_minutes {
                physics_time.unpause();
                virtual_time.set_relative_speed_f64(1.0);
                *state = SleepState::FadingIn { timer: 0.0 };
                info!("woke up at {}", clock.0.time_str());
            }
        }

        SleepState::FadingIn { ref mut timer } => {
            *timer += dt;
            let alpha = 1.0 - (*timer / FADE_SECS).min(1.0);
            for mut bg in &mut overlay_q {
                bg.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
            }
            if *timer >= FADE_SECS {
                for entity in &overlay_entities {
                    commands.entity(entity).despawn();
                }
                commands.remove_resource::<MovementLocked>();
                commands.remove_resource::<InteractionLocked>();
                *state = SleepState::Awake;
            }
        }
    }
}
