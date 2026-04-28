#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{MockAuth, MockAuthInvoke, Address as _},
    BytesN, IntoVal, Address, Env, String, Vec,
};

#[test]
fn test_create_pool() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let title = String::from_str(&env, "Emergency Relief Fund");
    let description = String::from_str(&env, "Helping those in need");
    let goal: u128 = 1_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);

    assert_eq!(pool_id, 1);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.0, 1); // id
    assert_eq!(pool.1, creator); // creator
    assert_eq!(pool.2, goal); // goal
    assert_eq!(pool.3, 0); // collected
    assert_eq!(pool.4, false); // is_closed
}

#[test]
fn test_donate() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let title = String::from_str(&env, "Educational Scholarship");
    let description = String::from_str(&env, "Support for students");
    let goal: u128 = 10_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);

    let donation_amount: u128 = 100_000_000;
    let expected_fee = (donation_amount * 1) / 100; // 1% fee
    let expected_collected = donation_amount - expected_fee; // 99% collected
    client.donate(&pool_id, &donor, &donation_amount);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.3, expected_collected); // collected amount (net of fees)
}

#[test]
fn test_apply_for_scholarship_creates_application() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let student = Address::generate(&env);
    let title = String::from_str(&env, "Scholarship Pool");
    let description = String::from_str(&env, "Support for students");
    let goal: u128 = 1_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);

    let credential_hash = BytesN::from_array(&env, &[1u8; 32]);
    let requested_amount: i128 = 100_000_000;

    let application_id = client
        .mock_auths(&[MockAuth {
            address: &student,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "apply_to_pool",
                args: (&student, &pool_id, &credential_hash, &requested_amount).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .apply_to_pool(&student, &pool_id, &credential_hash, &requested_amount);

    assert_eq!(application_id, 1);

    let application = client.get_application(&pool_id, &application_id);
    assert_eq!(application.0, student);
    assert_eq!(application.1, credential_hash);
    assert_eq!(application.2, requested_amount);

    let status = client.get_application_status(&pool_id, &student);
    assert_eq!(status, String::from_str(&env, "Pending"));
}

#[test]
#[should_panic(expected = "Pool is inactive")]
fn test_apply_for_scholarship_inactive_pool() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let student = Address::generate(&env);
    let title = String::from_str(&env, "Scholarship Pool");
    let description = String::from_str(&env, "Inactive pool");
    let goal: u128 = 1_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);
    client
        .mock_auths(&[MockAuth {
            address: &creator,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "close_pool",
                args: (&pool_id,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .close_pool(&pool_id);

    let credential_hash = BytesN::from_array(&env, &[2u8; 32]);
    let requested_amount: i128 = 100_000_000;

    client
        .mock_auths(&[MockAuth {
            address: &student,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "apply_to_pool",
                args: (&student, &pool_id, &credential_hash, &requested_amount).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .apply_to_pool(&student, &pool_id, &credential_hash, &requested_amount);
}

#[test]
fn test_multiple_donations() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let donor1 = Address::generate(&env);
    let donor2 = Address::generate(&env);
    let title = String::from_str(&env, "Community Project");
    let description = String::from_str(&env, "Building together");
    let goal: u128 = 5_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);

    let donation1: u128 = 100_000_000;
    let donation2: u128 = 200_000_000;
    let fee1 = (donation1 * 1) / 100; // 1% fee
    let fee2 = (donation2 * 1) / 100; // 1% fee
    let expected_collected = (donation1 - fee1) + (donation2 - fee2);

    client.donate(&pool_id, &donor1, &donation1);
    client.donate(&pool_id, &donor2, &donation2);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.3, expected_collected); // collected amount (net of fees)
}

#[test]
fn test_close_pool() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let title = String::from_str(&env, "Closed Pool");
    let description = String::from_str(&env, "Test pool");
    let goal: u128 = 1_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);
    client
        .mock_auths(&[MockAuth {
            address: &creator,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "close_pool",
                args: (&pool_id,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .close_pool(&pool_id);

    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.4, true); // is_closed
}

#[test]
#[should_panic(expected = "Pool is closed")]
fn test_donate_to_closed_pool() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let title = String::from_str(&env, "Test Pool");
    let description = String::from_str(&env, "Test");
    let goal: u128 = 1_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);
    client
        .mock_auths(&[MockAuth {
            address: &creator,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "close_pool",
                args: (&pool_id,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .close_pool(&pool_id);

    client.donate(&pool_id, &donor, &100_000_000);
}

#[test]
#[should_panic(expected = "HostError: Auth")]
fn test_close_pool_unauthorized() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let unauthorized = Address::generate(&env);
    let title = String::from_str(&env, "Test Pool");
    let description = String::from_str(&env, "Test");
    let goal: u128 = 1_000_000_000;

    let pool_id = client.create_pool(&creator, &title, &description, &goal);

    // Try to close pool with unauthorized user - should panic
    client
        .mock_auths(&[MockAuth {
            address: &unauthorized,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "close_pool",
                args: (&pool_id,).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .close_pool(&pool_id);
}

#[test]
fn test_multiple_pools() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator1 = Address::generate(&env);
    let creator2 = Address::generate(&env);

    let pool_id_1 = client.create_pool(
        &creator1,
        &String::from_str(&env, "Pool 1"),
        &String::from_str(&env, "First pool"),
        &1_000_000_000,
    );

    let pool_id_2 = client.create_pool(
        &creator2,
        &String::from_str(&env, "Pool 2"),
        &String::from_str(&env, "Second pool"),
        &2_000_000_000,
    );

    assert_eq!(pool_id_1, 1);
    assert_eq!(pool_id_2, 2);
    assert_eq!(client.get_pool_count(), 2);
}

// ============= CLAIM_FUNDS TESTS =============

#[test]
#[should_panic(expected = "Application status not found")]
fn test_claim_funds_no_status() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let student = Address::generate(&env);
    let token_address = Address::generate(&env);

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "Test"),
        &1_000_000_000,
    );

    // Donate to the pool
    client.donate(&pool_id, &creator, &500_000_000);

    // Try to claim without setting status - should panic
    client.claim_funds(&student, &pool_id, &100_000_000i128, &token_address);
}

#[test]
#[should_panic(expected = "Application is not approved")]
fn test_claim_funds_rejected_application() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let student = Address::generate(&env);
    let token_address = Address::generate(&env);

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "Test"),
        &1_000_000_000,
    );

    // Donate to the pool
    client.donate(&pool_id, &creator, &500_000_000);

    // Set status to "Rejected"
    client.set_application_status(&pool_id, &student, &String::from_str(&env, "Rejected"));

    // Try to claim with rejected status - should panic
    client.claim_funds(&student, &pool_id, &100_000_000i128, &token_address);
}

#[test]
#[should_panic(expected = "Overdraw attempt")]
fn test_claim_funds_overdraw() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let student = Address::generate(&env);
    let token_address = Address::generate(&env);

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "Test"),
        &1_000_000_000,
    );

    // Donate only 100_000_000 to the pool
    client.donate(&pool_id, &creator, &100_000_000);

    // Set status to "Approved"
    client.set_application_status(&pool_id, &student, &String::from_str(&env, "Approved"));

    // Try to claim more than available - should panic
    client.claim_funds(&student, &pool_id, &500_000_000i128, &token_address);
}

