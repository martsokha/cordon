//! NPC behavior: Action (state machine) + Intent (goal).

use bevy::prelude::*;
use cordon_core::entity::npc::Npc;
use cordon_core::primitive::{Id, Uid};
use cordon_core::world::narrative::quest::Quest;
use moonshine_behavior::prelude::*;

/// What the NPC is physically doing right now.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub enum Action {
    /// Standing around, waiting.
    Idle { timer: f32 },
    /// Moving toward a point.
    Walk { target: Vec2, speed: f32 },
    /// Following another NPC.
    Follow {
        #[reflect(ignore)]
        target: Uid<Npc>,
    },
    /// At the counter, engaged in trade. Timer counts down.
    Trade { timer: f32 },
    /// Running from danger.
    Flee { target: Vec2 },
}

impl Behavior for Action {
    fn filter_next(&self, next: &Self) -> bool {
        use Action::*;
        match_next! {
            self => next,
            Idle { .. } => Walk { .. } | Trade { .. } | Follow { .. } | Flee { .. },
            Walk { .. } => Idle { .. } | Trade { .. } | Follow { .. } | Flee { .. },
            Follow { .. } => Idle { .. } | Walk { .. } | Flee { .. },
            Trade { .. } => Idle { .. } | Walk { .. } | Flee { .. },
            Flee { .. } => Idle { .. } | Walk { .. }
        }
    }
}

/// Why the NPC is doing what they're doing.
///
/// Positional intents store a resolved world position so the behavior
/// system can drive movement without looking up area definitions.
#[allow(dead_code)]
#[derive(Component, Debug, Clone)]
pub enum Intent {
    /// Came to trade at the bunker.
    Visit,
    /// Heading to an area to scavenge loot.
    Scavenge { target: Vec2 },
    /// Faction patrol route through an area.
    Patrol { target: Vec2 },
    /// Pursuing a quest objective.
    Quest(Id<Quest>),
    /// Escorting another NPC.
    Escort(Uid<Npc>),
    /// Looking for work at the bunker.
    Recruit,
    /// Done, heading out of the map.
    Leave,
}

/// Tracks which phase of the intent lifecycle the NPC is in.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentPhase {
    /// Moving toward the goal.
    Approach,
    /// Doing the thing (trading, searching, waiting).
    Execute,
    /// Heading back / leaving.
    Depart,
}

const BUNKER_POS: Vec2 = Vec2::ZERO;
const COUNTER_POS: Vec2 = Vec2::new(8.0, 0.0);
const WALK_SPEED: f32 = 10.0;
const LEAVE_SPEED: f32 = 12.0;
const EXIT_DIST: f32 = 100.0;

/// Pick an intent for a newly spawned NPC based on their attributes.
///
/// `area_positions` is a list of resolved (x, y) positions for roaming targets.
/// `recruitable` should be true only if the NPC's faction allows recruitment.
pub fn pick_intent(
    npc: &cordon_core::entity::npc::Npc,
    area_positions: &[Vec2],
    recruitable: bool,
) -> Intent {
    let roll = simple_hash(npc.id.value()) % 100;

    let pick_area = || -> Vec2 {
        if area_positions.is_empty() {
            Vec2::new(50.0, 50.0)
        } else {
            let idx = (simple_hash(npc.id.value().wrapping_add(7)) as usize) % area_positions.len();
            area_positions[idx]
        }
    };

    if roll < 5 {
        Intent::Visit
    } else if recruitable && roll < 8 {
        Intent::Recruit
    } else if roll < 30 {
        Intent::Patrol {
            target: pick_area(),
        }
    } else {
        Intent::Scavenge {
            target: pick_area(),
        }
    }
}

/// Deterministic hash for consistent intent selection.
fn simple_hash(v: u32) -> u32 {
    let mut x = v;
    x = x.wrapping_mul(2654435761);
    x ^= x >> 16;
    x
}

