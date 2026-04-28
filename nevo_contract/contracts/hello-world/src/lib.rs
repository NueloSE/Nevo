#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, String, Symbol};

// Storage key constants
const POOL_COUNT: &str = "pool_count";
const POOL_PREFIX: &str = "p";
const CREATOR_SUFFIX: &str = "_creator";
const GOAL_SUFFIX: &str = "_goal";
const COLLECTED_SUFFIX: &str = "_collected";
const CLOSED_SUFFIX: &str = "_closed";
const APPLICATION_COUNT_PREFIX: &str = "a_count_";
const APPLICATION_PREFIX: &str = "a_";
const APPLICANT_PREFIX: &str = "ap_";

// Application and claim tracking constants
const APPLICATION_STATUS_PREFIX: &str = "app_status";
const CLAIMED_AMOUNT_PREFIX: &str = "claimed_amount";
const APPLICATION_STATUS_APPROVED: &str = "Approved";
const APPLICATION_STATUS_REJECTED: &str = "Rejected";

/// Tracks a student's approved funding and how much has been streamed so far.
///
/// `amount_claimed` starts at zero and increments with each partial withdrawal,
/// allowing the contract to enforce the invariant:
///   amount_claimed + new_claim <= approved_amount
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Application {
    /// The total amount the student is approved to receive from this pool.
    pub approved_amount: i128,
    /// Running total of funds already disbursed to the student.
    /// Starts at 0; incremented on every successful partial claim.
    pub amount_claimed: i128,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    // ─── Pool Management ─────────────────────────────────────────────────────

    /// Create a new donation / sponsorship pool.
    pub fn create_pool(
        env: Env,
        creator: Address,
        title: String,
        description: String,
        goal: u128,
    ) -> u32 {
        // creator.require_auth();  // TODO: Enable auth validation in production

        let pool_count_key = Symbol::new(&env, POOL_COUNT);
        let mut pool_count: u32 = env
            .storage()
            .persistent()
            .get::<_, u32>(&pool_count_key)
            .unwrap_or(0);

        let pool_id = pool_count + 1;
        pool_count = pool_id;

        // Store pool data - using numeric pool ID as key
        let pool_key = pool_id;

        env.storage()
            .persistent()
            .set(&pool_id, &(creator.clone(), goal, 0u128, false));

        env.storage().persistent().set(&pool_count_key, &pool_count);

        pool_id
    }

    /// Donate to an existing pool.
    pub fn donate(env: Env, pool_id: u32, donor: Address, amount: u128) {
        // donor.require_auth();  // TODO: Enable auth validation in production

        let pool_data: (Address, u128, u128, bool) = env
            .storage()
            .persistent()
            .get::<_, (Address, u128, u128, bool)>(&pool_id)
            .expect("Pool not found");

        if pool_data.3 {
            panic!("Pool is closed");
        }

        let new_collected = pool_data.2 + amount;
        env.storage().persistent().set(
            &pool_id,
            &(pool_data.0.clone(), pool_data.1, new_collected, pool_data.3),
        );

        let donor_index: u32 = env
            .storage()
            .persistent()
            .get::<_, u32>(&(pool_id, "d_count"))
            .unwrap_or(0);

        env.storage()
            .persistent()
            .set(&(pool_id, "d_count"), &(donor_index + 1));
    }

    /// Get pool information as a tuple (id, creator, goal, collected, is_closed).
    pub fn get_pool(env: Env, pool_id: u32) -> (u32, Address, u128, u128, bool) {
        let pool_data: (Address, u128, u128, bool) = env
            .storage()
            .persistent()
            .get::<_, (Address, u128, u128, bool)>(&pool_id)
            .expect("Pool not found");

        (pool_id, pool_data.0, pool_data.1, pool_data.2, pool_data.3)
    }

    /// Close a donation pool.
    pub fn close_pool(env: Env, pool_id: u32) {
        let pool_data: (Address, u128, u128, bool) = env
            .storage()
            .persistent()
            .get::<_, (Address, u128, u128, bool)>(&pool_id)
            .expect("Pool not found");

        // pool_data.0.require_auth();  // TODO: Enable auth validation in production

        env.storage()
            .persistent()
            .set(&pool_id, &(pool_data.0, pool_data.1, pool_data.2, true));
    }

    /// Get the total number of pools.
    pub fn get_pool_count(env: Env) -> u32 {
        let pool_count_key = Symbol::new(&env, POOL_COUNT);
        env.storage()
            .persistent()
            .get::<_, u32>(&pool_count_key)
            .unwrap_or(0)
    }

    /// Set application status for a student in a pool (helper for testing and admin)
    pub fn set_application_status(env: Env, pool_id: u32, student: Address, status: String) {
        let status_key = (APPLICATION_STATUS_PREFIX, pool_id, student.clone());
        env.storage().persistent().set(&status_key, &status);
    }

    /// Get application status for a student in a pool
    pub fn get_application_status(env: Env, pool_id: u32, student: Address) -> String {
        let status_key = (APPLICATION_STATUS_PREFIX, pool_id, student.clone());
        env.storage()
            .persistent()
            .get::<_, String>(&status_key)
            .unwrap_or(String::from_str(&env, ""))
    }

    /// Get claimed amount for a student in a pool
    pub fn get_claimed_amount(env: Env, pool_id: u32, student: Address) -> i128 {
        let claimed_key = (CLAIMED_AMOUNT_PREFIX, pool_id, student.clone());
        env.storage()
            .persistent()
            .get::<_, i128>(&claimed_key)
            .unwrap_or(0)
    }

    /// Get the full Application record for a student in a pool.
    /// Returns `None` if the student has not yet made any claim.
    pub fn get_application(env: Env, pool_id: u32, student: Address) -> Option<Application> {
        let app_key = (CLAIMED_AMOUNT_PREFIX, pool_id, student.clone());
        env.storage().persistent().get::<_, Application>(&app_key)
    }

    /// Claim funds: allows an approved student to receive a partial or full
    /// disbursement from a pool.
    ///
    /// Uses `Application` to persist `amount_claimed` across calls, enabling
    /// streamed / milestone-based withdrawals where the student draws down
    /// their approved allocation incrementally.
    ///
    /// # Arguments
    /// * `env`           - The contract environment
    /// * `student`       - The student address receiving funds (must authorize)
    /// * `pool_id`       - The ID of the pool to claim from
    /// * `claim_amount`  - The amount to claim this call (must be > 0)
    /// * `token_address` - The token used for the transfer
    ///
    /// # Panics
    /// - `"Claim amount must be positive"` if `claim_amount <= 0`
    /// - `"Application status not found"` if no status has been set
    /// - `"Application is not approved"` if status != "Approved"
    /// - `"Overdraw attempt"` if `amount_claimed + claim_amount > collected`
    pub fn claim_funds(
        env: Env,
        student: Address,
        pool_id: u32,
        claim_amount: i128,
        token_address: Address,
    ) {
        student.require_auth();

        if claim_amount <= 0 {
            panic!("Claim amount must be positive");
        }

        // Verify application is approved
        let status_key = (APPLICATION_STATUS_PREFIX, pool_id, student.clone());
        let status: String = env
            .storage()
            .persistent()
            .get::<_, String>(&status_key)
            .unwrap_or_else(|| panic!("Application status not found"));

        if status != String::from_str(&env, APPLICATION_STATUS_APPROVED) {
            panic!("Application is not approved");
        }

        // Load pool to check available collected funds
        let pool_data: (Address, u128, u128, bool) = env
            .storage()
            .persistent()
            .get::<_, (Address, u128, u128, bool)>(&pool_id)
            .expect("Pool not found");

        let collected = pool_data.2 as i128;

        // Load or initialise the Application record for this student
        let app_key = (CLAIMED_AMOUNT_PREFIX, pool_id, student.clone());
        let mut application: Application = env
            .storage()
            .persistent()
            .get::<_, Application>(&app_key)
            .unwrap_or(Application {
                approved_amount: collected,
                amount_claimed: 0,
            });

        // Enforce the partial-payment invariant
        if application.amount_claimed + claim_amount > collected {
            panic!("Overdraw attempt");
        }

        // Disburse tokens to the student
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(&env.current_contract_address(), &student, &claim_amount);

        // Persist the updated running total
        application.amount_claimed += claim_amount;
        env.storage().persistent().set(&app_key, &application);
    }
}

mod test;
