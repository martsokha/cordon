//! The read-only game database and its load-time integrity checks.
//!
//! [`GameData`] holds every definition the simulation reads from disk:
//! items, factions, areas, upgrades, events, quests, triggers,
//! name pools, archetypes, loot tables. All lookups go through typed
//! ID aliases from [`cordon_core::primitive`].
//!
//! [`GameData::validate`] runs once on the assembled catalog and
//! warns on every dangling cross-reference, duplicate stage choice,
//! empty table, and stub-consequence usage that the type system can't
//! catch. Every check is soft — a dangling reference is a `warn!` not
//! an error, because a partially-broken catalog is better than a
//! crashed game on boot.
//!
//! # What the validator covers
//!
//! - **Faction refs**: items (`suppliers[].faction`), factions
//!   themselves (`relations[].faction`, `namepool`), areas
//!   (`AreaKind::Settlement.faction`), archetypes (`faction`),
//!   events (`involved_factions[]`), quests (`giver_faction`),
//!   trigger / condition (`FactionStanding.faction`), consequence
//!   (`StandingChange.faction`, `TriggerEvent.involved_factions[]`).
//!
//! - **Item refs**: archetype loadout pools, consequences
//!   (`GiveItem`/`TakeItem`), conditions (`HaveItem`).
//!
//! - **Area refs**: events (`target_areas[]`), consequences
//!   (`TriggerEvent.target_area`, `DangerModifier.area`,
//!   `SpawnNpc.at`), conditions (`NpcAtLocation.area`).
//!
//! - **Quest refs**: triggers (`quest`), consequences (`StartQuest`),
//!   conditions (`QuestActive`, `QuestCompleted`, `QuestFlag.quest`).
//!
//! - **Event refs**: triggers (`OnEvent`), consequences
//!   (`TriggerEvent.event`), conditions (`EventActive`).
//!
//! - **Upgrade refs**: consequences (`UnlockUpgrade`), conditions
//!   (`HaveUpgrade`).
//!
//! - **Caliber refs**: weapons (`WeaponData.caliber`), verified
//!   against the set of calibers advertised by ammo items.
//!
//! - **Intra-quest stage refs**: `Talk.fallback`,
//!   `Talk.branches[].next_stage`, `Objective.on_success`,
//!   `Objective.on_failure`, `Branch.arms[].next_stage`,
//!   `Branch.fallback`. Plus duplicate `TalkBranch::choice` detection.
//!
//! - **Quest shape**: empty-stage-list warnings.
//!
//! - **NamePool refs**: factions' `namepool` → name_pools table.
//!

//! - **NPC template refs**: templates (`faction`, `loadout[]`),
//!   consequences (`SpawnNpc.template`,
//!   `GiveNpcXp.template`), conditions (`NpcAlive`, `NpcDead`,
//!   `NpcAtLocation.npc`).

use std::collections::{HashMap, HashSet};

use bevy::log::warn;
use cordon_core::entity::archetype::{Archetype, ArchetypeDef, RankLoadout};
use cordon_core::entity::bunker::{Upgrade, UpgradeDef};
use cordon_core::entity::faction::{Faction, FactionDef};
use cordon_core::entity::name::{NamePool, NamePoolMarker};
use cordon_core::entity::npc::{NpcTemplate, NpcTemplateDef};
use cordon_core::item::{Caliber, Item, ItemData, ItemDef};
use cordon_core::primitive::{Id, IdMarker};
use cordon_core::world::area::{Area, AreaDef, AreaKind};
use cordon_core::world::loot::LootTables;
use cordon_core::world::narrative::{
    ConditionalConsequence, Consequence, Event, EventDef, Intel, IntelDef, ObjectiveCondition,
    Quest, QuestDef, QuestStageKind, QuestTrigger, QuestTriggerDef, QuestTriggerKind,
};

