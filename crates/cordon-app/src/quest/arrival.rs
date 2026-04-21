//! Handle [`BunkerArrival`] messages from traveling template NPCs.
//!
//! When a template NPC reaches the bunker, the cordon-sim layer
//! fires a [`BunkerArrival`]. This module converts that arrival
//! into a bunker [`Visitor`] pushed onto the [`VisitorQueue`]
//! and strips the travel markers + squad link so the map dot
//! stops rendering. The entity itself is kept alive so
//! `NpcAlive` conditions keep holding during the conversation
//! and `GiveNpcXp` consequences still resolve.
//!
//! [`handle_home_arrival`] closes the loop: once the NPC has
//! finished dialogue and walked back to its `SpawnOrigin`, the
//! return-trip squad is despawned and replaced with a fresh
//! `Goal::Idle` squad so the NPC stays alive at their home
//! settlement and can be re-dispatched to the bunker later.

use bevy::prelude::*;
use cordon_core::entity::npc::Npc;
use cordon_core::entity::squad::{Formation, Goal, Squad};
use cordon_data::gamedata::GameDataResource;
use cordon_sim::plugin::prelude::{
    FactionId, PendingDeliveryItems, PendingYarnNode, SquadBundle, SquadMembership,
};
use cordon_sim::quest::travel::{BunkerArrival, HomeArrival};
use cordon_sim::resources::{SquadIdIndex, UidAllocator};

use crate::bunker::{Visitor, VisitorQueue};
use crate::locale::L10n;

pub fn handle_bunker_arrival(
    mut arrivals: MessageReader<BunkerArrival>,
    data: Res<GameDataResource>,
    l10n: L10n,
    mut queue: ResMut<VisitorQueue>,
    pending_q: Query<(&PendingYarnNode, Option<&PendingDeliveryItems>)>,
    mut commands: Commands,
) {
    for arrival in arrivals.read() {
        let Some(template) = data.0.npc_template(&arrival.template) else {
            warn!(
                "BunkerArrival: unknown template `{}`",
                arrival.template.as_str()
            );
            continue;
        };
        let display_name = l10n.get(&template.name_key());
        // PendingYarnNode is required: without it the visitor
        // would admit to an empty-named yarn node, which panics the
        // runner. Skip enqueue if missing — log loudly so the
        // missing-payload case surfaces in authoring.
        let (yarn_node, delivery_items) = match pending_q.get(arrival.entity) {
            Ok((p, delivery)) => (
                p.0.clone(),
                delivery.map(|d| d.0.clone()).unwrap_or_default(),
            ),
            Err(_) => {
                error!(
                    "BunkerArrival: template `{}` has no PendingYarnNode; \
                     refusing to enqueue visitor (would crash dialogue)",
                    arrival.template.as_str()
                );
                // Still strip the squad membership so the map dot
                // stops rendering — the entity otherwise loiters
                // at the bunker as a dot with no purpose.
                commands.entity(arrival.entity).remove::<SquadMembership>();
                continue;
            }
        };
        queue.0.push_back(Visitor {
            display_name: display_name.clone(),
            faction: template.faction.clone(),
            yarn_node,
            template: Some(arrival.template.clone()),
            delivery_items,
        });
        info!("{display_name} has arrived at the bunker");
        commands
            .entity(arrival.entity)
            .remove::<SquadMembership>()
            .remove::<PendingYarnNode>()
            .remove::<PendingDeliveryItems>();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_home_arrival(
    mut arrivals: MessageReader<HomeArrival>,
    data: Res<GameDataResource>,
    l10n: L10n,
    mut uids: ResMut<UidAllocator>,
    mut squad_index: ResMut<SquadIdIndex>,
    entity_q: Query<(&Transform, &FactionId, &SquadMembership)>,
    mut commands: Commands,
) {
    for arrival in arrivals.read() {
        let name = data
            .0
            .npc_template(&arrival.template)
            .map(|def| l10n.get(&def.name_key()))
            .unwrap_or_else(|| arrival.template.as_str().to_string());

        let Ok((transform, faction, membership)) = entity_q.get(arrival.entity) else {
            warn!(
                "HomeArrival: entity {:?} for template `{}` missing Transform/FactionId/SquadMembership",
                arrival.entity,
                arrival.template.as_str()
            );
            continue;
        };
        let pos = transform.translation.truncate();
        let old_squad = membership.squad;

        commands.entity(old_squad).despawn();

        let squad_uid = uids.alloc::<Squad>();
        let squad = Squad {
            id: squad_uid,
            faction: faction.0.clone(),
            members: vec![uids.alloc::<Npc>()],
            leader: uids.alloc::<Npc>(),
            goal: Goal::Idle,
            formation: Formation::Column,
            facing: [0.0, 1.0],
            waypoints: Vec::new(),
            next_waypoint: 0,
        };
        let squad_bundle =
            SquadBundle::from_squad(squad, arrival.entity, vec![arrival.entity], pos);
        // MovementIntent defaults to None — an idle squad holds at
        // the leader's pos. The tree (idle_tree) will manage the
        // 4-second hold cycle going forward.
        let squad_entity = commands.spawn(squad_bundle).id();
        squad_index.0.insert(squad_uid, squad_entity);

        // TravelingHome was removed by detect_home_arrival to
        // prevent multi-fire. Here we just install the idle squad
        // membership.
        commands.entity(arrival.entity).insert(SquadMembership {
            squad: squad_entity,
            slot: 0,
        });

        info!("{name} is idle at home.");
    }
}
