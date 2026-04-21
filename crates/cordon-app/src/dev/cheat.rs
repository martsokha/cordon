//! Dev cheats — keybindings that mutate game state for testing.
//!
//! Cheat keys:
//! - **F3** — toggle map fog of war
//! - **F4** — cycle time-scale (1× → 4× → 16× → 64× → 1×)

use bevy::prelude::*;

use crate::laptop::fog::FogEnabled;

pub struct CheatPlugin;

impl Plugin for CheatPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TimeAcceleration::default());
        app.add_systems(
            Update,
            (cheat_toggle_fog, cheat_cycle_time_scale, apply_time_scale),
        );
    }
}

/// Player-selected time scale. Applied to `Time<Virtual>` so every
/// sim system that reads `delta_secs()` accelerates in lockstep
/// (combat, movement, goals, throttles, fire cooldowns). Real-time
/// systems that should not accelerate (UI smoothing, camera lerp)
/// must read `Res<Time<Real>>` explicitly.
#[derive(Resource, Debug, Clone, Copy)]
struct TimeAcceleration {
    multiplier: f32,
}

impl Default for TimeAcceleration {
    fn default() -> Self {
        Self { multiplier: 1.0 }
    }
}

/// Time-scale presets cycled by F4.
const TIME_SCALE_PRESETS: &[f32] = &[1.0, 4.0, 16.0, 64.0];

/// Push [`TimeAcceleration.multiplier`] into [`SimSpeed`] so
/// sim systems that read `Time<Sim>` see the scaled delta.
fn apply_time_scale(
    accel: Res<TimeAcceleration>,
    mut sim_speed: ResMut<cordon_sim::resources::SimSpeed>,
) {
    if !accel.is_changed() {
        return;
    }
    sim_speed.0 = accel.multiplier.max(0.0) as f64;
}

/// F4 → cycle through [`TIME_SCALE_PRESETS`].
fn cheat_cycle_time_scale(keys: Res<ButtonInput<KeyCode>>, mut accel: ResMut<TimeAcceleration>) {
    if !keys.just_pressed(KeyCode::F4) {
        return;
    }
    let current = accel.multiplier;
    // Next preset strictly greater than current; wrap to smallest.
    let next = TIME_SCALE_PRESETS
        .iter()
        .copied()
        .find(|&s| s > current + 0.01)
        .unwrap_or(TIME_SCALE_PRESETS[0]);
    accel.multiplier = next;
    info!("cheat: time scale → {next}×");
}

/// F3 → toggle map fog of war. Reveals every area and shows every
/// NPC/relic regardless of player line of sight.
fn cheat_toggle_fog(keys: Res<ButtonInput<KeyCode>>, mut fog: ResMut<FogEnabled>) {
    if !keys.just_pressed(KeyCode::F3) {
        return;
    }
    fog.enabled = !fog.enabled;
    info!("cheat: fog {}", if fog.enabled { "on" } else { "off" });
}