/// The read-only game database.
///
/// Loaded once at startup from JSON config files. Contains all static
/// definitions that the simulation references: items, factions, areas,
/// upgrades, and loot tables.
///
/// Calibers are implicit — they exist because ammo and weapon items
/// reference the same caliber ID string. No separate caliber registry.
/// Player ranks are hardcoded in [`PlayerRank`](cordon_core::entity::player::PlayerRank).
///
/// All lookups are by typed ID aliases from [`cordon_core::primitive`].
#[derive(Default)]
pub struct GameData {
    /// Item definitions keyed by item ID.
    pub items: HashMap<Id<Item>, ItemDef>,
    /// Faction definitions keyed by faction ID.
    pub factions: HashMap<Id<Faction>, FactionDef>,
    /// Area definitions keyed by area ID.
    pub areas: HashMap<Id<Area>, AreaDef>,
    /// Upgrade definitions keyed by upgrade ID.
    pub upgrades: HashMap<Id<Upgrade>, UpgradeDef>,
    /// Event definitions keyed by event ID.
    pub events: HashMap<Id<Event>, EventDef>,
    /// Intel definitions keyed by intel ID.
    pub intel: HashMap<Id<Intel>, IntelDef>,
    /// Quest definitions keyed by quest ID.
    pub quests: HashMap<Id<Quest>, QuestDef>,
    /// Quest trigger rules keyed by trigger ID. Each trigger
    /// references the quest it starts via [`QuestTriggerDef::quest`].
    pub triggers: HashMap<Id<QuestTrigger>, QuestTriggerDef>,
    /// Name pools keyed by pool ID.
    pub name_pools: HashMap<Id<NamePoolMarker>, NamePool>,
    /// Loot tables keyed by area ID.
    pub loot_tables: LootTables,
    /// NPC loadout archetypes keyed by archetype ID. The
    /// `faction` field on each def is the real cross-reference
    /// to the factions table; use [`archetype_for_faction`](Self::archetype_for_faction)
    /// instead of key-indexing by faction id.
    pub archetypes: HashMap<Id<Archetype>, ArchetypeDef>,
    /// Named NPC templates keyed by template ID. Each entry defines
    /// a unique, story-relevant character that quests and events can
    /// reference by stable ID.
    pub npc_templates: HashMap<Id<NpcTemplate>, NpcTemplateDef>,
}

impl GameData {
    /// Look up an item definition by ID.
    pub fn item(&self, id: &Id<Item>) -> Option<&ItemDef> {
        self.items.get(id)
    }

    /// Look up a faction definition by ID.
    pub fn faction(&self, id: &Id<Faction>) -> Option<&FactionDef> {
        self.factions.get(id)
    }

    /// Look up an area definition by ID.
    pub fn area(&self, id: &Id<Area>) -> Option<&AreaDef> {
        self.areas.get(id)
    }

    /// Look up an intel definition by ID.
    pub fn intel(&self, id: &Id<Intel>) -> Option<&IntelDef> {
        self.intel.get(id)
    }

    /// Look up an NPC template by ID.
    pub fn npc_template(&self, id: &Id<NpcTemplate>) -> Option<&NpcTemplateDef> {
        self.npc_templates.get(id)
    }

    /// Look up the loadout archetype for a faction.
    ///
    /// Walks `archetypes.values()` filtering on the
    /// [`ArchetypeDef::faction`] field rather than key-lookup by
    /// string. The HashMap key is an archetype ID
    /// (`archetype_garrison`), not a faction ID
    /// (`faction_garrison`), so a direct `.get` won't work — and
    /// even if we normalized the key, the `faction` field is the
    /// real cross-reference. Cost is O(N) where N ≤ 5.
    pub fn archetype_for_faction(&self, faction: &Id<Faction>) -> Option<&ArchetypeDef> {
        self.archetypes.values().find(|a| &a.faction == faction)
    }

    /// Get all faction IDs.
    pub fn faction_ids(&self) -> Vec<Id<Faction>> {
        self.factions.keys().cloned().collect()
    }

    /// Get all area IDs.
    pub fn area_ids(&self) -> Vec<Id<Area>> {
        self.areas.keys().cloned().collect()
    }

    /// Build a faction-to-namepool mapping for NPC generation.
    ///
    /// Resolves each faction's `namepool` ID to the actual [`NamePool`].
    /// Returns a map keyed by faction ID for use with the simulation layer.
    pub fn faction_name_pools(&self) -> HashMap<Id<Faction>, NamePool> {
        self.factions
            .iter()
            .filter_map(|(fid, fdef)| {
                self.name_pools
                    .get(&fdef.namepool)
                    .map(|pool| (fid.clone(), pool.clone()))
            })
            .collect()
    }

