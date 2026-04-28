#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, String, Symbol, Vec, IntoVal, BytesN};

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
const MILESTONES_PREFIX: &str = "milestones";
const ADMIN_KEY: &str = "admin";
const SCHOOL_REG_PREFIX: &str = "school_reg";
const POOL_SCHOOL_PREFIX: &str = "pool_school";

// Application and claim tracking constants
const APPLICATION_STATUS_PREFIX: &str = "app_status";
const CLAIMED_AMOUNT_PREFIX: &str = "claimed_amount";
const APPLICATION_STATUS_APPROVED: &str = "Approved";
const APPLICATION_STATUS_REJECTED: &str = "Rejected";

// Protocol fees tracking constants
const UNCLAIMED_FEES_KEY: &str = "unclaimed_fees";
const FEE_PERCENTAGE: u128 = 1; // 1% fee on donations

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pool {
    pub sponsor: Address,
    pub goal: u128,
    pub collected: u128,
    pub is_closed: bool,
}
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Application {
    /// The total amount the student is approved to receive from this pool.
    pub approved_amount: i128,
    /// Running total of funds already disbursed to the student.
    /// Starts at 0; incremented on every successful partial claim.
    pub amount_claimed: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub amount: u128,
    pub unlock_time: u64,
}

#[contract]
pub struct Contract;

#[contractimpl]
impl Contract {
    /// Set the platform admin address.
    pub fn set_admin(env: Env, admin: Address) {
        admin.require_auth();
        let admin_key = Symbol::new(&env, ADMIN_KEY);
        env.storage().persistent().set(&admin_key, &admin);
    }

    /// Register a school by admin authorization.
    pub fn register_school(env: Env, admin: Address, school: Address) {
        admin.require_auth();

        let admin_key = Symbol::new(&env, ADMIN_KEY);
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get::<_, Address>(&admin_key)
            .expect("Admin not set");
        if stored_admin != admin {
            panic!("Unauthorized admin");
        }

        let school_key = (Symbol::new(&env, SCHOOL_REG_PREFIX), school);
        env.storage().persistent().set(&school_key, &true);
    }

    /// Check if a school has been registered.
    pub fn is_school_registered(env: Env, school: Address) -> bool {
        let school_key = (Symbol::new(&env, SCHOOL_REG_PREFIX), school);
        env.storage()
            .persistent()
            .get::<_, bool>(&school_key)
            .unwrap_or(false)
    }

    // ─── Pool Management ─────────────────────────────────────────────────────

    /// Create a new donation / sponsorship pool.
    pub fn create_pool(
        env: Env,
        creator: Address,
        title: String,
        description: String,
        goal: u128,
    ) -> u32 {
        let _ = (title, description);

        let pool_count_key = Symbol::new(&env, POOL_COUNT);
        let mut pool_count: u32 = env
            .storage()
            .persistent()
            .get::<_, u32>(&pool_count_key)
            .unwrap_or(0);

        let pool_id = pool_count + 1;
        pool_count = pool_id;

        // Legacy compatibility: keep old symbolic key constants reachable.
        let _ = (
            POOL_PREFIX,
            CREATOR_SUFFIX,
            GOAL_SUFFIX,
            COLLECTED_SUFFIX,
            CLOSED_SUFFIX,
        );

        let pool = Pool {
            sponsor: creator.clone(),
            goal,
            collected: 0u128,
            is_closed: false,
        };

        env.storage()
            .persistent()
            .set(&pool_id, &pool);

        env.storage().persistent().set(&pool_count_key, &pool_count);

        pool_id
    }

    /// Create a new sponsorship pool linked to a registered school.
    pub fn create_pool_for_school(
        env: Env,
        creator: Address,
        title: String,
        description: String,
        goal: u128,
        school: Address,
    ) -> u32 {
        creator.require_auth();

        if !Self::is_school_registered(env.clone(), school.clone()) {
            panic!("School is not registered");
        }

        let pool_id = Self::create_pool(env.clone(), creator, title, description, goal);
        let pool_school_key = (Symbol::new(&env, POOL_SCHOOL_PREFIX), pool_id);
        env.storage().persistent().set(&pool_school_key, &school);
        pool_id
    }

    /// Get the school linked to a pool.
    pub fn get_pool_school(env: Env, pool_id: u32) -> Address {
        let pool_school_key = (Symbol::new(&env, POOL_SCHOOL_PREFIX), pool_id);
        env.storage()
            .persistent()
            .get::<_, Address>(&pool_school_key)
            .expect("Pool school not set")
    }