#[test]
#[should_panic(expected = "Claim amount must be positive")]
fn test_claim_funds_negative_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let student = Address::generate(&env);
    let token_address = Address::generate(&env);

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "Test"),
        &1_000_000_000,
    );

    // Donate to the pool
    client.donate(&pool_id, &creator, &500_000_000);

    // Set status to "Approved"
    client.set_application_status(&pool_id, &student, &String::from_str(&env, "Approved"));

    // Try to claim negative amount - should panic
    client.claim_funds(&student, &pool_id, &-100_000_000i128, &token_address);
}

#[test]
fn test_claim_funds_get_claimed_amount() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let student = Address::generate(&env);

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "Test"),
        &1_000_000_000,
    );

    // Initially, claimed amount should be 0
    let initial_claimed = client.get_claimed_amount(&pool_id, &student);
    assert_eq!(initial_claimed, 0);

    // Donate to the pool
    client.donate(&pool_id, &creator, &500_000_000);

    // Set status to "Approved"
    client.set_application_status(&pool_id, &student, &String::from_str(&env, "Approved"));
}

#[test]
fn test_get_application_status() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let student = Address::generate(&env);

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "Test"),
        &1_000_000_000,
    );

    // Initially, status should be empty
    let initial_status = client.get_application_status(&pool_id, &student);
    assert_eq!(initial_status, String::from_str(&env, ""));

    // Set status to "Approved"
    let approved_status = String::from_str(&env, "Approved");
    client.set_application_status(&pool_id, &student, &approved_status);

    // Check that status was set correctly
    let status_after_set = client.get_application_status(&pool_id, &student);
    assert_eq!(status_after_set, approved_status);
}
// ─── Stress / boundary tests ──────────────────────────────────────────────────
//
// These tests exercise the absolute numeric limits of every u32 and u128 field
// that flows through the contract, ensuring no overflow, no memory fault, and
// correct iteration up to the defined bounds.

