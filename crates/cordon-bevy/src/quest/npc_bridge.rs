//! Template NPC spawn and XP bridge.
//!
//! Consumes [`SpawnNpcRequest`] and [`GiveNpcXpRequest`] messages
//! emitted by the consequence applier in cordon-sim and turns them
//! into live ECS entities (or mutates existing ones). Also watches
//! [`NpcDied`] to update the [`TemplateRegistry`] when a
//! template-spawned NPC dies.

use bevy::prelude::*;
use bevy_fluent::prelude::Localization;
use cordon_core::entity::name::{NameFormat, NpcName};
use cordon_core::entity::npc::Npc;
use cordon_core::entity::squad::{Formation, Goal, Squad};
use cordon_core::item::{ItemInstance, Loadout};
use cordon_core::primitive::{Experience, Health, Loyalty, Pool};
use cordon_core::world::BUNKER_MAP_POS;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::plugin::prelude::{
    ActiveEffects, BaseMaxes, CorruptionPool, Employment, FactionId, MovementIntent, NpcAttributes,
    NpcBundle, NpcDied, NpcMarker, PendingYarnNode, Perks, QuestCritical, SpawnOrigin, SquadBundle,
    SquadMembership, StaminaPool, TemplateId, TravelingHome, TravelingToBunker,
};

use crate::locale::l10n_or;
use cordon_sim::quest::consequence::{DismissTemplateNpc, GiveNpcXpRequest, SpawnNpcRequest};
use cordon_sim::quest::registry::TemplateRegistry;
use cordon_sim::resources::{FactionSettlements, SquadIdIndex, UidAllocator};
use cordon_sim::spawn::loadout::generate_loadout;
use rand::RngExt;

