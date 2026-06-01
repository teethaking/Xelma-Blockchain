//! Security tests for Oracle data freshness and round validation.

use crate::contract::{VirtualTokenContract, VirtualTokenContractClient};
use crate::errors::ContractError;
use crate::types::{DataKey, OraclePayload};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, IntoVal,
};

#[test]
fn test_resolve_round_stale_timestamp() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin, &oracle);
    client.create_round(&1_0000000, &None);

    // Advance ledger time to 1000
    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
        li.sequence_number = 12; // Allow resolution
    });

    // Submit payload with timestamp 600 (400s old, > 300s limit)
    let payload = OraclePayload {
        price: 1_5000000,
        timestamp: 600,
        round_id: 0, // Starts at ledger 0
        nonce: 1u64,
    };

    let result = client.try_resolve_round(&payload);
    assert_eq!(result, Err(Ok(ContractError::StaleOracleData)));
}

#[test]
fn test_resolve_round_invalid_round_id() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin, &oracle);
    client.create_round(&1_0000000, &None);

    env.ledger().with_mut(|li| {
        li.sequence_number = 12;
    });

    // Submit payload with wrong round_id (e.g., 999 instead of 0)
    let payload = OraclePayload {
        price: 1_5000000,
        timestamp: env.ledger().timestamp(),
        round_id: 999,
        nonce: 1u64,
    };

    let result = client.try_resolve_round(&payload);
    assert_eq!(result, Err(Ok(ContractError::InvalidOracleRound)));
}

#[test]
fn test_resolve_round_valid_payload() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin, &oracle);
    client.create_round(&1_0000000, &None);

    env.ledger().with_mut(|li| {
        li.sequence_number = 12;
        li.timestamp = 1000;
    });

    // Valid payload: within 300s and correct round_id
    let payload = OraclePayload {
        price: 1_5000000,
        timestamp: 900, // 100s old, OK
        round_id: 0,
        nonce: 1u64,
    };

    client.resolve_round(&payload);
    assert_eq!(client.get_active_round(), None);
}

#[test]
fn test_resolve_round_future_timestamp() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin, &oracle);
    client.create_round(&1_0000000, &None);

    // Current ledger time is 1000
    env.ledger().with_mut(|li| {
        li.timestamp = 1000;
        li.sequence_number = 12;
    });

    // Submit payload with timestamp 1001 (future)
    let payload = OraclePayload {
        price: 1_5000000,
        timestamp: 1001,
        round_id: 0,
        nonce: 1u64,
    };

    let result = client.try_resolve_round(&payload);
    assert_eq!(result, Err(Ok(ContractError::FutureOracleData)));
}

// ─── Cancel-round security tests (Issue #111) ────────────────────────────────

#[test]
fn test_cancelled_round_cannot_be_resolved() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin, &oracle);
    client.create_round(&1_0000000, &None);

    client.cancel_round(&0u32);

    // After cancellation there is no active round, so resolve_round returns NoActiveRound
    env.ledger().with_mut(|li| {
        li.sequence_number = 12;
    });

    let result = client.try_resolve_round(&OraclePayload {
        price: 1_5000000,
        timestamp: env.ledger().timestamp(),
        round_id: 0,
        nonce: 1u64,
    });
    assert_eq!(result, Err(Ok(ContractError::NoActiveRound)));
}

#[test]
fn test_cancel_round_without_admin_auth_fails() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);

    // Initialize with only admin auth
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (&admin, &oracle).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &oracle);

    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "create_round",
            args: (1_0000000u128, Option::<u32>::None).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.create_round(&1_0000000, &None);

    // No auth for cancel_round
    let result = client.try_cancel_round(&0u32);
    assert!(result.is_err());
}

// ─── Oracle nonce replay protection (Issue #118) ─────────────────────────────

/// A nonce already consumed for a round must be rejected on re-submission.
/// We seed the consumed-nonce marker to simulate a prior submission, then
/// assert the resolver rejects a payload reusing that nonce for the same round.
#[test]
fn test_resolve_round_duplicate_nonce_rejected() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin, &oracle);
    client.create_round(&1_0000000, &None);
    let round = client.get_active_round().unwrap();

    env.ledger().with_mut(|li| {
        li.sequence_number = 12;
        li.timestamp = 1000;
    });

    // Simulate a prior submission having consumed nonce 42 for this round.
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&DataKey::ConsumedOracleNonce(round.round_id, 42u64), &true);
    });

    let result = client.try_resolve_round(&OraclePayload {
        price: 1_5000000,
        timestamp: 900,
        round_id: round.start_ledger,
        nonce: 42u64,
    });
    assert_eq!(result, Err(Ok(ContractError::OracleNonceReused)));
}

/// A fresh, unique nonce resolves normally and records the consumed marker.
#[test]
fn test_resolve_round_unique_nonce_resolves() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin, &oracle);
    client.create_round(&1_0000000, &None);
    let round = client.get_active_round().unwrap();

    env.ledger().with_mut(|li| {
        li.sequence_number = 12;
        li.timestamp = 1000;
    });

    client.resolve_round(&OraclePayload {
        price: 1_5000000,
        timestamp: 900,
        round_id: round.start_ledger,
        nonce: 7u64,
    });

    // Round resolved and the nonce is recorded as consumed for that round.
    assert_eq!(client.get_active_round(), None);
    env.as_contract(&contract_id, || {
        let consumed: bool = env
            .storage()
            .persistent()
            .get(&DataKey::ConsumedOracleNonce(round.round_id, 7u64))
            .unwrap_or(false);
        assert!(consumed, "resolved nonce must be marked consumed");
    });
}

/// Boundary nonces (0 and u64::MAX) are rejected on reuse for the same round.
#[test]
fn test_resolve_round_nonce_boundary_values() {
    let env = Env::default();
    let contract_id = env.register(VirtualTokenContract, ());
    let client = VirtualTokenContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    env.mock_all_auths();

    client.initialize(&admin, &oracle);
    client.create_round(&1_0000000, &None);
    let round = client.get_active_round().unwrap();

    env.ledger().with_mut(|li| {
        li.sequence_number = 12;
        li.timestamp = 1000;
    });

    // Pre-seed both boundary nonces as consumed for this round.
    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&DataKey::ConsumedOracleNonce(round.round_id, 0u64), &true);
        env.storage().persistent().set(
            &DataKey::ConsumedOracleNonce(round.round_id, u64::MAX),
            &true,
        );
    });

    let zero = client.try_resolve_round(&OraclePayload {
        price: 1_5000000,
        timestamp: 900,
        round_id: round.start_ledger,
        nonce: 0u64,
    });
    assert_eq!(zero, Err(Ok(ContractError::OracleNonceReused)));

    let max = client.try_resolve_round(&OraclePayload {
        price: 1_5000000,
        timestamp: 900,
        round_id: round.start_ledger,
        nonce: u64::MAX,
    });
    assert_eq!(max, Err(Ok(ContractError::OracleNonceReused)));
}