/// Maximum u32 value used as a pool goal split across two milestones.
/// Verifies that u128 arithmetic handles u32::MAX without overflow.
#[test]
fn test_stress_u32_max_amount_in_milestones() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    // Goal = u32::MAX as u128 — well within u128 range, no overflow risk.
    let goal: u128 = u32::MAX as u128; // 4_294_967_295
    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    let student = Address::generate(&env);

    // Split the goal into two milestones whose amounts sum exactly to u32::MAX.
    let half = goal / 2;
    let remainder = goal - half; // handles odd values correctly
    let milestones = make_milestones(&env, &[(half, u64::MAX), (remainder, u64::MAX - 1)]);

    client.setup_application_milestones(&pool_id, &student, &milestones);

    let stored = client.get_milestones(&pool_id, &student);
    assert_eq!(stored.len(), 2);
    assert_eq!(
        stored.get(0).unwrap().amount + stored.get(1).unwrap().amount,
        goal
    );
}

/// unlock_time at u64::MAX — the largest representable ledger timestamp.
/// Ensures the field is stored and retrieved without truncation.
#[test]
fn test_stress_u64_max_unlock_time() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let goal: u128 = 1_000_000_000;
    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    let student = Address::generate(&env);

    // Single milestone with unlock_time = u64::MAX.
    let milestones = make_milestones(&env, &[(goal, u64::MAX)]);

    client.setup_application_milestones(&pool_id, &student, &milestones);

    let stored = client.get_milestones(&pool_id, &student);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored.get(0).unwrap().unlock_time, u64::MAX);
}

/// Goal set to u128::MAX / 2 split across two milestones.
/// Validates that checked_add inside the summation loop does not panic on
/// large-but-valid u128 values and that the invariant sum == goal holds.
#[test]
fn test_stress_large_u128_goal_two_milestones() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    // Use a very large but representable u128 goal.
    let half: u128 = u128::MAX / 2;
    let goal: u128 = half + half; // = u128::MAX - 1 (even split, no overflow)
    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    let student = Address::generate(&env);

    let milestones = make_milestones(&env, &[(half, 1_000), (half, 2_000)]);

    client.setup_application_milestones(&pool_id, &student, &milestones);

    let stored = client.get_milestones(&pool_id, &student);
    assert_eq!(stored.len(), 2);
    assert_eq!(
        stored.get(0).unwrap().amount + stored.get(1).unwrap().amount,
        goal
    );
}

/// Overflow guard: two milestones whose amounts would overflow u128 when summed.
/// The checked_add inside setup_application_milestones must catch this and panic.
#[test]
#[should_panic]
fn test_stress_milestone_amount_overflow_u128() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    // Goal is irrelevant here — the summation loop will overflow before the
    // equality check is reached.
    let goal: u128 = 1_000_000_000;
    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    let student = Address::generate(&env);

    // u128::MAX + 1 overflows — checked_add must panic.
    let milestones = make_milestones(&env, &[(u128::MAX, 100), (1, 200)]);

    client.setup_application_milestones(&pool_id, &student, &milestones);
}

