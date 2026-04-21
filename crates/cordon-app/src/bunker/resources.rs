use bevy::prelude::*;
use bevy_yarnspinner::prelude::OptionId;

#[derive(Resource)]
pub struct BunkerSpawned;

#[derive(Resource)]
pub struct MovementLocked;

/// When present as a resource, all interactions are blocked. The
/// visitor module inserts this while a visitor is inside the bunker
/// so the player can't escape mid-conversation.
#[derive(Resource)]
pub struct InteractionLocked;

#[derive(Resource)]
pub struct LaptopPlacement {
    pub pos: Vec3,
    pub rot: Quat,
}

#[derive(Resource)]
pub struct RadioPlacement {
    pub pos: Vec3,
    pub rot: Quat,
}

pub const ANTECHAMBER_VISITOR_POS: Vec3 = Vec3::new(0.0, -49.75, -49.5);
// 15 cm inside the left wall of the antechamber (wall at x = -1.5),
// mounted high in the front-left corner looking down at the
// visitor. Bound to the antechamber's world offset (y = -49,
// z = -50) so the camera is inside the room, not the main bunker.
pub(crate) const CCTV_CAMERA_POS: Vec3 = Vec3::new(-1.35, -47.9, -48.15);

/// Camera zoomed to laptop. desk_z=1.0, laptop at y≈1.05.
pub(crate) const LAPTOP_VIEW_POS: Vec3 = Vec3::new(0.0, 1.35, 0.5);
pub(crate) const LAPTOP_VIEW_TARGET: Vec3 = Vec3::new(0.0, 1.10, 1.12);
pub(crate) const CAMERA_LERP_SPEED: f32 = 6.0;

#[derive(Resource, Clone)]
pub enum CameraMode {
    Free,
    AtLaptop {
        saved_transform: Transform,
    },
    Returning(Transform),
    /// Smoothly turn (rotation only) to face a world-space point.
    /// Used while a visitor is inside the bunker. The position is
    /// untouched — the player stays where they were standing.
    /// No saved transform: dialogue-end transitions leave the
    /// camera where it is rather than snapping back to the
    /// admit-time pose.
    LookingAt {
        target: Vec3,
    },
    /// Player is studying the CCTV feed in fullscreen. The CCTV
    /// camera takes over the window and the FPS camera goes
    /// inactive until the player presses E or Esc.
    AtCctv {
        saved_transform: Transform,
    },
}

/// Shared material handles used across every room.
///
/// `concrete` and `concrete_dark` are full PBR materials sharing
/// one ambientCG texture set (Concrete044C) with only the
/// base-colour tint differing — keeps GPU memory flat while still
/// reading as two distinct surfaces. `wood` and `metal` stay flat
/// because their call sites are tiny accents (a 25 cm counter top
/// and 2 cm grate bars respectively) where tiling a full PBR set
/// would be more noise than signal.
///
/// Meshes that use these materials are expected to carry UV data
/// scaled to physical dimensions (see
/// [`geometry::cuboid_tiled`](super::geometry::cuboid_tiled) and
/// siblings) — the shared texture samplers are set to `Repeat` so
/// tiling falls out correctly.
pub(crate) struct Palette {
    pub concrete: Handle<StandardMaterial>,
    pub concrete_dark: Handle<StandardMaterial>,
    pub wood: Handle<StandardMaterial>,
    pub metal: Handle<StandardMaterial>,
}

impl Palette {
    pub(crate) fn new(mats: &mut Assets<StandardMaterial>, asset_server: &AssetServer) -> Self {
        // One texture set for every structural concrete surface
        // (walls, floor, ceiling). Using separate textures for
        // walls vs. floor created a visible seam at the wall/
        // floor edge because the two materials' tones didn't
        // quite match — real bunker interiors are usually one
        // continuous pour, so mirroring that in-game reads
        // correctly. `concrete` and `concrete_dark` both bind
        // the same handle for now; the pair is kept so future
        // accent-surface variants can slot in without touching
        // every call site.
        let concrete_set = super::textures::TextureSet::load_ambient_cg(
            asset_server,
            "Concrete024_1K-JPG",
            "Concrete024_1K-JPG",
        );

        let concrete = mats.add(concrete_material(&concrete_set, Color::WHITE));
        let concrete_dark = concrete.clone();

        Self {
            concrete,
            concrete_dark,
            wood: mats.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.16, 0.10),
                perceptual_roughness: 0.85,
                ..default()
            }),
            metal: mats.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.28, 0.26),
                perceptual_roughness: 0.5,
                metallic: 0.6,
                ..default()
            }),
        }
    }
}

