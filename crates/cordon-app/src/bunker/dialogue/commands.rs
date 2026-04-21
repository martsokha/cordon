//! Yarn-callable commands for trading mechanics.
//!
//! These are Bevy systems contributed to the shared
//! [`YarnCommandRegistry`](super::registry::YarnCommandRegistry)
//! at plugin-build time via
//! [`AppYarnCommandExt::add_yarn_command`]. The dialogue runtime
//! binds every registered command onto each fresh
//! `DialogueRunner`, and yarn files invoke them as
//! `<<command_name args...>>`. Each takes typed [`In<_>`] input
//! from the yarn call site and mutates game resources directly.
//!
//! Mirror variables (see [`super::mirror`]) refresh automatically
//! on the next frame via change detection on `Carrying`,
//! `PlayerStash`, and `PlayerIdentity` — so yarn can read updated
//! state in subsequent lines of the same node.
//!
//! # Registered commands
//!
//! - `<<give_item expected_id>>` — give the currently-carried
//!   item to the NPC. `expected_id` is a defensive check: if the
//!   carried item's id doesn't match (or the player isn't
//!   carrying anything), the command logs a warning and does
//!   nothing. Yarn authors should gate the option with
//!   `<<if $carrying == "expected_id">>` so the check only trips
//!   on misauthored yarn, never on normal play.
//! - `<<take_credits amount>>` — deduct credits from the player.
//!   Saturates at zero; no failure signal. Yarn should gate with
//!   `<<if $credits >= amount>>` to prevent negative-balance
//!   transactions showing as enabled options.
//! - `<<give_credits amount>>` — add credits to the player.
//! - `<<step_away "resume_node">>` — keeps the visitor at the
//!   counter after the current dialogue ends, with the given
//!   yarn node as the resume target. Player regains FPS
//!   control; interacting with the visitor sprite re-enters
//!   the runner at `resume_node`. Dialogue still needs to end
//!   on its own (end-of-node, `<<stop>>`, etc.) — this command
//!   only tags *how* the lifecycle handles that end.
//!
//! Dialogue that ends without `<<step_away>>` dismisses the
//! visitor — there's no explicit dismiss command because
//! dismissal is the default.
//!
//! The set is deliberately small. Quest-scale item transfers
//! (reward payouts, consumption from deep storage) stay on the
//! `quest/consequence.rs` path where consequences can be
//! declared in quest json rather than scattered through yarn.

use bevy::ecs::system::In;
use bevy::prelude::*;
use cordon_core::item::ItemInstance;
use cordon_core::primitive::Credits;
use cordon_data::gamedata::GameDataResource;
use cordon_sim::resources::{PlayerIdentity, PlayerStash};

use super::registry::AppYarnCommandExt;
use crate::bunker::radio::{ExitListening, ListeningToRadio};
use crate::bunker::rack::Carrying;
use crate::bunker::visitor::state::{PendingStepAway, VisitorState};

/// Register the built-in trade commands into the yarn-command
/// registry. Other plugins (e.g. the quest bridge) register
/// their own commands through the same
/// [`AppYarnCommandExt::add_yarn_command`] extension.
pub(super) fn register(app: &mut App) {
    app.add_yarn_command("give_item", give_item);
    app.add_yarn_command("take_credits", take_credits);
    app.add_yarn_command("give_credits", give_credits);
    app.add_yarn_command("step_away", step_away);
    app.add_yarn_command("deliver_order", deliver_order);
    app.add_yarn_command("close_radio", close_radio);
}

/// Give the currently-carried item to the interlocutor.
///
/// The yarn caller passes the id it *expects* to be carried, so
/// a mis-gated option (e.g. author forgot the `<<if>>` check)
/// fails loudly instead of silently consuming the wrong item.
fn give_item(In(expected_id): In<String>, mut commands: Commands, mut carrying: ResMut<Carrying>) {
    let Some(carried) = carrying.0.as_ref() else {
        warn!(
            "give_item `{expected_id}`: nothing carried; yarn should gate \
             this option with `<<if $carrying == \"{expected_id}\">>`"
        );
        return;
    };
    if carried.instance.def_id.as_str() != expected_id {
        warn!(
            "give_item `{expected_id}`: player is carrying `{}` instead; \
             yarn author gated on the wrong id",
            carried.instance.def_id.as_str()
        );
        return;
    }
    // Checked above; `take` always yields `Some` here. Despawn
    // the visual child on the FPS camera and clear the carry
    // slot — trade is one-way.
    let carried = carrying.0.take().expect("guarded by as_ref check above");
    commands.entity(carried.visual).despawn();
    info!("gave item `{expected_id}`");
}

/// Deduct credits from the player. Saturates at zero.
fn take_credits(In(amount): In<f32>, mut identity: ResMut<PlayerIdentity>) {
    let amount = to_credits(amount, "take_credits");
    let current = identity.credits.value();
    let new = current.saturating_sub(amount.value());
    identity.credits = Credits::new(new);
    info!("took {} credits (new balance: {})", amount.value(), new);
}

