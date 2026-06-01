//! Tests for boundary conditions and unusual scenarios.

use crate::contract::{VirtualTokenContract, VirtualTokenContractClient};
use crate::types::{BetSide, DataKey, OraclePayload, Round, UserPosition};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, Map,
};

#[test]
fn test_round_with_no_participants() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);

    env.mock_all_auths();

    client.initialize(&admin, &oracle);

    // Create round with no bets
    client.create_round(&1_0000000, &None);

    let round = client.get_active_round().unwrap();
    assert_eq!(round.pool_up, 0);
    assert_eq!(round.pool_down, 0);

    // Advance ledger to allow resolution
    env.ledger().with_mut(|li| {
        li.sequence_number = 12;
    });
    // Resolve with no participants
    client.resolve_round(&OraclePayload {
        price: 1_5000000,
        timestamp: env.ledger().timestamp(),
        round_id: 0,
        nonce: 1u64,
    });

    // Should clear round without errors
    assert_eq!(client.get_active_round(), None);
}

#[test]
fn test_round_with_only_one_side() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    env.mock_all_auths();

    client.initialize(&admin, &oracle);
    client.mint_initial(&alice);
    client.mint_initial(&bob);

    // Create round and only bet on UP
    client.create_round(&1_0000000, &None);
    client.place_bet(&alice, &100_0000000, &BetSide::Up);
    client.place_bet(&bob, &150_0000000, &BetSide::Up);

    let round = client.get_active_round().unwrap();
    assert_eq!(round.pool_up, 250_0000000);
    assert_eq!(round.pool_down, 0);

    // Advance ledger to allow resolution
    env.ledger().with_mut(|li| {
        li.sequence_number = 12;
    });
    // Resolve - UP wins but no losers to take from
    client.resolve_round(&OraclePayload {
        price: 1_5000000,
        timestamp: env.ledger().timestamp(),
        round_id: 0,
        nonce: 1u64,
    });

    // Winners should only get their bets back (no losing pool to split)
    assert_eq!(client.get_pending_winnings(&alice), 100_0000000);
    assert_eq!(client.get_pending_winnings(&bob), 150_0000000);
}

#[test]
fn test_accumulate_pending_winnings() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let alice = Address::generate(&env);

    env.mock_all_auths();

    client.initialize(&admin, &oracle);
    client.mint_initial(&alice);

    // Round 1: Alice bets UP and wins
    client.create_round(&1_0000000, &None);
    client.place_bet(&alice, &100_0000000, &BetSide::Up);

    env.as_contract(&contract_id, || {
        let mut positions = Map::<Address, UserPosition>::new(&env);
        positions.set(
            alice.clone(),
            UserPosition {
                amount: 100_0000000,
                side: BetSide::Up,
            },
        );
        env.storage()
            .persistent()
            .set(&DataKey::UpDownPositions, &positions);

        let mut round: Round = env
            .storage()
            .persistent()
            .get(&DataKey::ActiveRound)
            .unwrap();
        round.pool_up = 100_0000000;
        round.pool_down = 50_0000000;
        env.storage()
            .persistent()
            .set(&DataKey::ActiveRound, &round);
    });

    // Advance ledger to allow resolution
    env.ledger().with_mut(|li| {
        li.sequence_number = 12;
    });
    let round1 = client.get_active_round().unwrap();
    client.resolve_round(&OraclePayload {
        price: 1_5000000, // UP wins
        timestamp: env.ledger().timestamp(),
        round_id: round1.start_ledger,
        nonce: 1u64,
    });

    let first_pending = client.get_pending_winnings(&alice);
    assert!(first_pending > 0);

    // Round 2: Alice bets and gets refund
    client.create_round(&2_0000000, &None);
    client.place_bet(&alice, &50_0000000, &BetSide::Down);

    // Advance ledger to allow resolution
    env.ledger().with_mut(|li| {
        li.sequence_number = 24; // 12 + 12 for second round
    });
    let round2 = client.get_active_round().unwrap();
    client.resolve_round(&OraclePayload {
        price: 2_0000000, // Price unchanged - refund
        timestamp: env.ledger().timestamp(),
        round_id: round2.start_ledger,
        nonce: 1u64,
    });

    // Should have accumulated pending from both rounds
    let total_pending = client.get_pending_winnings(&alice);
    assert_eq!(total_pending, first_pending + 50_0000000);

    // Claim all at once
    let claimed = client.claim_winnings(&alice);
    assert_eq!(claimed, total_pending);
    assert_eq!(client.get_pending_winnings(&alice), 0);
}

#[test]
fn test_claim_winnings_checked_overflow() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let alice = Address::generate(&env);

    env.mock_all_auths();

    client.initialize(&admin, &oracle);
    client.mint_initial(&alice);

    // Artificially set balance to near i128::MAX and pending winnings to a
    // value that would overflow when added.
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&DataKey::Balance(alice.clone()), &i128::MAX);
        env.storage()
            .persistent()
            .set(&DataKey::PendingWinnings(alice.clone()), &1_i128);
    });

    // claim_winnings should fail with Overflow because balance + pending > i128::MAX
    let result = client.try_claim_winnings(&alice);
    assert!(result.is_err());
}

#[test]
fn test_stats_checked_overflow() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let alice = Address::generate(&env);

    env.mock_all_auths();

    client.initialize(&admin, &oracle);
    client.mint_initial(&alice);

    // Set stats near u32::MAX so the next win overflows total_wins
    use crate::types::UserStats;
    env.as_contract(&contract_id, || {
        let stats = UserStats {
            total_wins: u32::MAX,
            total_losses: 0,
            current_streak: 0,
            best_streak: 0,
        };
        env.storage()
            .persistent()
            .set(&DataKey::UserStats(alice.clone()), &stats);
    });

    // Create a round, bet, and resolve so _update_stats_win is triggered
    client.create_round(&1_0000000, &None);
    client.place_bet(&alice, &100_0000000, &BetSide::Up);

    // Add a losing side so there's a loser pool
    let bob = Address::generate(&env);
    client.mint_initial(&bob);
    client.place_bet(&bob, &50_0000000, &BetSide::Down);

    env.ledger().with_mut(|li| {
        li.sequence_number = 12;
    });

    // Resolve should fail because _update_stats_win overflows total_wins
    let result = client.try_resolve_round(&OraclePayload {
        price: 2_0000000,
        timestamp: env.ledger().timestamp(),
        round_id: 0,
        nonce: 1u64,
    });
    assert!(result.is_err());
}