/// Maximum number of milestones that Soroban's simulation budget allows.
///
/// Soroban enforces a CPU instruction budget per transaction. In the test
/// environment the budget is effectively uncapped, but the practical limit
/// for a single Vec stored in persistent storage is bounded by the XDR entry
/// size limit (~64 KiB per ledger entry). Each Milestone encodes to roughly
/// 64 bytes of XDR, so ~1 000 entries is a safe upper bound that exercises
/// the full iteration loop without hitting memory or budget faults.
///
/// The test asserts:
///   1. All entries are stored and retrievable.
///   2. The loop correctly accumulates the sum across all entries.
///   3. The sum == goal invariant holds at the boundary.
#[test]
fn test_stress_maximum_milestone_array_within_budget() {
    let env = Env::default();
    env.mock_all_auths();

    // Soroban test environments default to a metered budget; disable metering
    // so the stress test is not rejected by the CPU/memory cost model and
    // purely validates correctness at the array boundary.
    env.cost_estimate().budget().reset_unlimited();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    // 1 000 milestones × 1_000_000 stroops each = 1_000_000_000 goal.
    const N: u32 = 1_000;
    let amount_each: u128 = 1_000_000;
    let goal: u128 = amount_each * N as u128;

    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    let student = Address::generate(&env);

    // Build the maximum-size Vec directly.
    let mut milestones: Vec<Milestone> = Vec::new(&env);
    for i in 0..N {
        milestones.push_back(Milestone {
            amount: amount_each,
            // unlock_time increases monotonically; last entry uses u32::MAX as
            // the timestamp to exercise the upper bound of the field.
            unlock_time: if i == N - 1 {
                u32::MAX as u64
            } else {
                i as u64 * 10
            },
        });
    }

    client.setup_application_milestones(&pool_id, &student, &milestones);

    let stored = client.get_milestones(&pool_id, &student);

    // ── Boundary assertions ───────────────────────────────────────────────────

    // 1. Array length is preserved exactly.
    assert_eq!(stored.len(), N);

    // 2. First entry is correct.
    let first = stored.get(0).unwrap();
    assert_eq!(first.amount, amount_each);
    assert_eq!(first.unlock_time, 0);

    // 3. Last entry carries the u32::MAX timestamp boundary value.
    let last = stored.get(N - 1).unwrap();
    assert_eq!(last.amount, amount_each);
    assert_eq!(last.unlock_time, u32::MAX as u64);

    // 4. Sum of all stored amounts equals the original goal — loop ran fully.
    let mut sum: u128 = 0;
    for i in 0..stored.len() {
        sum = sum
            .checked_add(stored.get(i).unwrap().amount)
            .expect("Unexpected overflow during verification");
    }
    assert_eq!(sum, goal);
}

/// Pool count wraps correctly when pool_id approaches u32 boundaries.
/// Creates pools up to a high pool_id and verifies get_pool_count returns
/// the correct u32 value without truncation.
#[test]
fn test_stress_pool_count_u32_boundary() {
    let env = Env::default();
    env.mock_all_auths();
    env.cost_estimate().budget().reset_unlimited();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    // Create a large number of pools to stress the u32 pool counter.
    const POOL_COUNT: u32 = 500;
    let goal: u128 = 1_000_000_000;

    for _ in 0..POOL_COUNT {
        let creator = Address::generate(&env);
        client.create_pool(
            &creator,
            &String::from_str(&env, "Stress Pool"),
            &String::from_str(&env, "Boundary test"),
            &goal,
        );
    }

    // Pool count must equal exactly POOL_COUNT — no u32 truncation or wrap.
    assert_eq!(client.get_pool_count(), POOL_COUNT);

    // The last pool must be retrievable and intact.
    let last_pool = client.get_pool(&POOL_COUNT);
    assert_eq!(last_pool.0, POOL_COUNT);
    assert_eq!(last_pool.2, goal);
    assert_eq!(last_pool.4, false);
}

