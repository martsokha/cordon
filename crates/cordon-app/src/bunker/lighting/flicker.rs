//! Subtle bunker-wide light flicker.
//!
//! Attaches a [`Flickering`] component to every light source (and
//! its bloom-anchor bulb proxy) spawned by [`LightFixtureBundle`].
//! A system runs rare, short flicker bursts on each flickering
//! entity — mostly the lights sit at their base intensity; every
//! 15-40 seconds they dip and stutter for a fraction of a second,
//! then settle.
//!
//! Per-fixture phase is randomized so fixtures don't flicker in
//! lockstep.
//!
//! Scripting: the [`FlickerEnabled`] resource gates the whole
//! system. External code can flip it on/off with
//! `commands.insert_resource(FlickerEnabled(false))` (or mutate
//! via `ResMut`). When disabled, any in-flight burst is ended and
//! lights snap back to their base intensity.

use bevy::prelude::*;
use bevy_prng::WyRand;
use rand::{RngExt, SeedableRng};

/// Toggle flicker on/off from scripts. Default: on.
#[derive(Resource, Debug, Clone, Copy)]
pub struct FlickerEnabled(pub bool);

impl Default for FlickerEnabled {
    fn default() -> Self {
        Self(true)
    }
}

/// Attach to a light entity (and its optional bulb proxy) to
/// participate in the bunker-wide flicker.
///
/// `base_intensity` snapshots the fixture's nominal intensity at
/// spawn — the flicker modulates around this value and restores it
/// when a burst ends (or when the whole system is disabled).
#[derive(Component, Debug, Clone)]
pub struct Flickering {
    /// The fixture's nominal intensity. Set once at spawn; never
    /// overwritten so we can always snap back to steady state.
    pub base_intensity: f32,
    /// Seconds of steady illumination remaining before the next
    /// burst starts. Randomized at spawn and after every burst.
    steady_remaining: f32,
    /// Active burst state, if any.
    burst: Option<Burst>,
    /// Per-fixture rng so the timing desyncs between entities.
    rng: WyRand,
}

#[derive(Debug, Clone)]
struct Burst {
    /// Seconds remaining in the current burst.
    remaining: f32,
    /// How long until the next intra-burst "beat" fires. Inside a
    /// burst the intensity jumps to a new random multiplier on
    /// each beat rather than sampling per-frame — reads as a
    /// distinct stutter instead of noise.
    beat_in: f32,
    /// The current multiplier applied to `base_intensity` until
    /// the next beat.
    current_mult: f32,
}

impl Flickering {
    const MAX_BEAT: f32 = 0.09;
    const MAX_BURST: f32 = 0.45;
    const MAX_MULT: f32 = 1.05;
    const MAX_STEADY: f32 = 40.0;
    /// Within a burst, each intensity beat holds for this long.
    const MIN_BEAT: f32 = 0.03;
    /// A burst lasts this long total.
    const MIN_BURST: f32 = 0.15;
    /// Multiplier range during a burst. Never goes to 0 — a
    /// fully-black frame reads as the power failing rather than
    /// a flicker. Never goes above 1.05 either; we want dips,
    /// not strobes.
    const MIN_MULT: f32 = 0.35;
    /// Seconds between bursts. Wide spread so adjacent fixtures
    /// rarely fire together.
    const MIN_STEADY: f32 = 15.0;

    pub fn new(base_intensity: f32, seed: u64) -> Self {
        let mut rng = WyRand::seed_from_u64(seed);
        let steady_remaining = rng.random_range(Self::MIN_STEADY..Self::MAX_STEADY);
        Self {
            base_intensity,
            steady_remaining,
            burst: None,
            rng,
        }
    }

    /// Advance the flicker clock. Returns the multiplier to apply
    /// to `base_intensity` this frame.
    fn tick(&mut self, dt: f32) -> f32 {
        if let Some(burst) = &mut self.burst {
            burst.remaining -= dt;
            burst.beat_in -= dt;
            if burst.remaining <= 0.0 {
                self.burst = None;
                self.steady_remaining = self.rng.random_range(Self::MIN_STEADY..Self::MAX_STEADY);
                return 1.0;
            }
            if burst.beat_in <= 0.0 {
                burst.current_mult = self.rng.random_range(Self::MIN_MULT..Self::MAX_MULT);
                burst.beat_in = self.rng.random_range(Self::MIN_BEAT..Self::MAX_BEAT);
            }
            return burst.current_mult;
        }

        self.steady_remaining -= dt;
        if self.steady_remaining <= 0.0 {
            let total = self.rng.random_range(Self::MIN_BURST..Self::MAX_BURST);
            let first_mult = self.rng.random_range(Self::MIN_MULT..Self::MAX_MULT);
            let first_beat = self.rng.random_range(Self::MIN_BEAT..Self::MAX_BEAT);
            self.burst = Some(Burst {
                remaining: total,
                beat_in: first_beat,
                current_mult: first_mult,
            });
            return first_mult;
        }
        1.0
    }

    /// Force the fixture back to steady state and queue a fresh
    /// steady interval. Used when [`FlickerEnabled`] flips off
    /// mid-burst.
    fn reset_to_steady(&mut self) {
        self.burst = None;
        self.steady_remaining = self.rng.random_range(Self::MIN_STEADY..Self::MAX_STEADY);
    }
}

pub struct FlickerPlugin;

impl Plugin for FlickerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlickerEnabled>();
        app.add_systems(Update, tick_flicker);
    }
}

/// Shared tick for every flickering light source. Runs in `Update`
/// every frame regardless of game state — light atmospherics don't
/// pause with the sim.
fn tick_flicker(
    time: Res<Time>,
    enabled: Res<FlickerEnabled>,
    mut point_lights: Query<(&mut Flickering, &mut PointLight)>,
    mut spot_lights: Query<(&mut Flickering, &mut SpotLight), Without<PointLight>>,
) {
    let dt = time.delta_secs();

    if !enabled.0 {
        // Disabled: snap every flickering fixture back to steady
        // state and clear any in-flight burst. Cheap — only touches
        // fixtures mid-burst thanks to the `is_some` gate.
        for (mut flick, mut light) in &mut point_lights {
            if flick.burst.is_some() || light.intensity != flick.base_intensity {
                flick.reset_to_steady();
                light.intensity = flick.base_intensity;
            }
        }
        for (mut flick, mut light) in &mut spot_lights {
            if flick.burst.is_some() || light.intensity != flick.base_intensity {
                flick.reset_to_steady();
                light.intensity = flick.base_intensity;
            }
        }
        return;
    }

    for (mut flick, mut light) in &mut point_lights {
        let mult = flick.tick(dt);
        light.intensity = flick.base_intensity * mult;
    }
    for (mut flick, mut light) in &mut spot_lights {
        let mult = flick.tick(dt);
        light.intensity = flick.base_intensity * mult;
    }
}