/// Consume [`SpawnNpcRequest`] messages, spawning a template NPC
/// entity for each one and registering it in the
/// [`TemplateRegistry`].
#[allow(clippy::too_many_arguments)]
pub fn handle_spawn_npc_requests(
    mut commands: Commands,
    mut requests: MessageReader<SpawnNpcRequest>,
    data: Res<GameDataResource>,
    settlements: Res<FactionSettlements>,
    localization: Option<Res<Localization>>,
    mut registry: ResMut<TemplateRegistry>,
    mut uids: ResMut<UidAllocator>,
    mut squad_index: ResMut<SquadIdIndex>,
    entity_q: Query<(&Transform, Option<&SquadMembership>)>,
) {
    let mut rng = rand::rng();
    let catalog = &data.0;
    for req in requests.read() {
        let Some(def) = catalog.npc_template(&req.template) else {
            warn!(
                "SpawnNpcRequest: unknown template `{}`",
                req.template.as_str()
            );
            continue;
        };

        if !def.respawnable && registry.is_permanently_dead(&req.template) {
            continue;
        }

        if registry.is_alive(&req.template) {
            let Some(entity) = registry.entity(&req.template) else {
                continue;
            };
            let Ok((transform, membership)) = entity_q.get(entity) else {
                warn!(
                    "SpawnNpcRequest: alive template `{}` entity {:?} missing Transform",
                    req.template.as_str(),
                    entity,
                );
                continue;
            };
            // Start the new travel squad at the NPC's *current*
            // position so the renderer doesn't teleport them. The
            // travel target is baked into the Move activity below.
            let current_pos = transform.translation.truncate();
            if let Some(m) = membership {
                commands.entity(m.squad).despawn();
            }

            let squad_uid = uids.alloc::<Squad>();
            let squad = Squad {
                id: squad_uid,
                faction: def.faction.clone(),
                members: vec![uids.alloc::<Npc>()],
                leader: uids.alloc::<Npc>(),
                goal: Goal::GoTo {
                    target: [BUNKER_MAP_POS.x, BUNKER_MAP_POS.y],
                    intent: cordon_core::entity::squad::TravelIntent::Arriving,
                },
                formation: Formation::Column,
                facing: [0.0, 1.0],
                waypoints: Vec::new(),
                next_waypoint: 0,
            };
            let mut squad_bundle =
                SquadBundle::from_squad(squad, entity, vec![entity], current_pos);
            // Pre-seed the movement intent so formation pulls the
            // dot toward the bunker on the very first tick instead
            // of waiting for the BT to fire.
            squad_bundle.movement = MovementIntent(Some(BUNKER_MAP_POS));
            let squad_entity = commands.spawn(squad_bundle).id();
            squad_index.0.insert(squad_uid, squad_entity);

            let mut entity_cmds = commands.entity(entity);
            entity_cmds
                .remove::<TravelingHome>()
                .insert(TravelingToBunker)
                .insert(QuestCritical)
                .insert(SquadMembership {
                    squad: squad_entity,
                    slot: 0,
                });
            if let Some(yarn) = req.yarn_node.clone() {
                entity_cmds.insert(PendingYarnNode(yarn));
            }

            let name = match localization.as_deref() {
                Some(l10n) => l10n_or(l10n, &def.name_key, &def.name_key),
                None => def.name_key.clone(),
            };
            info!("{name} is heading back to the bunker.");
            continue;
        }

        let rank = def.rank;
        let (lo, hi) = rank.xp_range();
        let xp = Experience::new(rng.random_range(lo..=hi));

        let health: Pool<Health> = Pool::full();
        let hp_max = health.max();

        let loadout = if let Some(ref item_ids) = def.loadout {
            let mut lo = Loadout::new();
            for item_id in item_ids {
                let Some(item_def) = catalog.item(item_id) else {
                    warn!(
                        "SpawnNpcRequest: template `{}` references unknown item `{}`",
                        req.template.as_str(),
                        item_id.as_str()
                    );
                    continue;
                };
                lo.general.push(ItemInstance::new(item_def));
            }
            lo
        } else if let Some(archetype) = catalog.archetype_for_faction(&def.faction) {
            generate_loadout(archetype, rank, &catalog.items, &mut rng)
        } else {
            Loadout::new()
        };

        let npc_uid = uids.alloc::<Npc>();
        let bundle = NpcBundle {
            marker: NpcMarker,
            id: npc_uid,
            name: NpcName {
                format: NameFormat::Alias,
                first: def.name_key.clone(),
                second: None,
            },
            faction: FactionId(def.faction.clone()),
            xp,
            hp: health,
            stamina: StaminaPool::full(),
            corruption: CorruptionPool::empty(),
            active_effects: ActiveEffects::default(),
            base_maxes: BaseMaxes {
                hp: hp_max,
                stamina: 100,
            },
            loadout,
            wealth: cordon_core::primitive::Credits::new(0),
            attributes: NpcAttributes {
                trust: def.trust,
                loyalty: Loyalty(0.5),
                personality: def.personality,
            },
            perks: Perks {
                all: def.perks.clone(),
                revealed: Vec::new(),
            },
            employment: Employment {
                role: None,
                daily_pay: cordon_core::primitive::Credits::new(0),
            },
        };

        // Pick a random faction settlement as the spawn point.
        // Without a home to travel from, a template NPC with a
        // yarn payload would be stranded at the bunker already —
        // skip and warn so authoring surfaces the issue.
        let spawn_pos = match settlements.0.get(&def.faction) {
            Some(centres) if !centres.is_empty() => {
                let idx = rng.random_range(0..centres.len());
                let base = centres[idx];
                let jx = rng.random_range(-30.0_f32..30.0);
                let jy = rng.random_range(-30.0_f32..30.0);
                base + bevy::math::Vec2::new(jx, jy)
            }
            _ => {
                warn!(
                    "SpawnNpcRequest: faction `{}` has no settlements, cannot place template `{}`",
                    def.faction.as_str(),
                    req.template.as_str()
                );
                continue;
            }
        };

        // Map-layer z matches `attach_npc_visuals` (dots live at
        // z=10 above the cloud layer at z=5).
        let transform = Transform::from_xyz(spawn_pos.x, spawn_pos.y, 10.0);
        let mut entity_cmds = commands.spawn((
            bundle,
            TemplateId(req.template.clone()),
            TravelingToBunker,
            QuestCritical,
            SpawnOrigin(spawn_pos),
            transform,
        ));
        if let Some(yarn) = req.yarn_node.clone() {
            entity_cmds.insert(PendingYarnNode(yarn));
        }
        let entity = entity_cmds.id();
        registry.register(req.template.clone(), entity);

        // Spawn a 1-member squad so the map renderer and squad
        // systems treat the traveling NPC like any other member.
        // The squad's goal is `GoTo { bunker, Arriving }` — goals.rs
        // turns that into a Move activity, which formation.rs
        // flips to Hold on arrival.
        let squad_uid = uids.alloc::<Squad>();
        let squad = Squad {
            id: squad_uid,
            faction: def.faction.clone(),
            members: vec![uids.alloc::<Npc>()],
            leader: uids.alloc::<Npc>(),
            goal: Goal::GoTo {
                target: [BUNKER_MAP_POS.x, BUNKER_MAP_POS.y],
                intent: cordon_core::entity::squad::TravelIntent::Arriving,
            },
            formation: Formation::Column,
            facing: [0.0, 1.0],
            waypoints: Vec::new(),
            next_waypoint: 0,
        };
        // The Uid<Npc> values inside `squad` are placeholders —
        // the runtime squad is driven through Entity handles via
        // `SquadLeader` / `SquadMembers`, not uids. Pass the real
        // entity as leader + sole member.
        let mut squad_bundle = SquadBundle::from_squad(squad, entity, vec![entity], spawn_pos);
        // Start moving immediately so the dot peels off toward the
        // bunker on spawn — the behavior tree will write the same
        // intent on its first tick but this avoids a one-frame
        // flicker.
        squad_bundle.movement = MovementIntent(Some(BUNKER_MAP_POS));
        let squad_entity = commands.spawn(squad_bundle).id();
        squad_index.0.insert(squad_uid, squad_entity);

        commands.entity(entity).insert(SquadMembership {
            squad: squad_entity,
            slot: 0,
        });

        info!(
            "SpawnNpcRequest: spawned template `{}` as entity {:?} at {:?}; traveling to bunker",
            req.template.as_str(),
            entity,
            spawn_pos,
        );
    }
}