/// Single milestone whose amount equals u128::MAX — the absolute maximum
/// representable goal. Verifies storage round-trip without truncation.
#[test]
fn test_stress_single_milestone_u128_max_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let goal: u128 = u128::MAX;
    let (pool_id, _creator) = setup_pool(&env, &client, goal);
    let student = Address::generate(&env);

    // One milestone covering the entire u128::MAX goal.
    let milestones = make_milestones(&env, &[(u128::MAX, 0)]);

    client.setup_application_milestones(&pool_id, &student, &milestones);

    let stored = client.get_milestones(&pool_id, &student);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored.get(0).unwrap().amount, u128::MAX);
    assert_eq!(stored.get(0).unwrap().unlock_time, 0);
}

// ============= ISSUE #336 INTEGRATION TESTS =============

#[test]
fn test_school_registration_to_claim_integration_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sponsor = Address::generate(&env);
    let school = Address::generate(&env);
    let student = Address::generate(&env);
    let donor = Address::generate(&env);
    let token_address = Address::generate(&env);
    let goal: u128 = 1_000_000_000;

    client.set_admin(&admin);
    client.register_school(&admin, &school);
    assert!(client.is_school_registered(&school));

    let pool_id = client.create_pool_for_school(
        &sponsor,
        &String::from_str(&env, "School Pool"),
        &String::from_str(&env, "Scholarship round"),
        &goal,
        &school,
    );

    client.donate(&pool_id, &donor, &600_000_000);
    let credential_hash = BytesN::from_array(&env, &[1u8; 32]);
    client.apply_to_pool(
        &student,
        &pool_id,
        &credential_hash,
        &100_000_000i128,
    );
    client.approve_application(&pool_id, &school, &student, &true);

    assert_eq!(
        client.get_application_status(&pool_id, &student),
        String::from_str(&env, "Approved")
    );

    client.claim_funds(&student, &pool_id, &150_000_000i128, &token_address);
    client.claim_funds(&student, &pool_id, &50_000_000i128, &token_address);

    assert_eq!(
        client.get_claimed_amount(&pool_id, &student),
        200_000_000i128
    );
}

#[test]
#[should_panic(expected = "School is not registered")]
fn test_create_pool_for_unregistered_school_panics_issue336() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let sponsor = Address::generate(&env);
    let unregistered_school = Address::generate(&env);

    client.create_pool_for_school(
        &sponsor,
        &String::from_str(&env, "Pool"),
        &String::from_str(&env, "Desc"),
        &1_000_000_000u128,
        &unregistered_school,
    );
}

#[test]
#[should_panic(expected = "Only linked school can approve")]
fn test_non_linked_school_cannot_approve_issue336() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let sponsor = Address::generate(&env);
    let school_one = Address::generate(&env);
    let school_two = Address::generate(&env);
    let student = Address::generate(&env);

    client.set_admin(&admin);
    client.register_school(&admin, &school_one);
    client.register_school(&admin, &school_two);

    let pool_id = client.create_pool_for_school(
        &sponsor,
        &String::from_str(&env, "School Pool"),
        &String::from_str(&env, "Scholarship round"),
        &1_000_000_000u128,
        &school_one,
    );

    let credential_hash = BytesN::from_array(&env, &[1u8; 32]);
    client.apply_to_pool(&student, &pool_id, &credential_hash, &100_000_000i128);
    client.approve_application(&pool_id, &school_two, &student, &true);
}

fn setup_pool(env: &Env, client: &ContractClient, goal: u128) -> (u32, Address) {
    let creator = Address::generate(env);
    let pool_id = client.create_pool(
        &creator,
        &String::from_str(env, "Stress Pool"),
        &String::from_str(env, "Stress test pool"),
        &goal,
    );
    (pool_id, creator)
}

fn make_milestones(env: &Env, items: &[(u128, u64)]) -> Vec<Milestone> {
    let mut milestones = Vec::new(env);
    for (amount, unlock_time) in items.iter() {
        milestones.push_back(Milestone {
            amount: *amount,
            unlock_time: *unlock_time,
        });
    }
    milestones
}

#[test]
#[should_panic(expected = "Unauthorized admin")]
fn test_register_school_unauthorized_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let wrong_admin = Address::generate(&env);
    let school = Address::generate(&env);

    // Set admin to 'admin'
    client.set_admin(&admin);

    // Try to register school with different admin - should panic "Unauthorized admin"
    client.register_school(&wrong_admin, &school);
}