/// Add credits to the player.
fn give_credits(In(amount): In<f32>, mut identity: ResMut<PlayerIdentity>) {
    let amount = to_credits(amount, "give_credits");
    identity.credits += amount;
    info!(
        "gave {} credits (new balance: {})",
        amount.value(),
        identity.credits.value()
    );
}

/// Convert the yarn-provided `f32` (yarn's only numeric type) to
/// the strongly-typed [`Credits`]. Negatives clamp to 0 with a
/// warning — a yarn author writing `<<take_credits -50>>` almost
/// certainly meant `<<give_credits 50>>`, and silently flipping
/// the sign would bury the mistake. Values above `u32::MAX`
/// saturate instead of wrapping (which `f32 as u32` would do
/// for `≥ 2^32`).
fn to_credits(amount: f32, command: &str) -> Credits {
    if amount < 0.0 {
        warn!(
            "{command}: negative amount `{amount}` clamped to 0; \
             yarn should call the opposite command instead"
        );
        return Credits::new(0);
    }
    let saturated = amount.clamp(0.0, u32::MAX as f32) as u32;
    Credits::new(saturated)
}

/// Signal that the current conversation should leave the
/// visitor in `Waiting` with `resume_node` as the re-entry
/// target, rather than dismiss them. The visitor lifecycle's
/// `dismiss_on_dialogue_complete` consumes the flag on the
/// frame the runner returns to Idle.
///
/// The flag survives across lines — yarn can call
/// `<<step_away "…">>` mid-branch and continue speaking, and
/// the transition fires whenever the node eventually ends.
/// Calling it twice in one conversation overwrites with the
/// latest resume node.
///
/// **Ordering matters relative to `<<stop>>`**: if yarn writes
/// `<<stop>> <<step_away "…">>`, the runner ends before the
/// command executes and the flag is set too late to affect
/// *this* dialogue. Put `<<step_away>>` before `<<stop>>`.
/// Close the radio listening mode. Called from the `broadcast_static`
/// yarn node's close option.
///
/// Flips `ListeningToRadio::active` **synchronously** — when yarn
/// runs this command, the dialog it's inside is about to hit
/// end-of-node and fire `DialogueCompleted` in the same frame.
/// The radio's dialogue-complete observer checks `listening.active`
/// to decide whether to restart the idle yarn. Writing only the
/// `ExitListening` message here (processed next frame) leaves the
/// observer seeing `active = true` and reopening the idle yarn
/// for a second time before the teardown runs. Synchronous flip
/// avoids that race.
///
/// Also emits [`ExitListening`] so `handle_exit_listening` can do
/// the rest of the teardown (locks, audio, camera) on its normal
/// schedule slot.
fn close_radio(
    In(_): In<()>,
    mut listening: ResMut<ListeningToRadio>,
    mut exit_tx: MessageWriter<ExitListening>,
) {
    listening.active = false;
    exit_tx.write(ExitListening);
}

fn step_away(In(resume_node): In<String>, mut commands: Commands) {
    info!("visitor step-away requested (resume `{resume_node}`)");
    commands.insert_resource(PendingStepAway { resume_node });
}

/// Hand every item the currently-visiting supplier is carrying to
/// the player. Reads from the visitor attached to
/// [`VisitorState::Inside`] — the spawn path attaches the ordered
/// item ids to the visitor when the delivery spawn is dispatched.
///
/// Call once from the delivery yarn node; all queued items land
/// in the player's main stash, each as a fresh instance (no stack
/// merging). A supplier with multiple pending orders delivers
/// them all in one visit.
///
/// Warns and no-ops if:
/// - No visitor is currently inside (yarn called from a non-
///   visitor context).
/// - The visitor has no delivery items (yarn author mistakenly
///   put `<<deliver_order>>` on a non-delivery dialogue).
fn deliver_order(
    In(_): In<()>,
    mut state: ResMut<VisitorState>,
    data: Res<GameDataResource>,
    mut stash: ResMut<PlayerStash>,
) {
    let VisitorState::Inside { visitor, .. } = &mut *state else {
        warn!("deliver_order: no visitor currently inside; yarn misfire?");
        return;
    };
    if visitor.delivery_items.is_empty() {
        warn!(
            "deliver_order: visitor `{}` has no pending delivery items; \
             yarn should only call this from a delivery node",
            visitor.display_name
        );
        return;
    }
    // Drain all queued items at once. The yarn author calls
    // `<<deliver_order>>` a single time and every item the
    // visitor is carrying lands in the stash — no per-item
    // yarn loop needed.
    let drained: Vec<_> = std::mem::take(&mut visitor.delivery_items);
    let count = drained.len();
    for item_id in drained {
        let Some(item_def) = data.0.item(&item_id) else {
            warn!(
                "deliver_order: item `{}` not found in catalog",
                item_id.as_str()
            );
            continue;
        };
        stash.add_item(
            ItemInstance::new(item_def),
            cordon_core::item::StashScope::Main,
        );
        info!(
            "delivered `{}` from `{}`",
            item_id.as_str(),
            visitor.display_name
        );
    }
    info!("visit complete: {count} item(s) handed over");
}