    /// Donate to an existing pool.
    pub fn donate(env: Env, pool_id: u32, donor: Address, amount: u128) {
        let mut pool: Pool = env
            .storage()
            .persistent()
            .get::<_, Pool>(&pool_id)
            .expect("Pool not found");

        if pool.is_closed {
            panic!("Pool is closed");
        }

        // Calculate protocol fee (1% of donation)
        let fee = (amount * FEE_PERCENTAGE) / 100;
        let net_donation = amount - fee;

        // Accumulate fees
        let fees_key = Symbol::new(&env, UNCLAIMED_FEES_KEY);
        let current_fees: u128 = env
            .storage()
            .persistent()
            .get::<_, u128>(&fees_key)
            .unwrap_or(0);
        let new_fees = current_fees + fee;
        env.storage().persistent().set(&fees_key, &new_fees);

        // Update pool collected net of fees
        pool.collected = pool.collected + net_donation;
        env.storage().persistent().set(&pool_id, &pool);

        let donor_index: u32 = env
            .storage()
            .persistent()
            .get::<_, u32>(&(pool_id, "d_count"))
            .unwrap_or(0);
        let _ = donor;
        env.storage()
            .persistent()
            .set(&(pool_id, "d_count"), &(donor_index + 1));
    }

    /// Get pool information as a tuple (id, creator, goal, collected, is_closed).
    pub fn get_pool(env: Env, pool_id: u32) -> (u32, Address, u128, u128, bool) {
        let pool: Pool = env
            .storage()
            .persistent()
            .get::<_, Pool>(&pool_id)
            .expect("Pool not found");

        (pool_id, pool.sponsor, pool.goal, pool.collected, pool.is_closed)
    }

    /// Close a donation pool.
    pub fn close_pool(env: Env, pool_id: u32) {
        let pool: Pool = env
            .storage()
            .persistent()
            .get::<_, Pool>(&pool_id)
            .expect("Pool not found");

        // Only the creator can close the pool
        pool.sponsor.require_auth();

        // Mark the pool as closed and persist
        let mut updated_pool = pool.clone();
        updated_pool.is_closed = true;
        env.storage()
            .persistent()
            .set(&pool_id, &updated_pool);
    }

    /// Get the total number of pools.
    pub fn get_pool_count(env: Env) -> u32 {
        let pool_count_key = Symbol::new(&env, POOL_COUNT);
        env.storage()
            .persistent()
            .get::<_, u32>(&pool_count_key)
            .unwrap_or(0)
    }

    /// Student applies to a school-linked pool.
    pub fn apply_to_pool(
        env: Env,
        student: Address,
        pool_id: u32,
        credential_hash: BytesN<32>,
        requested_amount: i128,
    ) -> u32 {
        student.require_auth();

        let pool: Pool = env
            .storage()
            .persistent()
            .get::<_, Pool>(&pool_id)
            .expect("Pool not found");

        if pool.is_closed {
            panic!("Pool is inactive");
        }

        let applicant_key = (
            Symbol::new(&env, APPLICANT_PREFIX),
            pool_id,
            student.clone(),
        );
        if env.storage().persistent().has(&applicant_key) {
            panic!("Duplicate application");
        }

        let count_key = (Symbol::new(&env, APPLICATION_COUNT_PREFIX), pool_id);
        let mut app_count: u32 = env
            .storage()
            .persistent()
            .get::<_, u32>(&count_key)
            .unwrap_or(0);
        app_count += 1;

        let app_key = (Symbol::new(&env, APPLICATION_PREFIX), pool_id, app_count);
        env.storage()
            .persistent()
            .set(&app_key, &(student.clone(), credential_hash.clone(), requested_amount));

        env.storage().persistent().set(&applicant_key, &true);
        env.storage().persistent().set(&count_key, &app_count);

        let pending = String::from_str(&env, "Pending");
        Self::set_application_status(env, pool_id, student, pending);
        
        app_count
    }

    /// School approves or rejects a student's application.
    pub fn approve_application(
        env: Env,
        pool_id: u32,
        school: Address,
        student: Address,
        approved: bool,
    ) {
        school.require_auth();

        let linked_school = Self::get_pool_school(env.clone(), pool_id);
        if linked_school != school {
            panic!("Only linked school can approve");
        }

        let applicant_key = (
            Symbol::new(&env, APPLICANT_PREFIX),
            pool_id,
            student.clone(),
        );
        if !env.storage().persistent().has(&applicant_key) {
            panic!("Student has not applied");
        }

        let status = if approved {
            String::from_str(&env, APPLICATION_STATUS_APPROVED)
        } else {
            String::from_str(&env, APPLICATION_STATUS_REJECTED)
        };
        Self::set_application_status(env, pool_id, student, status);
    }

    /// Set application milestones and enforce sum(amounts) == pool goal.
    pub fn setup_application_milestones(
        env: Env,
        pool_id: u32,
        student: Address,
        milestones: Vec<Milestone>,
    ) {
        student.require_auth();

        let pool_data: Pool = env
            .storage()
            .persistent()
            .get::<_, Pool>(&pool_id)
            .expect("Pool not found");

        if milestones.is_empty() {
            panic!("Milestones required");
        }

        let mut sum: u128 = 0;
        for i in 0..milestones.len() {
            sum = sum
                .checked_add(milestones.get(i).unwrap().amount)
                .expect("Milestone amount overflow");
        }

        if sum != pool_data.goal {
            panic!("Milestone total must equal pool goal");
        }

        let milestones_key = (Symbol::new(&env, MILESTONES_PREFIX), pool_id, student);
        env.storage().persistent().set(&milestones_key, &milestones);
    }