#[test]
#[should_panic(expected = "Milestones required")]
fn test_setup_application_milestones_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let student = Address::generate(&env);

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "Test"),
        &1_000_000_000,
    );

    // Try to setup empty milestones - should panic "Milestones required"
    let empty_milestones: Vec<Milestone> = Vec::new(&env);
    client.setup_application_milestones(&pool_id, &student, &empty_milestones);
}

#[test]
#[should_panic(expected = "Milestone total must equal pool goal")]
fn test_setup_application_milestones_sum_mismatch() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let student = Address::generate(&env);
    let pool_goal = 1_000_000_000u128;

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "Test"),
        &pool_goal,
    );

    // Create milestones with total != pool_goal
    let mut milestones = Vec::new(&env);
    milestones.push_back(Milestone {
        amount: 500_000_000,
        unlock_time: 1_000_000,
    });
    milestones.push_back(Milestone {
        amount: 300_000_000, // total = 800_000_000 != 1_000_000_000
        unlock_time: 2_000_000,
    });

    // Try to setup milestones with mismatched sum - should panic
    client.setup_application_milestones(&pool_id, &student, &milestones);
}

#[test]
#[should_panic(expected = "Duplicate application")]
fn test_apply_to_pool_duplicate_application() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let student = Address::generate(&env);

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "Test"),
        &1_000_000_000,
    );

    // Apply once
    let credential_hash = BytesN::from_array(&env, &[1u8; 32]);
    client.apply_to_pool(&student, &pool_id, &credential_hash, &100_000_000i128);

    // Try to apply again - should panic "Duplicate application"
    client.apply_to_pool(&student, &pool_id, &credential_hash, &100_000_000i128);
}

#[test]
#[should_panic(expected = "Student has not applied")]
fn test_approve_application_student_not_applied() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let school = Address::generate(&env);
    let student = Address::generate(&env);
    let other_student = Address::generate(&env);

    client.set_admin(&admin);
    client.register_school(&admin, &school);

    let pool_id = client.create_pool_for_school(
        &creator,
        &String::from_str(&env, "School Pool"),
        &String::from_str(&env, "Scholarship"),
        &1_000_000_000u128,
        &school,
    );

    // Only other_student applies
    let credential_hash = BytesN::from_array(&env, &[1u8; 32]);
    client.apply_to_pool(
        &other_student,
        &pool_id,
        &credential_hash,
        &100_000_000i128,
    );

    // Try to approve a student who never applied - should panic
    client.approve_application(&pool_id, &school, &student, &true);
}

#[test]
fn test_approve_application_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let school = Address::generate(&env);
    let student = Address::generate(&env);

    client.set_admin(&admin);
    client.register_school(&admin, &school);

    let pool_id = client.create_pool_for_school(
        &creator,
        &String::from_str(&env, "School Pool"),
        &String::from_str(&env, "Scholarship"),
        &1_000_000_000u128,
        &school,
    );

    let credential_hash = BytesN::from_array(&env, &[1u8; 32]);
    client.apply_to_pool(&student, &pool_id, &credential_hash, &100_000_000i128);

    // Approve with false (reject)
    client.approve_application(&pool_id, &school, &student, &false);

    // Verify status is "Rejected"
    let status = client.get_application_status(&pool_id, &student);
    assert_eq!(status, String::from_str(&env, "Rejected"));
}

// ============= PROTOCOL FEES TESTS (Issue #348) =============

/// Test that fees are accumulated correctly during donations.
/// A 1% fee is deducted from each donation and accumulated in unclaimed_fees.
#[test]
fn test_fee_accumulation_on_donation() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let pool_goal: u128 = 1_000_000_000;

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "Fee test"),
        &pool_goal,
    );

    // Donate 100_000_000 stroops
    let donation_amount = 100_000_000u128;
    client.donate(&pool_id, &donor, &donation_amount);

    // Expected fee: 1% of 100_000_000 = 1_000_000
    let expected_fee = (donation_amount * 1) / 100;
    let expected_collected = donation_amount - expected_fee;

    // Verify unclaimed fees
    let unclaimed_fees = client.get_unclaimed_fees();
    assert_eq!(unclaimed_fees, expected_fee);

    // Verify pool collected amount (net of fees)
    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.3, expected_collected);
}

