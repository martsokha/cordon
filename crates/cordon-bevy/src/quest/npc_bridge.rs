//! Template NPC spawn and XP bridge.
//!
//! Consumes [`SpawnNpcRequest`] and [`GiveNpcXpRequest`] messages
//! emitted by the consequence applier in cordon-sim and turns them
//! into live ECS entities (or mutates existing ones). Also watches
//! [`NpcDied`] to update the [`TemplateRegistry`] when a
//! template-spawned NPC dies.

use bevy::prelude::*;
use cordon_core::entity::name::{NameFormat, NpcName};
use cordon_core::entity::npc::Npc;
use cordon_core::item::{ItemInstance, Loadout};
use cordon_core::primitive::{Experience, Health, Loyalty, Pool};
use cordon_data::gamedata::GameDataResource;
use cordon_sim::components::{
    ActiveEffects, BaseMaxes, CorruptionPool, Employment, FactionId, NpcAttributes, NpcBundle,
    NpcMarker, Perks, StaminaPool, TemplateId,
};
use cordon_sim::death::NpcDied;
use cordon_sim::quest::consequence::{GiveNpcXpRequest, SpawnNpcRequest};
use cordon_sim::quest::registry::TemplateRegistry;
use cordon_sim::resources::UidAllocator;
use cordon_sim::spawn::loadout::generate_loadout;
use rand::RngExt;

/// Consume [`SpawnNpcRequest`] messages, spawning a template NPC
/// entity for each one and registering it in the
/// [`TemplateRegistry`].
pub fn handle_spawn_npc_requests(
    mut commands: Commands,
    mut requests: MessageReader<SpawnNpcRequest>,
    data: Res<GameDataResource>,
    mut registry: ResMut<TemplateRegistry>,
    mut uids: ResMut<UidAllocator>,
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

        if def.unique && registry.is_alive(&req.template) {
            continue;
        }
        if !def.respawnable && registry.is_permanently_dead(&req.template) {
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

        let entity = commands.spawn((bundle, TemplateId(req.template.clone()))).id();
        registry.register(req.template.clone(), entity);

        info!(
            "SpawnNpcRequest: spawned template `{}` as entity {:?}",
            req.template.as_str(),
            entity
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