/// Consume [`GiveNpcXpRequest`] messages, adding XP to the
/// template NPC's [`Experience`] component.
pub fn handle_give_npc_xp_requests(
    mut requests: MessageReader<GiveNpcXpRequest>,
    registry: Res<TemplateRegistry>,
    mut xp_q: Query<&mut Experience>,
) {
    for req in requests.read() {
        let Some(entity) = registry.entity(&req.template) else {
            warn!(
                "GiveNpcXpRequest: template `{}` has no live entity",
                req.template.as_str()
            );
            continue;
        };
        let Ok(mut xp) = xp_q.get_mut(entity) else {
            warn!(
                "GiveNpcXpRequest: entity {:?} for template `{}` has no Experience component",
                entity,
                req.template.as_str()
            );
            continue;
        };
        xp.add(req.amount.value());
    }
}

/// Consume [`DismissTemplateNpc`] messages by starting the NPC's
/// return-travel leg: strip `QuestCritical`, attach
/// `TravelingHome`, and build a fresh 1-member squad moving toward
/// the stored `SpawnOrigin`. The map dot reappears and the NPC
/// walks home; `detect_home_arrival` fires `HomeArrival` when
/// they get close enough.
#[allow(clippy::too_many_arguments)]
pub fn handle_template_dismissal(
    mut commands: Commands,
    mut requests: MessageReader<DismissTemplateNpc>,
    data: Res<GameDataResource>,
    localization: Option<Res<Localization>>,
    dismissed_q: Query<(&Transform, &SpawnOrigin, &FactionId)>,
    mut uids: ResMut<UidAllocator>,
    mut squad_index: ResMut<SquadIdIndex>,
) {
    for req in requests.read() {
        let Ok((transform, origin, faction)) = dismissed_q.get(req.entity) else {
            warn!(
                "DismissTemplateNpc: entity {:?} for template `{}` missing Transform/SpawnOrigin/FactionId",
                req.entity,
                req.template.as_str()
            );
            continue;
        };
        // The NPC is at the bunker (dialogue just ended). Start
        // the return-home squad at their *current* Transform so
        // the renderer doesn't teleport them to the origin.
        let current_pos = transform.translation.truncate();
        let origin_pos = origin.0;

        let squad_uid = uids.alloc::<Squad>();
        let squad = Squad {
            id: squad_uid,
            faction: faction.0.clone(),
            members: vec![uids.alloc::<Npc>()],
            leader: uids.alloc::<Npc>(),
            goal: Goal::GoTo {
                target: [origin_pos.x, origin_pos.y],
                intent: cordon_core::entity::squad::TravelIntent::Returning,
            },
            formation: Formation::Column,
            facing: [0.0, 1.0],
            waypoints: Vec::new(),
            next_waypoint: 0,
        };
        let mut squad_bundle =
            SquadBundle::from_squad(squad, req.entity, vec![req.entity], current_pos);
        squad_bundle.movement = MovementIntent(Some(origin_pos));
        let squad_entity = commands.spawn(squad_bundle).id();
        squad_index.0.insert(squad_uid, squad_entity);

        commands
            .entity(req.entity)
            .remove::<QuestCritical>()
            .insert(TravelingHome)
            .insert(SquadMembership {
                squad: squad_entity,
                slot: 0,
            });

        let name = data
            .0
            .npc_template(&req.template)
            .map(|def| match localization.as_deref() {
                Some(l10n) => l10n_or(l10n, &def.name_key, &def.name_key),
                None => def.name_key.clone(),
            })
            .unwrap_or_else(|| req.template.as_str().to_string());
        info!("{name} is heading home.");
    }
}

/// Watch for [`NpcDied`] and update the [`TemplateRegistry`]
/// when a template-spawned NPC dies.
pub fn handle_template_npc_deaths(
    mut deaths: MessageReader<NpcDied>,
    data: Res<GameDataResource>,
    mut registry: ResMut<TemplateRegistry>,
    template_q: Query<&TemplateId>,
) {
    let catalog = &data.0;
    for ev in deaths.read() {
        let Ok(tid) = template_q.get(ev.entity) else {
            continue;
        };
        let permanent = catalog
            .npc_template(&tid.0)
            .is_some_and(|def| !def.respawnable);
        registry.mark_dead(&tid.0, permanent);
        info!(
            "Template NPC `{}` died (permanent={})",
            tid.0.as_str(),
            permanent
        );
    }
}