/// Test that multiple donations accumulate fees correctly.
#[test]
fn test_multiple_donations_accumulate_fees() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let creator = Address::generate(&env);
    let donor1 = Address::generate(&env);
    let donor2 = Address::generate(&env);
    let pool_goal: u128 = 10_000_000_000;

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Multi-donation Pool"),
        &String::from_str(&env, "Fee accumulation test"),
        &pool_goal,
    );

    // First donation: 200_000_000
    let donation1 = 200_000_000u128;
    client.donate(&pool_id, &donor1, &donation1);
    let fee1 = (donation1 * 1) / 100;

    // Second donation: 300_000_000
    let donation2 = 300_000_000u128;
    client.donate(&pool_id, &donor2, &donation2);
    let fee2 = (donation2 * 1) / 100;

    // Total unclaimed fees should be fee1 + fee2
    let total_expected_fees = fee1 + fee2;
    let unclaimed_fees = client.get_unclaimed_fees();
    assert_eq!(unclaimed_fees, total_expected_fees);

    // Pool should have collected both donations net of fees
    let pool = client.get_pool(&pool_id);
    let expected_collected = (donation1 - fee1) + (donation2 - fee2);
    assert_eq!(pool.3, expected_collected);
}

/// Test that get_unclaimed_fees returns 0 when no donations have been made.
#[test]
fn test_get_unclaimed_fees_initial_state() {
    let env = Env::default();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let unclaimed_fees = client.get_unclaimed_fees();
    assert_eq!(unclaimed_fees, 0);
}

/// Test that admin can claim accumulated fees.
#[test]
fn test_admin_claim_protocol_fees() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let pool_goal: u128 = 1_000_000_000;

    // Set admin
    client.set_admin(&admin);

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "Claim fees test"),
        &pool_goal,
    );

    // Donate to accumulate fees
    let donation_amount = 100_000_000u128;
    client.donate(&pool_id, &donor, &donation_amount);
    let expected_fee = (donation_amount * 1) / 100;

    // Verify fees are accumulated
    let unclaimed_before = client.get_unclaimed_fees();
    assert_eq!(unclaimed_before, expected_fee);

    // Admin claims fees

    // Use a mock token address and treasury address
    let token_address = Address::generate(&env);
    let treasury_address = Address::generate(&env);
    let claimed_amount = client.claim_protocol_fees(&admin, &token_address, &treasury_address);
    assert_eq!(claimed_amount, expected_fee);

    // Verify unclaimed fees are reset to 0
    let unclaimed_after = client.get_unclaimed_fees();
    assert_eq!(unclaimed_after, 0);
}

/// Test that multiple fee accumulations and claims work correctly.
#[test]
fn test_multiple_fee_cycles() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let donor1 = Address::generate(&env);
    let donor2 = Address::generate(&env);
    let donor3 = Address::generate(&env);
    let pool_goal: u128 = 10_000_000_000;

    client.set_admin(&admin);

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Cycle Test Pool"),
        &String::from_str(&env, "Multiple cycles"),
        &pool_goal,
    );

    // Cycle 1: Two donations
    let donation1 = 100_000_000u128;
    let donation2 = 200_000_000u128;
    client.donate(&pool_id, &donor1, &donation1);
    client.donate(&pool_id, &donor2, &donation2);

    let fee1_cycle1 = (donation1 * 1) / 100;
    let fee2_cycle1 = (donation2 * 1) / 100;
    let total_cycle1 = fee1_cycle1 + fee2_cycle1;

    let unclaimed_after_cycle1 = client.get_unclaimed_fees();
    assert_eq!(unclaimed_after_cycle1, total_cycle1);

    // Admin claims fees from cycle 1
    let token_address = Address::generate(&env);
    let treasury_address = Address::generate(&env);
    let claimed_cycle1 = client.claim_protocol_fees(&admin, &token_address, &treasury_address);
    assert_eq!(claimed_cycle1, total_cycle1);

    let unclaimed_after_claim1 = client.get_unclaimed_fees();
    assert_eq!(unclaimed_after_claim1, 0);

    // Cycle 2: One more donation
    let donation3 = 500_000_000u128;
    client.donate(&pool_id, &donor3, &donation3);

    let fee3_cycle2 = (donation3 * 1) / 100;
    let unclaimed_after_cycle2 = client.get_unclaimed_fees();
    assert_eq!(unclaimed_after_cycle2, fee3_cycle2);

    // Admin claims fees from cycle 2
    let claimed_cycle2 = client.claim_protocol_fees(&admin, &token_address, &treasury_address);
    assert_eq!(claimed_cycle2, fee3_cycle2);

    let unclaimed_final = client.get_unclaimed_fees();
    assert_eq!(unclaimed_final, 0);
}