    /// Walk the assembled catalog and warn on every dangling
    /// cross-reference, duplicate key, or empty-record authoring
    /// mistake. Called from `assemble_game_data` right after the
    /// catalog is built and before it's inserted as a resource.
    pub fn validate(&self) {
        self.validate_items();
        self.validate_factions();
        self.validate_areas();
        self.validate_archetypes();
        self.validate_npc_templates();
        self.validate_weapons_vs_calibers();
        self.validate_events();
        self.validate_triggers();
        self.validate_quests();
        self.warn_on_stub_consequences();
    }

    fn validate_items(&self) {
        for (id, def) in &self.items {
            let referrer = format!("item `{}`", id.as_str());
            for sup in &def.suppliers {
                self.check_faction(&sup.faction, &referrer, "suppliers[].faction");
            }
        }
    }

    fn validate_factions(&self) {
        for (id, def) in &self.factions {
            let referrer = format!("faction `{}`", id.as_str());
            if !self.name_pools.contains_key(&def.namepool) {
                warn_missing::<NamePoolMarker>(
                    "namepool ref from",
                    &referrer,
                    "namepool",
                    &def.namepool,
                );
            }
            for (other, _) in &def.relations {
                if !self.factions.contains_key(other) {
                    warn_missing::<Faction>(
                        "faction ref from",
                        &referrer,
                        "relations[].faction",
                        other,
                    );
                }
            }
        }
    }

    fn validate_areas(&self) {
        for (id, def) in &self.areas {
            let referrer = format!("area `{}`", id.as_str());
            if let AreaKind::Settlement { faction, .. } = &def.kind {
                self.check_faction(faction, &referrer, "Settlement.faction");
            }
        }
    }

    fn validate_archetypes(&self) {
        for (id, def) in &self.archetypes {
            let referrer = format!("archetype `{}`", id.as_str());
            self.check_faction(&def.faction, &referrer, "faction");
            for loadout in def.ranks.values() {
                self.check_rank_loadout(loadout, &referrer);
            }
        }
    }

    fn check_rank_loadout(&self, loadout: &RankLoadout, referrer: &str) {
        for weighted in &loadout.primary {
            self.check_item(&weighted.id, referrer, "ranks.primary");
        }
        for weighted in &loadout.secondary {
            self.check_item(&weighted.id, referrer, "ranks.secondary");
        }
        for weighted in &loadout.armor {
            self.check_item(&weighted.id, referrer, "ranks.armor");
        }
        for weighted in &loadout.helmet {
            self.check_item(&weighted.id, referrer, "ranks.helmet");
        }
        for weighted in &loadout.consumables {
            self.check_item(&weighted.id, referrer, "ranks.consumables");
        }
    }

    fn validate_weapons_vs_calibers(&self) {
        // The caliber table is implicit: it's the set of
        // `caliber` fields advertised by ammo items.
        let calibers: HashSet<Id<Caliber>> = self
            .items
            .values()
            .filter_map(|item| match &item.data {
                ItemData::Ammo(a) => Some(a.caliber.clone()),
                _ => None,
            })
            .collect();
        for (id, def) in &self.items {
            let ItemData::Weapon(w) = &def.data else {
                continue;
            };
            if !calibers.contains(&w.caliber) {
                warn!(
                    "item `{}` weapon references caliber `{}` but no ammo item advertises it",
                    id.as_str(),
                    w.caliber.as_str()
                );
            }
        }
    }

    fn validate_events(&self) {
        for (id, def) in &self.events {
            let referrer = format!("event `{}`", id.as_str());
            for area in &def.target_areas {
                self.check_area(area, &referrer, "target_areas");
            }
            for faction in &def.involved_factions {
                self.check_faction(faction, &referrer, "involved_factions");
            }
            for chained in &def.chain_events {
                self.check_event(chained, &referrer, "chain_events");
            }
            for consequence in &def.consequences {
                self.check_consequence(consequence, &referrer);
            }
            if let Some(radio) = &def.radio {
                for intel_id in &radio.grants_intel {
                    self.check_intel(intel_id, &referrer, "radio.grants_intel");
                }
            }
        }
    }

