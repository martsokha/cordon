//! NPC behavior: Action (state machine) + Intent (goal).

use bevy::prelude::*;
use cordon_core::entity::npc::Npc;
use cordon_core::primitive::id::Id;
use cordon_core::primitive::uid::Uid;
use cordon_core::world::area::Area;
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
    /// At the counter, engaged in trade.
    Trade,
    /// Running from danger.
    Flee { target: Vec2 },
}

impl Behavior for Action {
    fn filter_next(&self, next: &Self) -> bool {
        use Action::*;
        match_next! {
            self => next,
            Idle { .. } => Walk { .. } | Trade | Follow { .. } | Flee { .. },
            Walk { .. } => Idle { .. } | Trade | Follow { .. } | Flee { .. },
            Follow { .. } => Idle { .. } | Walk { .. } | Flee { .. },
            Trade => Idle { .. } | Walk { .. } | Flee { .. },
            Flee { .. } => Idle { .. } | Walk { .. }
        }
    }
}

/// Why the NPC is doing what they're doing.
#[derive(Component, Debug, Clone)]
pub enum Intent {
    /// Came to trade at the bunker.
    Visit,
    /// Heading to an area to scavenge loot.
    Scavenge(Id<Area>),
    /// Faction patrol route.
    Patrol(Id<Area>),
    /// Pursuing a quest objective.
    Quest(Id<Quest>),
    /// Escorting another NPC.
    Escort(Uid<Npc>),
    /// Looking for work at the bunker.
    Recruit,
    /// Done, heading out of the map.
    Leave,
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
            Action::Follow { .. } => {
                // TODO: look up target transform, move toward it
            }
            Action::Trade => {}
            Action::Flee { target } => {
                let dir = (*target - pos).normalize_or_zero();
                let speed = 20.0;
                transform.translation.x += dir.x * speed * dt;
                transform.translation.y += dir.y * speed * dt;
            }
        }
    }
}

/// Transition NPCs between actions based on their intent.
pub fn drive_intents(mut query: Query<(BehaviorMut<Action>, &Intent, &Transform)>) {
    for (mut behavior, intent, transform) in &mut query {
        let pos = transform.translation.truncate();

        match behavior.current() {
            Action::Idle { timer } if *timer <= 0.0 => match intent {
                Intent::Visit => {
                    let counter = Vec2::new(8.0, 0.0);
                    let _ = behavior.try_start(Action::Walk {
                        target: counter,
                        speed: 10.0,
                    });
                }
                Intent::Leave => {
                    let exit = pos.normalize_or_zero() * 100.0;
                    let _ = behavior.try_start(Action::Walk {
                        target: exit,
                        speed: 12.0,
                    });
                }
                _ => {}
            },
            Action::Walk { target, .. } if pos.distance(*target) < 2.0 => match intent {
                Intent::Visit => {
                    let _ = behavior.try_start(Action::Trade);
                }
                _ => {
                    let _ = behavior.try_start(Action::Idle { timer: 2.0 });
                }
            },
            _ => {}
        }
    }
}