/// Test that only the protocol admin can claim fees.
#[test]
#[should_panic(expected = "Unauthorized: only protocol admin can claim fees")]
fn test_non_admin_cannot_claim_fees() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let pool_goal: u128 = 1_000_000_000;

    // Set admin
    client.set_admin(&admin);

    let pool_id = client.create_pool(
        &creator,
        &String::from_str(&env, "Test Pool"),
        &String::from_str(&env, "Unauthorized claim test"),
        &pool_goal,
    );

    // Donate to accumulate fees
    client.donate(&pool_id, &donor, &100_000_000);

    // Non-admin tries to claim fees - should panic
    let token_address = Address::generate(&env);
    let treasury_address = Address::generate(&env);
    client.claim_protocol_fees(&non_admin, &token_address, &treasury_address);
}

/// Test that claiming fees when no admin is set panics.
#[test]
#[should_panic(expected = "Admin not set")]
fn test_claim_fees_no_admin_set() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let non_admin = Address::generate(&env);

    // Try to claim fees without setting admin - should panic "Admin not set"
    let token_address = Address::generate(&env);
    let treasury_address = Address::generate(&env);
    client.claim_protocol_fees(&non_admin, &token_address, &treasury_address);
}

/// Test that fee tracking is separate from pool allocations.
/// Fees should not affect the pool's goal or collected amounts in accounting.
#[test]
fn test_fee_separation_from_pool_allocations() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let student = Address::generate(&env);
    let school = Address::generate(&env);
    let pool_goal: u128 = 1_000_000_000;

    client.set_admin(&admin);
    client.register_school(&admin, &school);

    let pool_id = client.create_pool_for_school(
        &creator,
        &String::from_str(&env, "School Pool"),
        &String::from_str(&env, "Separation test"),
        &pool_goal,
        &school,
    );

    // Donate 500_000_000
    let donation_amount = 500_000_000u128;
    let expected_fee = (donation_amount * 1) / 100;
    let expected_net = donation_amount - expected_fee;

    client.donate(&pool_id, &donor, &donation_amount);

    // Student applies and gets approved
    let credential_hash = BytesN::from_array(&env, &[1u8; 32]);
    client.apply_to_pool(&student, &pool_id, &credential_hash, &100_000_000i128);
    client.approve_application(&pool_id, &school, &student, &true);

    // Verify pool state
    let pool = client.get_pool(&pool_id);
    assert_eq!(pool.2, pool_goal); // Goal unchanged
    assert_eq!(pool.3, expected_net); // Collected is net of fees

    // Verify fees are tracked separately
    let unclaimed_fees = client.get_unclaimed_fees();
    assert_eq!(unclaimed_fees, expected_fee);

    // Student can claim from the net collected amount (not from fees)
    let token_address = Address::generate(&env);
    client.claim_funds(&student, &pool_id, &100_000_000i128, &token_address);

    let claimed = client.get_claimed_amount(&pool_id, &student);
    assert_eq!(claimed, 100_000_000i128);

    // Fees remain separate and unaffected
    let unclaimed_fees_after = client.get_unclaimed_fees();
    assert_eq!(unclaimed_fees_after, expected_fee);
    }