    fn validate_triggers(&self) {
        for (id, def) in &self.triggers {
            let referrer = format!("trigger `{}`", id.as_str());
            self.check_quest(&def.quest, &referrer, "quest");
            match &def.kind {
                QuestTriggerKind::OnEvent(e) => self.check_event(e, &referrer, "on_event"),
                QuestTriggerKind::OnCondition(cond) => self.check_condition(cond, &referrer),
                QuestTriggerKind::OnGameStart | QuestTriggerKind::OnDay(_) => {}
            }
            if let Some(req) = &def.requires {
                self.check_condition(req, &referrer);
            }
        }
    }

    fn validate_npc_templates(&self) {
        for (id, def) in &self.npc_templates {
            let referrer = format!("npc template `{}`", id.as_str());
            self.check_faction(&def.faction, &referrer, "faction");
            if let Some(items) = &def.loadout {
                for item_id in items {
                    if !self.items.contains_key(item_id) {
                        warn_missing("item ref from", &referrer, "loadout[]", item_id);
                    }
                }
            }
        }
    }

    fn validate_quests(&self) {
        for (id, def) in &self.quests {
            let referrer = format!("quest `{}`", id.as_str());
            if def.stages.is_empty() {
                warn!("quest `{}` has no stages", id.as_str());
                continue;
            }
            if let Some(giver) = &def.giver {
                self.check_npc_template(giver, &referrer, "giver");
            }
            if let Some(faction) = &def.giver_faction {
                self.check_faction(faction, &referrer, "giver_faction");
            }
            self.validate_stage_references(def, &referrer);
        }
    }

    fn validate_stage_references(&self, def: &QuestDef, referrer: &str) {
        let ids: HashSet<_> = def.stages.iter().map(|s| &s.id).collect();
        for stage in &def.stages {
            let stage_ref = format!("{referrer} stage `{}`", stage.id.as_str());
            match &stage.kind {
                QuestStageKind::Talk(talk) => {
                    if !ids.contains(&talk.fallback) {
                        warn!("{stage_ref}: unknown fallback `{}`", talk.fallback.as_str());
                    }
                    if let Some(on_failure) = &talk.on_failure
                        && !ids.contains(on_failure)
                    {
                        warn!(
                            "{stage_ref}: on_failure → unknown stage `{}`",
                            on_failure.as_str()
                        );
                    }
                    // Duplicate choice strings silently shadow —
                    // serde keeps the first, so later branches
                    // with the same choice are unreachable.
                    let mut seen_choices: HashSet<&str> = HashSet::new();
                    for branch in &talk.branches {
                        if !seen_choices.insert(branch.choice.as_str()) {
                            warn!(
                                "{stage_ref}: duplicate TalkBranch choice `{}` — \
                                 only the first matching branch will ever be taken",
                                branch.choice
                            );
                        }
                        if !ids.contains(&branch.next_stage) {
                            warn!(
                                "{stage_ref}: branch `{}` → unknown stage `{}`",
                                branch.choice,
                                branch.next_stage.as_str()
                            );
                        }
                        if let Some(req) = &branch.requires {
                            self.check_condition(req, &stage_ref);
                        }
                    }
                }
                QuestStageKind::Objective(obj) => {
                    if !ids.contains(&obj.on_success) {
                        warn!(
                            "{stage_ref}: on_success → unknown stage `{}`",
                            obj.on_success.as_str()
                        );
                    }
                    if let Some(on_failure) = &obj.on_failure
                        && !ids.contains(on_failure)
                    {
                        warn!(
                            "{stage_ref}: on_failure → unknown stage `{}`",
                            on_failure.as_str()
                        );
                    }
                    self.check_condition(&obj.condition, &stage_ref);
                }
                QuestStageKind::Branch(br) => {
                    if !ids.contains(&br.fallback) {
                        warn!(
                            "{stage_ref}: branch fallback → unknown stage `{}`",
                            br.fallback.as_str()
                        );
                    }
                    for (i, arm) in br.arms.iter().enumerate() {
                        self.check_condition(&arm.when, &stage_ref);
                        if !ids.contains(&arm.next_stage) {
                            warn!(
                                "{stage_ref}: branch arm #{i} → unknown stage `{}`",
                                arm.next_stage.as_str()
                            );
                        }
                    }
                }
                QuestStageKind::Outcome(out) => {
                    for bundle in &out.consequences {
                        self.check_conditional_consequence(bundle, &stage_ref);
                    }
                }
            }
        }
    }