/// Build a concrete `StandardMaterial` from a shared texture set
/// with a tint. `metallic: 0.0` deliberately zeroes out the
/// metallic channel of `metallic_roughness_texture` (see
/// [`textures`](super::textures) module docs).
fn concrete_material(set: &super::textures::TextureSet, tint: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: tint,
        base_color_texture: Some(set.base_color.clone()),
        normal_map_texture: set.normal.clone(),
        metallic_roughness_texture: set.metallic_roughness.clone(),
        occlusion_texture: set.ambient_occlusion.clone(),
        // Parallax mapping: adds perceived depth from the height
        // map so the concrete looks poured, not painted-flat.
        // 5 cm reads as real formwork relief without distorting
        // grazing-angle silhouettes (beyond ~8 cm the edges
        // warp obviously).
        depth_map: set.depth.clone(),
        parallax_depth_scale: 0.05,
        parallax_mapping_method: ParallaxMappingMethod::Relief { max_steps: 4 },
        max_parallax_layer_count: 16.0,
        metallic: 0.0,
        perceptual_roughness: 1.0,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    }
}

/// Bundle of references every per-room spawner needs. Collapses what
/// used to be 6 positional arguments into `ctx: &mut RoomCtx`, which
/// makes adding a new room mechanical and keeps call sites uniform.
pub(crate) struct RoomCtx<'a, 'w, 's> {
    pub commands: &'a mut Commands<'w, 's>,
    pub meshes: &'a mut Assets<Mesh>,
    pub mats: &'a mut Assets<StandardMaterial>,
    pub effects: &'a mut Assets<bevy_hanabi::EffectAsset>,
    pub pal: &'a Palette,
    pub l: &'a Layout,
    /// Player upgrades — so rooms can gate visuals on what the
    /// player has unlocked/installed (e.g. rack upgrades that add
    /// storage racks in the hall).
    pub upgrades: &'a cordon_sim::resources::PlayerUpgrades,
    /// Game-data catalog. Rooms that resolve `UpgradeEffect`s on
    /// the player's installed upgrades need this to look up effect
    /// lists by upgrade id.
    pub game_data: &'a cordon_data::gamedata::GameDataResource,
}

/// Bunker dimensions. Only stores the primary constants; derived
/// values are computed via methods so nothing can go stale.
///
/// The corridor has **two T-junctions**:
/// - **T1** (kitchen / quarters) — the original pair, at the back
///   of the original corridor span. Z range `[tj1_north, tj1_south]`.
/// - **T2** (infirmary / workshop) — the newer pair, past T1 on
///   the way to the back wall. Z range `[tj2_north, tj2_south]`.
///
/// Between `tj1_south` and `tj2_north` sits a short straight hall
/// so the corridor reads as “two branching zones with a passage
/// between them” rather than a single mashed-together space.
pub(crate) struct Layout {
    /// Ceiling height.
    pub h: f32,
    /// Main corridor half-width (x extent from center).
    pub hw: f32,
    /// Z of the front wall (stairs / entrance).
    pub front_z: f32,
    /// Z of the trade grate.
    pub trade_z: f32,
    /// Z of the office ↔ armory divider grate.
    pub divider_z: f32,
    /// Half-width of the grate opening.
    pub hole_half: f32,
    /// Z of the north edge of the first T-junction (kitchen/quarters).
    pub tj1_north: f32,
    /// Z of the south edge of the first T-junction.
    pub tj1_south: f32,
    /// Z of the north edge of the second T-junction (infirmary/workshop).
    pub tj2_north: f32,
    /// Z of the back wall (south edge of corridor + T2 side rooms).
    pub back_z: f32,
    /// How far each side room extends from the corridor wall.
    pub side_depth: f32,
    /// Width of the side-room doorframe openings.
    pub side_door_width: f32,
}

