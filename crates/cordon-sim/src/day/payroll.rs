//! Daily expense deduction: runs once per [`DayRolled`], tallies
//! all recurring costs (squad upkeep, garrison bribe, syndicate
//! interest on outstanding debt), deducts what the player can
//! afford, and pushes any shortfall onto [`PlayerState::debt`].
//!
//! The resulting [`DailyExpenseReport`] is stored in
//! [`LastDailyExpenses`] so the UI can display "last day's costs"
//! at any time.

use bevy::prelude::*;
use cordon_core::entity::player::{DailyExpenseReport, ExpenseKind, ExpenseLine};
use cordon_core::primitive::{Credits, Experience, Rank};

use crate::behavior::squad::Owned;
use crate::behavior::squad::identity::SquadMembers;
use crate::resources::{GameClock, PlayerIdentity};

/// Fixed daily bribe the Garrison demands for "protection."
const GARRISON_BRIBE: u32 = 50;

/// Daily interest rate on outstanding debt, in basis points
/// (1 bp = 0.01%). 500 bp = 5% per day — aggressive, because
/// the Syndicate doesn't do charity.
const SYNDICATE_INTEREST_BPS: u32 = 500;

/// Most recent day's expense breakdown, readable by the UI.
#[derive(Resource, Default)]
pub struct LastDailyExpenses(pub Option<DailyExpenseReport>);

/// Emitted when daily expenses are processed. Consumed by the
/// toast system.
#[derive(Message, Debug, Clone)]
pub struct DailyExpensesProcessed {
    pub total: Credits,
}

/// Compute all daily expenses, deduct from the player's credits,
/// push any shortfall onto debt, and store the report for the UI.
pub(super) fn process_daily_expenses(
    clock: Res<GameClock>,
    mut identity: ResMut<PlayerIdentity>,
    mut last: ResMut<LastDailyExpenses>,
    owned_squads: Query<&SquadMembers, With<Owned>>,
    members_xp: Query<&Experience>,
    mut processed_tx: MessageWriter<DailyExpensesProcessed>,
) {
    let mut lines = Vec::new();

    // Squad upkeep: sum Rank::pay() across every member of every
    // owned squad.
    let mut squad_total = 0u32;
    for members in &owned_squads {
        for &member in &members.0 {
            let rank = members_xp
                .get(member)
                .map(|xp| Rank::from_xp(*xp))
                .unwrap_or(Rank::Novice);
            squad_total += rank.pay().value();
        }
    }
    if squad_total > 0 {
        lines.push(ExpenseLine {
            kind: ExpenseKind::SquadUpkeep,
            amount: Credits::new(squad_total),
        });
    }

    // Garrison bribe: flat daily fee.
    lines.push(ExpenseLine {
        kind: ExpenseKind::GarrisonBribe,
        amount: Credits::new(GARRISON_BRIBE),
    });

    // Syndicate interest on outstanding debt.
    let debt_val = identity.debt.value();
    if debt_val > 0 {
        let interest = ((debt_val as u64 * SYNDICATE_INTEREST_BPS as u64 / 10_000) as u32).max(1);
        lines.push(ExpenseLine {
            kind: ExpenseKind::SyndicateInterest,
            amount: Credits::new(interest),
        });
    }

    // Total and deduct.
    let total_val: u32 = lines.iter().map(|l| l.amount.value()).sum();
    let total = Credits::new(total_val);
    let available = identity.credits.value();

    let shortfall = if available >= total_val {
        identity.credits -= total;
        0
    } else {
        identity.credits = Credits::new(0);
        total_val - available
    };
    identity.debt += Credits::new(shortfall);

    last.0 = Some(DailyExpenseReport {
        day: clock.0.day,
        lines,
        total,
        shortfall: Credits::new(shortfall),
    });

    if total_val > 0 {
        processed_tx.write(DailyExpensesProcessed { total });
    }

    info!(
        "day {} payroll: {} total, {} shortfall (debt now {})",
        clock.0.day.value(),
        total_val,
        shortfall,
        identity.debt.value(),
    );
}