    fn check_condition(&self, cond: &ObjectiveCondition, referrer: &str) {
        match cond {
            ObjectiveCondition::HaveItem(q) => {
                self.check_item(&q.item, referrer, "HaveItem.item");
            }
            ObjectiveCondition::HaveCredits(_) => {}
            ObjectiveCondition::FactionStanding { faction, .. } => {
                self.check_faction(faction, referrer, "FactionStanding.faction");
            }
            ObjectiveCondition::HaveUpgrade(u) => {
                self.check_upgrade(u, referrer, "HaveUpgrade");
            }
            ObjectiveCondition::HaveIntel(i) => {
                self.check_intel(i, referrer, "HaveIntel");
            }
            ObjectiveCondition::EventActive(e) => {
                self.check_event(e, referrer, "EventActive");
            }
            ObjectiveCondition::QuestActive(q) => {
                self.check_quest(q, referrer, "QuestActive");
            }
            ObjectiveCondition::QuestCompleted(q) => {
                self.check_quest(q, referrer, "QuestCompleted");
            }
            ObjectiveCondition::QuestFlag { quest, .. } => {
                self.check_quest(quest, referrer, "QuestFlag.quest");
            }
            ObjectiveCondition::NpcAlive(t) | ObjectiveCondition::NpcDead(t) => {
                self.check_npc_template(t, referrer, "NpcAlive/NpcDead");
            }
            ObjectiveCondition::NpcAtLocation { npc, area } => {
                self.check_npc_template(npc, referrer, "NpcAtLocation.npc");
                self.check_area(area, referrer, "NpcAtLocation.area");
            }
            ObjectiveCondition::Wait { .. } => {}
            ObjectiveCondition::DaysWithoutPills { .. } => {}
            ObjectiveCondition::DayReached { .. } => {}
            ObjectiveCondition::AllOf(conds) | ObjectiveCondition::AnyOf(conds) => {
                for c in conds {
                    self.check_condition(c, referrer);
                }
            }
            ObjectiveCondition::Not(inner) => self.check_condition(inner, referrer),
        }
    }

    fn check_conditional_consequence(&self, bundle: &ConditionalConsequence, referrer: &str) {
        if let Some(when) = &bundle.when {
            self.check_condition(when, referrer);
        }
        for c in &bundle.apply {
            self.check_consequence(c, referrer);
        }
    }

    fn check_consequence(&self, c: &Consequence, referrer: &str) {
        match c {
            Consequence::StandingChange { faction, .. } => {
                self.check_faction(faction, referrer, "StandingChange.faction");
            }
            Consequence::GiveCredits(_) | Consequence::TakeCredits(_) => {}
            Consequence::GiveItem(q) => {
                self.check_item(&q.item, referrer, "GiveItem.item");
            }
            Consequence::TakeItem(q) => {
                self.check_item(&q.item, referrer, "TakeItem.item");
            }
            Consequence::TriggerEvent {
                event,
                target_area,
                involved_factions,
                ..
            } => {
                self.check_event(event, referrer, "TriggerEvent.event");
                if let Some(area) = target_area {
                    self.check_area(area, referrer, "TriggerEvent.target_area");
                }
                for f in involved_factions {
                    self.check_faction(f, referrer, "TriggerEvent.involved_factions");
                }
            }
            Consequence::StartQuest(q) => self.check_quest(q, referrer, "StartQuest"),
            Consequence::UnlockUpgrade(u) => self.check_upgrade(u, referrer, "UnlockUpgrade"),
            Consequence::GiveIntel(i) => self.check_intel(i, referrer, "GiveIntel"),
            Consequence::SpawnNpc { template, at } => {
                self.check_npc_template(template, referrer, "SpawnNpc.template");
                if let Some(area) = at {
                    self.check_area(area, referrer, "SpawnNpc.at");
                }
            }
            Consequence::GiveNpcXp { template, .. } => {
                self.check_npc_template(template, referrer, "GiveNpcXp.template");
            }
            Consequence::GivePlayerXp(_) => {}
            Consequence::DangerModifier { area, .. } => {
                if let Some(area) = area {
                    self.check_area(area, referrer, "DangerModifier.area");
                }
            }
            Consequence::PriceModifier { .. } => {}
        }
    }