/// Drive NPC actions based on their current state.
pub fn drive_actions(time: Res<Time>, mut query: Query<(BehaviorMut<Action>, &mut Transform)>) {
    let dt = time.delta_secs();

    for (mut behavior, mut transform) in &mut query {
        let pos = transform.translation.truncate();

        match behavior.current_mut() {
            Action::Idle { timer } => {
                *timer -= dt;
                let phase = time.elapsed_secs() * 0.3 + pos.x * 0.1;
                let wander = Vec2::new(phase.sin() * 2.0, (phase * 0.7).cos() * 2.0);
                transform.translation.x += wander.x * dt;
                transform.translation.y += wander.y * dt;
            }
            Action::Walk { target, speed } => {
                let dir = (*target - pos).normalize_or_zero();
                transform.translation.x += dir.x * *speed * dt;
                transform.translation.y += dir.y * *speed * dt;
            }
            Action::Follow { .. } => {}
            Action::Trade { timer } => {
                *timer -= dt;
            }
            Action::Flee { target } => {
                let dir = (*target - pos).normalize_or_zero();
                let speed = 20.0;
                transform.translation.x += dir.x * speed * dt;
                transform.translation.y += dir.y * speed * dt;
            }
        }
    }
}

/// Transition NPCs between actions based on their intent and phase.
pub fn drive_intents(
    mut query: Query<(BehaviorMut<Action>, &Intent, &mut IntentPhase, &Transform)>,
) {
    for (mut behavior, intent, mut phase, transform) in &mut query {
        let pos = transform.translation.truncate();

        match (*phase, behavior.current()) {
            // Idle timer expired → decide next action based on intent + phase
            (IntentPhase::Approach, Action::Idle { timer }) if *timer <= 0.0 => {
                let target = approach_target(intent);
                let _ = behavior.try_start(Action::Walk {
                    target,
                    speed: WALK_SPEED,
                });
            }

            // Arrived at approach target → execute
            (IntentPhase::Approach, Action::Walk { target, .. }) if pos.distance(*target) < 2.0 => {
                *phase = IntentPhase::Execute;
                match intent {
                    Intent::Visit => {
                        let _ = behavior.try_start(Action::Trade { timer: 8.0 });
                    }
                    Intent::Recruit => {
                        let _ = behavior.try_start(Action::Idle { timer: 10.0 });
                    }
                    Intent::Patrol { .. } => {
                        let _ = behavior.try_start(Action::Idle { timer: 5.0 });
                    }
                    Intent::Leave => {
                        // Already at exit, nothing to execute
                        *phase = IntentPhase::Depart;
                    }
                    _ => {
                        let _ = behavior.try_start(Action::Idle { timer: 5.0 });
                    }
                }
            }

            // Trade timer expired → depart
            (IntentPhase::Execute, Action::Trade { timer }) if *timer <= 0.0 => {
                *phase = IntentPhase::Depart;
                let exit = depart_target(pos);
                let _ = behavior.try_start(Action::Walk {
                    target: exit,
                    speed: LEAVE_SPEED,
                });
            }

            // Execute idle expired → depart
            (IntentPhase::Execute, Action::Idle { timer }) if *timer <= 0.0 => {
                *phase = IntentPhase::Depart;
                let exit = depart_target(pos);
                let _ = behavior.try_start(Action::Walk {
                    target: exit,
                    speed: LEAVE_SPEED,
                });
            }

            // Departing and reached exit → idle forever (will be despawned later)
            (IntentPhase::Depart, Action::Walk { target, .. }) if pos.distance(*target) < 2.0 => {
                let _ = behavior.try_start(Action::Idle { timer: f32::MAX });
            }

            _ => {}
        }
    }
}

fn approach_target(intent: &Intent) -> Vec2 {
    match intent {
        Intent::Visit | Intent::Recruit => COUNTER_POS,
        Intent::Scavenge { target } | Intent::Patrol { target } => *target,
        Intent::Leave => Vec2::X * EXIT_DIST,
        _ => BUNKER_POS,
    }
}

fn depart_target(pos: Vec2) -> Vec2 {
    let dir = if pos.length_squared() > 0.01 {
        pos.normalize()
    } else {
        Vec2::X
    };
    dir * EXIT_DIST
}
