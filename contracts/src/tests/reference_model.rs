//! Simplified reference model for contract state used in invariant testing.
use std::collections::HashMap;
use soroban_sdk::Address;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct ReferenceModel {
    /// Balances of each user (including pending winnings).
    pub balances: HashMap<Address, i128>,
    /// Total pool amount for the current round.
    pub total_pool: i128,
    /// Pending winnings per user.
    pub pending_winnings: HashMap<Address, i128>,
    /// Recorded outcomes for diagnostics.
    pub outcomes: Vec<bool>,
}

impl ReferenceModel {
    /// Deposit tokens for a user.
    pub fn deposit(&mut self, user: &Address, amount: i128) {
        *self.balances.entry(user.clone()).or_default() += amount;
    }
    /// Withdraw tokens for a user (ensures non‑negative balance).
    pub fn withdraw(&mut self, user: &Address, amount: i128) {
        let entry = self.balances.entry(user.clone()).or_default();
        *entry = entry.saturating_sub(amount);
    }
    /// Place a bet (locks amount from user balance and adds to the pool).
    pub fn place_bet(&mut self, user: &Address, amount: i128) {
        self.withdraw(user, amount);
        self.total_pool = self.total_pool.saturating_add(amount);
    }
    /// Resolve a round. `winners` maps each winning user to the payout they should receive.
    pub fn resolve(&mut self, winners: &HashMap<Address, i128>) {
        for (user, payout) in winners {
            *self.pending_winnings.entry(user.clone()).or_default() += *payout;
            self.total_pool = self.total_pool.saturating_sub(*payout);
        }
        self.outcomes.push(true);
    }
    /// Claim pending winnings for a user (moves to balance).
    pub fn claim(&mut self, user: &Address) {
        if let Some(w) = self.pending_winnings.remove(user) {
            *self.balances.entry(user.clone()).or_default() += w;
        }
    }
    // ---------- Invariants ----------
    /// Invariant: total token count (balances + pending) never becomes negative.
    pub fn invariant_non_negative_total(&self) -> bool {
        let total_bal: i128 = self.balances.values().copied().sum();
        let total_pending: i128 = self.pending_winnings.values().copied().sum();
        total_bal + total_pending >= 0
    }
    /// Invariant: pending winnings never exceed the total pool that was available before resolution.
    pub fn invariant_pending_le_pool(&self) -> bool {
        let total_pending: i128 = self.pending_winnings.values().copied().sum();
        total_pending <= self.total_pool + total_pending
    }
}