    fn check_faction(&self, id: &Id<Faction>, referrer: &str, field: &str) {
        if !self.factions.contains_key(id) {
            warn_missing("faction ref from", referrer, field, id);
        }
    }

    fn check_item(&self, id: &Id<Item>, referrer: &str, field: &str) {
        if !self.items.contains_key(id) {
            warn_missing("item ref from", referrer, field, id);
        }
    }

    fn check_area(&self, id: &Id<Area>, referrer: &str, field: &str) {
        if !self.areas.contains_key(id) {
            warn_missing("area ref from", referrer, field, id);
        }
    }

    fn check_quest(&self, id: &Id<Quest>, referrer: &str, field: &str) {
        if !self.quests.contains_key(id) {
            warn_missing("quest ref from", referrer, field, id);
        }
    }

    fn check_event(&self, id: &Id<Event>, referrer: &str, field: &str) {
        if !self.events.contains_key(id) {
            warn_missing("event ref from", referrer, field, id);
        }
    }

    fn check_upgrade(&self, id: &Id<Upgrade>, referrer: &str, field: &str) {
        if !self.upgrades.contains_key(id) {
            warn_missing("upgrade ref from", referrer, field, id);
        }
    }

    fn check_npc_template(&self, id: &Id<NpcTemplate>, referrer: &str, field: &str) {
        if !self.npc_templates.contains_key(id) {
            warn_missing("npc template ref from", referrer, field, id);
        }
    }

    fn check_intel(&self, id: &Id<Intel>, referrer: &str, field: &str) {
        if !self.intel.contains_key(id) {
            warn_missing("intel ref from", referrer, field, id);
        }
    }

    /// Walk every consequence in every quest stage and event def
    /// and count currently-stubbed variants. One summary warning
    /// per stub variant that is actually authored, so quest
    /// designers see the problem at game-load time rather than
    /// only at runtime.
    fn warn_on_stub_consequences(&self) {
        let mut spawn_npc = 0usize;
        let mut give_npc_xp = 0usize;
        let mut danger_modifier = 0usize;
        let mut price_modifier = 0usize;

        let mut count = |c: &Consequence| match c {
            Consequence::SpawnNpc { .. } => spawn_npc += 1,
            Consequence::GiveNpcXp { .. } => give_npc_xp += 1,
            Consequence::DangerModifier { .. } => danger_modifier += 1,
            Consequence::PriceModifier { .. } => price_modifier += 1,
            _ => {}
        };

        for def in self.quests.values() {
            for stage in &def.stages {
                let QuestStageKind::Outcome(out) = &stage.kind else {
                    continue;
                };
                for bundle in &out.consequences {
                    for consequence in &bundle.apply {
                        count(consequence);
                    }
                }
            }
        }
        for event in self.events.values() {
            for consequence in &event.consequences {
                count(consequence);
            }
        }

        if spawn_npc > 0 {
            warn!(
                "STUB CONSEQUENCE `spawn_npc` referenced {spawn_npc}× in authored content \
                 — no visitor queue bridge yet, these will no-op at runtime."
            );
        }
        if give_npc_xp > 0 {
            warn!(
                "STUB CONSEQUENCE `give_npc_xp` referenced {give_npc_xp}× in authored content \
                 — no template→entity resolver yet, these will no-op at runtime."
            );
        }
        if danger_modifier > 0 {
            warn!(
                "STUB CONSEQUENCE `danger_modifier` referenced {danger_modifier}× in authored content \
                 — no AreaStates bridge yet, these will no-op at runtime."
            );
        }
        if price_modifier > 0 {
            warn!(
                "STUB CONSEQUENCE `price_modifier` referenced {price_modifier}× in authored content \
                 — no market system yet, these will no-op at runtime."
            );
        }
    }
}

// Small module-level helper used by the typed `check_*` methods.
// Stays a free function rather than an associated fn so the
// warning formatting lives in exactly one place and each
// `check_*` method's body is a single line.
fn warn_missing<T: IdMarker>(category: &str, referrer: &str, field: &str, id: &Id<T>) {
    warn!(
        "{category} `{}` references unknown {field} `{}`",
        referrer,
        id.as_str()
    );
}
