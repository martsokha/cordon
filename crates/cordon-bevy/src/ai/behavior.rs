//! NPC visitor behavior using moonshine_behavior.

use bevy::prelude::*;
use moonshine_behavior::prelude::*;

/// Visitor NPC behavior states.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub enum VisitorBehavior {
    /// Walking toward the bunker from spawn point.
    Arrive { target: Vec2 },
    /// Milling around near the bunker.
    Idle { timer: f32 },
    /// Walking to the trading counter.
    Approach { target: Vec2 },
    /// At the counter, ready to trade.
    Trade,
    /// Walking away, will despawn at edge.
    Leave { target: Vec2 },
}

impl Behavior for VisitorBehavior {
    fn filter_next(&self, next: &Self) -> bool {
        use VisitorBehavior::*;
        match_next! {
            self => next,
            Arrive { .. } => Idle { .. },
            Idle { .. } => Approach { .. } | Leave { .. },
            Approach { .. } => Trade,
            Trade => Leave { .. }
        }
    }
}

/// Move NPC dots toward their behavior target.
pub fn drive_visitor_behavior(
    time: Res<Time>,
    mut query: Query<(BehaviorMut<VisitorBehavior>, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (mut behavior, mut transform) in &mut query {
        let pos = transform.translation.truncate();

        match behavior.current_mut() {
            VisitorBehavior::Arrive { target } => {
                let dir = (*target - pos).normalize_or_zero();
                let speed = 15.0;
                transform.translation.x += dir.x * speed * dt;
                transform.translation.y += dir.y * speed * dt;

                if pos.distance(*target) < 2.0 {
                    let idle_time = 3.0 + (pos.x * 100.0).sin().abs() * 4.0;
                    let _ = behavior.try_start(VisitorBehavior::Idle { timer: idle_time });
                }
            }
            VisitorBehavior::Idle { timer } => {
                *timer -= dt;
                let wander = Vec2::new(
                    (time.elapsed_secs() * 0.3 + pos.x * 0.1).sin() * 2.0,
                    (time.elapsed_secs() * 0.25 + pos.y * 0.1).cos() * 2.0,
                );
                transform.translation.x += wander.x * dt;
                transform.translation.y += wander.y * dt;

                if *timer <= 0.0 {
                    let counter = Vec2::new(8.0, 0.0);
                    let _ = behavior.try_start(VisitorBehavior::Approach { target: counter });
                }
            }
            VisitorBehavior::Approach { target } => {
                let dir = (*target - pos).normalize_or_zero();
                let speed = 10.0;
                transform.translation.x += dir.x * speed * dt;
                transform.translation.y += dir.y * speed * dt;

                if pos.distance(*target) < 2.0 {
                    let _ = behavior.try_start(VisitorBehavior::Trade);
                }
            }
            VisitorBehavior::Trade => {
                // Stays here until player interacts or timeout
                // For now, trade for a few seconds then leave
            }
            VisitorBehavior::Leave { target } => {
                let dir = (*target - pos).normalize_or_zero();
                let speed = 12.0;
                transform.translation.x += dir.x * speed * dt;
                transform.translation.y += dir.y * speed * dt;
            }
        }
    }
}
