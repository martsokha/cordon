//! Plugin-contributed yarn command registry.
//!
//! Yarn-callable commands live near the systems they act on: a
//! trade command lives in `dialogue/commands.rs`, a
//! quest-advance command lives in `quest/bridge.rs`, and so on.
//! Each owning plugin registers its commands through
//! [`AppYarnCommandExt::add_yarn_command`], which stashes a
//! type-erased binder into the [`YarnCommandRegistry`] resource.
//!
//! When [`super::systems::spawn_dialogue_runner`] spawns a fresh
//! [`DialogueRunner`], it drains the registry onto the runner's
//! [`YarnCommands`] table. The dialogue runtime therefore never
//! needs to know which commands exist — each plugin contributes
//! its own, and nothing central grows when a new command lands.
//!
//! The alternative (a monolithic `TradeCommandSystems`-style
//! struct with hardcoded fields for every command) couples
//! unrelated feature modules through a shared resource. This
//! approach keeps feature plugins self-contained.

use std::borrow::Cow;

use bevy::ecs::system::SystemId;
use bevy::prelude::*;
use bevy_yarnspinner::TaskFinishedIndicator;
use bevy_yarnspinner::prelude::{YarnCommand, YarnCommands};
use yarnspinner::core::YarnFnParam;

/// A single yarn-command registration: its name and the
/// SystemId that executes it. Type-erased through this trait
/// so the registry can hold binders for commands with different
/// input types in one `Vec`. Internal: call sites outside this
/// module use [`AppYarnCommandExt::add_yarn_command`] instead.
trait YarnCommandBinder: Send + Sync + 'static {
    /// Install the command on the given yarnspinner registry
    /// under the stored name. Called once per spawned runner.
    fn bind(&self, commands: &mut YarnCommands);
}

/// Concrete binder: holds a name and a typed [`SystemId`], then
/// calls [`YarnCommands::add_command`] at bind time. Generic
/// over `In` (the `In<_>` input type of the registered system)
/// and `Output` (the system's return type) so the same binder
/// struct works for unit commands, string-arg commands, tuple-
/// arg commands, and task-returning commands alike.
struct SystemIdBinder<In, Output>
where
    In: YarnFnParam + for<'a> YarnFnParam<Item<'a> = In> + 'static,
    Output: TaskFinishedIndicator,
    SystemId<bevy::ecs::system::In<In>, Output>: YarnCommand<(In, Output)> + Clone,
{
    name: Cow<'static, str>,
    system: SystemId<bevy::ecs::system::In<In>, Output>,
}

impl<In, Output> YarnCommandBinder for SystemIdBinder<In, Output>
where
    In: YarnFnParam + for<'a> YarnFnParam<Item<'a> = In> + Send + Sync + 'static,
    Output: TaskFinishedIndicator,
    SystemId<bevy::ecs::system::In<In>, Output>:
        YarnCommand<(In, Output)> + Clone + Send + Sync + 'static,
{
    fn bind(&self, commands: &mut YarnCommands) {
        commands.add_command(self.name.clone(), self.system);
    }
}

/// Resource holding every plugin-contributed yarn command
/// registration. Populated at plugin-build time, consumed when
/// a new [`DialogueRunner`] spawns.
#[derive(Resource, Default)]
pub struct YarnCommandRegistry {
    entries: Vec<Box<dyn YarnCommandBinder>>,
}

impl YarnCommandRegistry {
    /// Install every registered command on the runner's command
    /// table. Called by [`super::systems::spawn_dialogue_runner`]
    /// once per runner.
    pub fn bind_all(&self, commands: &mut YarnCommands) {
        for entry in &self.entries {
            entry.bind(commands);
        }
    }
}

/// Extension trait on [`App`] that plugins use to contribute
/// yarn-callable commands. Registers the system with Bevy,
/// wraps the resulting SystemId in a binder, and pushes it into
/// the [`YarnCommandRegistry`] resource.
///
/// Usage inside a plugin's `build`:
///
/// ```ignore
/// app.add_yarn_command("give_item", give_item_system);
/// ```
///
/// The signature mirrors [`bevy::ecs::system::RegisterSystem`] —
/// the `system` value is whatever Bevy can coerce into
/// `IntoSystem<In<In>, Output, _>`, same as `register_system`
/// expects.
pub trait AppYarnCommandExt {
    fn add_yarn_command<InT, Output, M, S>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        system: S,
    ) -> &mut Self
    where
        InT: YarnFnParam + for<'a> YarnFnParam<Item<'a> = InT> + Send + Sync + 'static,
        Output: TaskFinishedIndicator,
        S: IntoSystem<bevy::ecs::system::In<InT>, Output, M> + 'static,
        M: 'static,
        SystemId<bevy::ecs::system::In<InT>, Output>:
            YarnCommand<(InT, Output)> + Clone + Send + Sync + 'static;
}

impl AppYarnCommandExt for App {
    fn add_yarn_command<InT, Output, M, S>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        system: S,
    ) -> &mut Self
    where
        InT: YarnFnParam + for<'a> YarnFnParam<Item<'a> = InT> + Send + Sync + 'static,
        Output: TaskFinishedIndicator,
        S: IntoSystem<bevy::ecs::system::In<InT>, Output, M> + 'static,
        M: 'static,
        SystemId<bevy::ecs::system::In<InT>, Output>:
            YarnCommand<(InT, Output)> + Clone + Send + Sync + 'static,
    {
        let system_id = self.world_mut().register_system(system);
        let binder: SystemIdBinder<InT, Output> = SystemIdBinder {
            name: name.into(),
            system: system_id,
        };
        let mut registry = self
            .world_mut()
            .get_resource_or_insert_with(YarnCommandRegistry::default);
        registry.entries.push(Box::new(binder));
        self
    }
}
