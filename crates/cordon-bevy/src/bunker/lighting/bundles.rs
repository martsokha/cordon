use bevy::prelude::*;

/// A light fixture that spawns both a GLB model and a matching
/// point light at the correct position. The model path determines
/// what kind of fixture it is (ceiling lamp, standing lamp, etc.).
pub struct LightFixtureBundle {
    /// Asset path for the fixture model (e.g. "models/interior/CeilingLamp.glb").
    pub model: &'static str,
    /// Where to place the model's origin.
    pub model_pos: Vec3,
    /// Rotation for the model.
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
}

impl LightFixtureBundle {
    /// Ceiling lamp with the bulb hanging ~0.35m below the ceiling.
    pub fn ceiling(
        x: f32,
        z: f32,
        ceiling_h: f32,
        intensity: f32,
        color: Color,
        shadows: bool,
    ) -> Self {
        Self {
            model: "models/interior/CeilingLamp.glb",
            model_pos: Vec3::new(x, ceiling_h, z),
            model_rot: Quat::IDENTITY,
            light_pos: Vec3::new(x, ceiling_h - 0.35, z),
            intensity,
            color,
            range: 10.0,
            shadows,
        }
    }

    /// Standing floor lamp with the bulb at shade height (~1.4m).
    pub fn standing(x: f32, z: f32, intensity: f32, color: Color) -> Self {
        Self {
            model: "models/interior/StandingLamp.glb",
            model_pos: Vec3::new(x, 0.0, z),
            model_rot: Quat::IDENTITY,
            light_pos: Vec3::new(x, 1.4, z),
            intensity,
            color,
            range: 3.5,
            shadows: false,
        }
    }

    /// Small desk/table lamp. Model placed at `pos`, light slightly above.
    pub fn desk(pos: Vec3, intensity: f32, color: Color) -> Self {
        Self {
            model: "models/interior/Lamp1.glb",
            model_pos: pos,
            model_rot: Quat::IDENTITY,
            light_pos: pos + Vec3::new(0.0, 0.3, 0.0),
            intensity,
            color,
            range: 2.5,
            shadows: false,
        }
    }

    /// A screen glow (no model -- the screen itself is the source).
    pub fn screen(pos: Vec3, intensity: f32, color: Color) -> Self {
        Self {
            model: "",
            model_pos: pos,
            model_rot: Quat::IDENTITY,
            light_pos: pos,
            intensity,
            color,
            range: 2.0,
            shadows: false,
        }
    }

    /// Spawn the fixture model (if any) and its point light.
    ///
    /// Fixtures use direct transform placement rather than [`prop`]'s
    /// feet-center semantics because the light source anchor, not the
    /// model base, is what matters for illumination.
    pub fn spawn(&self, commands: &mut Commands, asset_server: &AssetServer) {
        if !self.model.is_empty() {
            let scene: Handle<Scene> = asset_server.load(format!("{}#Scene0", self.model));
            commands.spawn((
                SceneRoot(scene),
                Transform::from_translation(self.model_pos).with_rotation(self.model_rot),
            ));
        }
        commands.spawn((
            PointLight {
                intensity: self.intensity,
                color: self.color,
                range: self.range,
                shadows_enabled: self.shadows,
                ..default()
            },
            Transform::from_translation(self.light_pos),
        ));
    }
}