impl Layout {
    pub(crate) fn new() -> Self {
        // Original back_z was -7.63. Corridor extends past the
        // original T1 by a 2.35 m straight hall — just long
        // enough for two 1.14 m storage racks end-to-end on each
        // wall with a small visual gap — plus a 3 m T2 section.
        Self {
            h: 2.4,
            hw: 2.05,
            front_z: 5.0,
            trade_z: 1.5,
            divider_z: -2.25,
            hole_half: 0.6,
            tj1_north: -4.63,
            tj1_south: -7.63,
            tj2_north: -9.58,
            back_z: -12.58,
            side_depth: 3.0,
            side_door_width: 1.6,
        }
    }

    pub fn hh(&self) -> f32 {
        self.h / 2.0
    }

    /// Height of a walkable doorframe opening. Leaves a 0.3 m air gap
    /// above for the lintel so the frame reads as a real doorway
    /// rather than a ceiling-height hole.
    pub fn opening_h(&self) -> f32 {
        self.h - 0.3
    }

    /// Command desk centre, south of the trade grate. The offset
    /// leaves enough room for the chair + a player-sized gap so
    /// the desk doesn't visually clip the grate bars from inside
    /// the command post.
    pub fn desk_z(&self) -> f32 {
        self.trade_z - 0.6
    }

    /// Centre of the first T-junction's Z extent.
    pub fn tj1_center(&self) -> f32 {
        (self.tj1_north + self.tj1_south) / 2.0
    }

    /// Z-length of the first T-junction (= kitchen/quarters depth).
    pub fn tj1_len(&self) -> f32 {
        self.tj1_north - self.tj1_south
    }

    /// Centre of the second T-junction's Z extent.
    pub fn tj2_center(&self) -> f32 {
        (self.tj2_north + self.back_z) / 2.0
    }

    /// Z-length of the second T-junction (= infirmary/workshop depth).
    pub fn tj2_len(&self) -> f32 {
        self.tj2_north - self.back_z
    }

    /// Kitchen (left of T1): furthest x.
    pub fn kitchen_x_min(&self) -> f32 {
        -(self.hw + self.side_depth)
    }

    /// Kitchen (left of T1): center x.
    pub fn kitchen_x_center(&self) -> f32 {
        (self.kitchen_x_min() + (-self.hw)) / 2.0
    }

    /// Quarters (right of T1): furthest x.
    pub fn quarters_x_max(&self) -> f32 {
        self.hw + self.side_depth
    }

    /// Quarters (right of T1): center x.
    pub fn quarters_x_center(&self) -> f32 {
        (self.hw + self.quarters_x_max()) / 2.0
    }

    /// Infirmary (left of T2): furthest x. Mirrors kitchen's span.
    pub fn infirmary_x_min(&self) -> f32 {
        -(self.hw + self.side_depth)
    }

    /// Infirmary (left of T2): center x.
    pub fn infirmary_x_center(&self) -> f32 {
        (self.infirmary_x_min() + (-self.hw)) / 2.0
    }

    /// Workshop (right of T2): furthest x.
    pub fn workshop_x_max(&self) -> f32 {
        self.hw + self.side_depth
    }

    /// Workshop (right of T2): center x.
    pub fn workshop_x_center(&self) -> f32 {
        (self.hw + self.workshop_x_max()) / 2.0
    }
}

/// What the dialogue UI should currently show. Mirrored from the
/// underlying `DialogueRunner` events so the UI doesn't have to know
/// about Yarn types directly.
#[derive(Resource, Default, Debug, Clone)]
pub enum CurrentDialogue {
    /// No dialogue is active.
    #[default]
    Idle,
    /// A line is being shown. The UI should render it and present a
    /// "Continue" affordance that emits a [`DialogueChoice::Continue`].
    ///
    /// When `autocontinue` is true, the line doesn't render in the
    /// UI at all — the `PresentLine` observer stashes its text into
    /// `PendingOptionsPrompt` and queues an auto-Continue, so by
    /// the time the following `Options` state fires, the prompt
    /// travels with it via `OptionsPrompt`. The `autocontinue` flag
    /// on the Line variant lets the UI short-circuit without
    /// flashing an empty row mid-transition.
    Line {
        speaker: Option<String>,
        text: String,
        autocontinue: bool,
    },
    /// A set of options is presented. The UI should render the lines
    /// as buttons; selecting one emits [`DialogueChoice::Option`].
    Options {
        lines: Vec<DialogueOptionView>,
        /// Prompt line header to render above the options. Set when
        /// the preceding Line had `#autocontinue` — that line's
        /// text survives as the options' context instead of being
        /// a stand-alone Line→Options transition the UI has to
        /// infer. `None` when options appear without a preceding
        /// prompt (e.g. first frame of a node that starts with
        /// options, step-away resume).
        prompt: Option<OptionsPrompt>,
    },
}