    /// Get student milestones for a pool.
    pub fn get_milestones(env: Env, pool_id: u32, student: Address) -> Vec<Milestone> {
        let milestones_key = (Symbol::new(&env, MILESTONES_PREFIX), pool_id, student);
        env.storage()
            .persistent()
            .get::<_, Vec<Milestone>>(&milestones_key)
            .unwrap_or(Vec::new(&env))
    }

    /// Set application status for a student in a pool.
    pub fn set_application_status(env: Env, pool_id: u32, student: Address, status: String) {
        let status_key = (
            Symbol::new(&env, APPLICATION_STATUS_PREFIX),
            pool_id,
            student.clone(),
        );
        env.storage().persistent().set(&status_key, &status);
    }

    /// Get application status for a student in a pool.
    pub fn get_application_status(env: Env, pool_id: u32, student: Address) -> String {
        let status_key = (
            Symbol::new(&env, APPLICATION_STATUS_PREFIX),
            pool_id,
            student.clone(),
        );
        env.storage()
            .persistent()
            .get::<_, String>(&status_key)
            .unwrap_or(String::from_str(&env, ""))
    }

    /// Get claimed amount for a student in a pool.
    pub fn get_claimed_amount(env: Env, pool_id: u32, student: Address) -> i128 {
        let claimed_key = (
            Symbol::new(&env, CLAIMED_AMOUNT_PREFIX),
            pool_id,
            student.clone(),
        );
        env.storage()
            .persistent()
            .get::<_, i128>(&claimed_key)
            .unwrap_or(0)
    }

    /// Get the full Application record for a student in a pool.
    /// Returns `None` if the student has not yet made any claim.
    pub fn get_claim_application(env: Env, pool_id: u32, student: Address) -> Option<Application> {
        let app_key = (CLAIMED_AMOUNT_PREFIX, pool_id, student.clone());
        env.storage().persistent().get::<_, Application>(&app_key)
    }

    /// Get a stored application tuple by its id: (id, student, application_data)
    pub fn get_application(env: Env, pool_id: u32, application_id: u32) -> (Address, BytesN<32>, i128) {
        let app_key = (Symbol::new(&env, APPLICATION_PREFIX), pool_id, application_id);
        env.storage()
            .persistent()
            .get::<_, (Address, BytesN<32>, i128)>(&app_key)
            .expect("Application not found")
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
        let status_key = (
            Symbol::new(&env, APPLICATION_STATUS_PREFIX),
            pool_id,
            student.clone(),
        );
        let status: String = env
            .storage()
            .persistent()
            .get::<_, String>(&status_key)
            .unwrap_or_else(|| panic!("Application status not found"));

        if status != String::from_str(&env, APPLICATION_STATUS_APPROVED) {
            panic!("Application is not approved");
        }

        // Load pool to check available collected funds
        let pool_data: Pool = env
            .storage()
            .persistent()
            .get::<_, Pool>(&pool_id)
            .expect("Pool not found");

        let collected = pool_data.collected as i128;

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

    /// Get the total unclaimed protocol fees.
    pub fn get_unclaimed_fees(env: Env) -> u128 {
        let fees_key = Symbol::new(&env, UNCLAIMED_FEES_KEY);
        env.storage()
            .persistent()
            .get::<_, u128>(&fees_key)
            .unwrap_or(0)
    }

    /// Claim accumulated protocol fees as admin and transfer to treasury.
    /// This function requires admin authorization and resets the unclaimed fees to zero.
    ///
    /// `token_address` - The token contract address to transfer fees from this contract to the treasury.
    /// `treasury_address` - The recipient address for protocol fees.
    pub fn claim_protocol_fees(env: Env, admin: Address, token_address: Address, treasury_address: Address) -> u128 {
        admin.require_auth();

        // Verify caller is the protocol admin
        let admin_key = Symbol::new(&env, ADMIN_KEY);
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get::<_, Address>(&admin_key)
            .expect("Admin not set");
        if stored_admin != admin {
            panic!("Unauthorized: only protocol admin can claim fees");
        }

        // Get current accumulated fees
        let fees_key = Symbol::new(&env, UNCLAIMED_FEES_KEY);
        let unclaimed_fees: u128 = env
            .storage()
            .persistent()
            .get::<_, u128>(&fees_key)
            .unwrap_or(0);

        if unclaimed_fees == 0 {
            return 0;
        }

        // Call token contract to transfer fees to treasury
        // This assumes the contract holds the protocol fees in the token contract already
        // and that the token contract follows the Soroban token interface
        let transfer_fn = Symbol::short("transfer");
        let _: () = env.invoke_contract(
            &token_address,
            &transfer_fn,
            (
                env.current_contract_address(), // from: this contract
                treasury_address.clone(),       // to: treasury
                unclaimed_fees as i128         // amount
            ).into_val(&env),
        );

        // Reset unclaimed fees to zero
        env.storage().persistent().set(&fees_key, &0u128);

        unclaimed_fees
    }
}

mod test;
