use bevy::prelude::*;

use super::flicker::Flickering;
use crate::bunker::geometry::{Prop, PropPlacement};

/// Derive a stable flicker rng seed from a fixture's light position.
/// FNV-1a over the bit-patterns of the three coords so tiny position
/// differences fan out into different schedules.
fn flicker_seed_from_pos(pos: Vec3) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for coord in [pos.x, pos.y, pos.z] {
        let bits = coord.to_bits() as u64;
        hash ^= bits;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

/// Radius of the emissive bulb proxy — a small self-lit sphere
/// spawned at each fixture's light position. The sphere is what
/// the bloom pass latches onto so the fixture reads as a glowing
/// source rather than an unlit shade with an invisible point
/// light floating next to it.
const BULB_RADIUS: f32 = 0.04;

/// Multiplier from `PointLight::intensity` (lumens) to the bulb
/// sphere's `emissive` scalar. Tuned so a 50k-lumen ceiling lamp
/// blooms brightly without clipping the tonemapper at typical
/// exposure.
const BULB_EMISSIVE_SCALE: f32 = 1.0 / 6000.0;

/// Kind of light source a fixture emits. Ceiling and desk lamps
/// project a *cone* downward — this reads as a real-world
/// enclosed fixture casting a lit pool on the surface below —
/// while standing lamps and screen glows are omnidirectional
/// so the light wraps around the viewer naturally.
#[derive(Clone, Copy)]
enum LightKind {
    /// Omnidirectional — a bare bulb or CRT glow.
    Point,
    /// Downward cone. The light's transform is rotated so its
    /// local −Z axis (the forward vector Bevy uses for spot
    /// lights) points straight down.
    SpotDown,
}

/// A light fixture spawner: an optional prop model plus a matching
/// point/spot light, with an optional bloom-anchor bulb sphere at
/// the light anchor.
///
/// Models are referenced via [`Prop`] so the same registry +
/// feet-centre placement math that every other prop uses applies
/// here too (no stringly-typed path duplication). Fixtures that
/// don't have a physical model (a screen glow, a light source in
/// the middle of the air) set `model = None`.
pub struct LightFixtureBundle {
    /// Prop to spawn as the fixture's body, if any. `None` for
    /// pure light sources (e.g. a CRT screen glow).
    pub model: Option<Prop>,
    /// Feet-centre position for the prop. Interpreted the same
    /// way `PropPlacement::new` interprets it: AABB bottom sits
    /// at `model_pos.y`.
    pub model_pos: Vec3,
    /// Rotation for the prop.
    pub model_rot: Quat,
    /// Where the actual light source sits (the bulb, not the base).
    pub light_pos: Vec3,
    /// Light intensity in lumens.
    pub intensity: f32,
    /// Light color.
    pub color: Color,
    /// Light range.
    pub range: f32,
    /// Whether to cast shadows.
    pub shadows: bool,
    /// Emission pattern (point vs. downward cone).
    kind: LightKind,
    /// Whether to spawn the small emissive sphere at `light_pos`
    /// as a bloom anchor. On for high fixtures (ceiling lamps)
    /// where the proxy fills the role the model's own shade
    /// can't at distance. Off for desk / standing / screen
    /// fixtures whose GLB models (or on-screen surface) already
    /// carry the "lit" read and where a floating proxy sphere
    /// would read as a mystery ball hovering above the object.
    visible_bulb: bool,
    /// Outer cone angle override for [`LightKind::SpotDown`]. When
    /// `None` the default 55° is used. Widen this for corridor
    /// lights that need to blanket a longer stretch of hallway
    /// rather than punch a tight pool.
    outer_angle: Option<f32>,
}

impl LightFixtureBundle {
    /// Ceiling lamp with the bulb hanging ~0.35m below the ceiling.
    /// `ceiling_h` is the ceiling's world Y — the lamp's AABB top
    /// lands there so the fixture appears to hang from the ceiling.
    /// Emits as a downward cone so the floor below gets a lit
    /// pool instead of a uniform spherical wash.
    pub fn ceiling(
        x: f32,
        z: f32,
        ceiling_h: f32,
        intensity: f32,
        color: Color,
        shadows: bool,
    ) -> Self {
        // `PropPlacement::pos` is feet-centre (AABB bottom). The
        // CeilingLamp's AABB spans y ∈ [-0.54, 0.0] in local
        // space, so placing its AABB bottom at
        // `ceiling_h - 0.54` lines the lamp's top flush with the
        // ceiling.
        let aabb_min_y = Prop::CeilingLamp.def().aabb_min.y;
        Self {
            model: Some(Prop::CeilingLamp),
            model_pos: Vec3::new(x, ceiling_h + aabb_min_y, z),
            model_rot: Quat::IDENTITY,
            light_pos: Vec3::new(x, ceiling_h - 0.35, z),
            intensity,
            color,
            range: 10.0,
            shadows,
            kind: LightKind::SpotDown,
            visible_bulb: true,
            outer_angle: None,
        }
    }

    /// Standing floor lamp with the bulb at shade height (~1.4m).
    pub fn standing(x: f32, z: f32, intensity: f32, color: Color) -> Self {
        Self {
            model: Some(Prop::StandingLamp),
            model_pos: Vec3::new(x, 0.0, z),
            model_rot: Quat::IDENTITY,
            light_pos: Vec3::new(x, 1.4, z),
            intensity,
            color,
            range: 3.5,
            shadows: false,
            kind: LightKind::Point,
            visible_bulb: false,
            outer_angle: None,
        }
    }

    /// Small desk/table lamp. Model sits at `pos` (feet-centre on
    /// the desk surface); light sits slightly above. Emits a tight
    /// downward cone onto the desk surface.
    pub fn desk(pos: Vec3, intensity: f32, color: Color) -> Self {
        Self {
            model: Some(Prop::Lamp1),
            model_pos: pos,
            model_rot: Quat::IDENTITY,
            light_pos: pos + Vec3::new(0.0, 0.3, 0.0),
            intensity,
            color,
            range: 2.5,
            shadows: false,
            kind: LightKind::SpotDown,
            visible_bulb: false,
            outer_angle: None,
        }
    }

    /// A screen glow (no model — the screen itself is the source).
    pub fn screen(pos: Vec3, intensity: f32, color: Color) -> Self {
        Self {
            model: None,
            model_pos: pos,
            model_rot: Quat::IDENTITY,
            light_pos: pos,
            intensity,
            color,
            range: 2.0,
            shadows: false,
            kind: LightKind::Point,
            visible_bulb: false,
            outer_angle: None,
        }
    }

    /// Widen the spotlight cone and extend its range. Only affects
    /// [`LightKind::SpotDown`] fixtures; ignored by point lights.
    /// Used for hall / corridor ceiling lights that need to cover
    /// more floor than the default 55° tight pool.
    pub fn wide(mut self) -> Self {
        self.outer_angle = Some(80.0_f32.to_radians());
        self.range = self.range.max(14.0);
        self
    }

    /// Spawn the fixture's prop (if any), a small emissive "bulb"
    /// sphere at the light anchor (if the fixture wants one), and
    /// the matching point/spot light.
    ///
    /// The bulb proxy is what bloom attaches to so the fixture
    /// reads as a glowing source rather than an unlit shade
    /// with an invisible light next to it. Emissive scale is
    /// derived from the point light's lumens so brighter
    /// fixtures glow harder automatically.
    pub fn spawn(
        &self,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        mats: &mut Assets<StandardMaterial>,
    ) {
        if let Some(prop) = self.model {
            // Route model spawns through the prop registry so the
            // `resolve_prop_placement` observer handles scene
            // loading, feet-centre correction, and collider
            // spawning consistently.
            commands.spawn(PropPlacement::new(prop, self.model_pos).rotated(self.model_rot));
        }

        if self.visible_bulb {
            let emissive_scalar = self.intensity * BULB_EMISSIVE_SCALE;
            let emissive = {
                let c = self.color.to_linear();
                LinearRgba::new(
                    c.red * emissive_scalar,
                    c.green * emissive_scalar,
                    c.blue * emissive_scalar,
                    1.0,
                )
            };
            let bulb_mat = mats.add(StandardMaterial {
                base_color: Color::BLACK,
                emissive,
                unlit: true,
                ..default()
            });
            commands.spawn((
                Mesh3d(meshes.add(Sphere::new(BULB_RADIUS))),
                MeshMaterial3d(bulb_mat),
                Transform::from_translation(self.light_pos),
            ));
        }

        // Seed the flicker rng off the fixture's position so every
        // light gets a different random schedule without a global
        // counter. Position is stable across runs (level layout is
        // deterministic) so bursts fire at the same relative times
        // from session to session.
        let flicker_seed = flicker_seed_from_pos(self.light_pos);
        let flickering = Flickering::new(self.intensity, flicker_seed);

        match self.kind {
            LightKind::Point => {
                commands.spawn((
                    PointLight {
                        intensity: self.intensity,
                        color: self.color,
                        range: self.range,
                        shadows_enabled: self.shadows,
                        ..default()
                    },
                    Transform::from_translation(self.light_pos),
                    flickering,
                ));
            }
            LightKind::SpotDown => {
                // Rotate the light so its local −Z (Bevy's
                // forward) points straight down. Default cone
                // widens from a tight 30° core to a 55° falloff
                // — narrow enough to read as a fixture's lit
                // pool, wide enough to light the whole corridor
                // cross-section from a standard ceiling height.
                // Corridor lights override this via `wide()` so
                // their pools overlap along the hallway.
                let outer_angle = self.outer_angle.unwrap_or(55.0_f32.to_radians());
                commands.spawn((
                    SpotLight {
                        intensity: self.intensity,
                        color: self.color,
                        range: self.range,
                        shadows_enabled: self.shadows,
                        inner_angle: std::f32::consts::FRAC_PI_6, // 30°
                        outer_angle,
                        ..default()
                    },
                    Transform::from_translation(self.light_pos).looking_to(Vec3::NEG_Y, Vec3::Z),
                    flickering,
                ));
            }
        }
    }
}