/// Prompt header attached to an [`CurrentDialogue::Options`] state
/// when the preceding Line was `#autocontinue`. Carries the line's
/// speaker and text so the UI can render the prompt above the
/// choice buttons without relying on frame-order heuristics.
#[derive(Debug, Clone)]
pub struct OptionsPrompt {
    pub speaker: Option<String>,
    pub text: String,
}

/// Holds the text of an `#autocontinue` line until the following
/// [`CurrentDialogue::Options`] consumes it as a prompt header.
/// Exists as a resource so `on_present_options` can attach the
/// prompt regardless of which order observers fire in.
#[derive(Resource, Default, Debug, Clone)]
pub struct PendingOptionsPrompt(pub Option<OptionsPrompt>);

/// Player-facing view of a single dialogue option.
#[derive(Debug, Clone)]
pub struct DialogueOptionView {
    pub id: OptionId,
    pub text: String,
    /// Yarn-evaluated `<<if>>` truth. `false` means the option
    /// was authored with a condition that failed (e.g. gated on
    /// `$carrying`). The UI renders unavailable options greyed
    /// and refuses clicks unless [`hide_when_unavailable`] is
    /// also set, in which case the option is skipped entirely.
    pub available: bool,
    /// `true` when the authored line carries the `#hide`
    /// metadata tag. Combined with `available = false`, tells
    /// the UI to skip the option rather than show it greyed —
    /// for trade contexts where we want *one* of two paired
    /// options to appear based on state (e.g. "Here, take this"
    /// vs. "I've got one in the back").
    pub hide_when_unavailable: bool,
}

/// Which subsystem owns a currently-running (or just-completed)
/// dialog. Tagged on every [`StartDialogue`] so observers of
/// `DialogueCompleted` can tell "was this mine?" without reading
/// each other's internal state.
///
/// The enum is intentionally coarse — just enough to route the
/// shared dialog UI back to its owning subsystem. Fine-grained
/// identity (which quest? which NPC?) stays in the subsystem's
/// own resources (e.g. `DialogueInFlight`); this tag only asks
/// "which concern?"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogueOwner {
    /// No dialog has run yet this session, or the last owner has
    /// been cleared after its completion window.
    #[default]
    None,
    /// A quest's Talk stage — visitor-driven or narrator-only.
    Quest,
    /// A bunker visitor's conversation (admit or step-away resume).
    Visitor,
    /// A radio broadcast or the idle static yarn.
    Radio,
}

/// Tracks the owner of the current (or most recently completed)
/// dialog. Set by [`apply_start_dialogue`] when a new dialog
/// starts; cleared one frame after [`DialogueCompleted`] so
/// observers watching the completion have the owner readable in
/// the same frame the event fires.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct CurrentDialogueOwner(pub DialogueOwner);

/// Sent by upstream code (visitor, quest bridge, radio) to begin
/// a conversation at the given yarn node. Resolved by
/// [`apply_start_dialogue`].
///
/// `by` identifies the subsystem starting the dialog so a
/// [`DialogueCompleted`] observer can route the event to the
/// right handler without coupling subsystems to each other.
#[derive(Message, Debug, Clone)]
pub struct StartDialogue {
    pub node: String,
    pub by: DialogueOwner,
}

/// Sent by the run-reset plumbing to abandon any in-flight Yarn
/// dialogue. Calls `DialogueRunner::stop()` and forces
/// `CurrentDialogue` back to Idle so the next run starts clean.
#[derive(Message, Debug, Clone, Copy)]
pub struct StopDialogue;

/// Player-side message: the UI emits one of these when the player
/// either continues past a line or picks an option.
#[derive(Message, Debug, Clone, Copy)]
pub enum DialogueChoice {
    Continue,
    Option { id: OptionId },
}
