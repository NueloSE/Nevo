#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Symbol, Vec};

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

        env.storage()
            .persistent()
            .set(&pool_id, &(creator.clone(), goal, 0u128, false));

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
        let _ = donor;
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

    /// Student applies to a school-linked pool.
    pub fn apply_to_pool(env: Env, pool_id: u32, student: Address, application_data: String) {
        student.require_auth();

        let _: (Address, u128, u128, bool) = env
            .storage()
            .persistent()
            .get::<_, (Address, u128, u128, bool)>(&pool_id)
            .expect("Pool not found");

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
            .set(&app_key, &(app_count, student.clone(), application_data));

        env.storage().persistent().set(&applicant_key, &true);
        env.storage().persistent().set(&count_key, &app_count);

        let pending = String::from_str(&env, "Pending");
        Self::set_application_status(env, pool_id, student, pending);
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

        let pool_data: (Address, u128, u128, bool) = env
            .storage()
            .persistent()
            .get::<_, (Address, u128, u128, bool)>(&pool_id)
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

        if sum != pool_data.1 {
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

    /// Claim funds for an approved application.
    pub fn claim_funds(
        env: Env,
        student: Address,
        pool_id: u32,
        claim_amount: i128,
        _token_address: Address,
    ) {
        student.require_auth();

        if claim_amount <= 0 {
            panic!("Claim amount must be positive");
        }

        let pool_data: (Address, u128, u128, bool) = env
            .storage()
            .persistent()
            .get::<_, (Address, u128, u128, bool)>(&pool_id)
            .expect("Pool not found");

        let status = Self::get_application_status(env.clone(), pool_id, student.clone());
        if status == String::from_str(&env, "") {
            panic!("Application status not found");
        }
        if status != String::from_str(&env, APPLICATION_STATUS_APPROVED) {
            panic!("Application is not approved");
        }

        let claimed_key = (
            Symbol::new(&env, CLAIMED_AMOUNT_PREFIX),
            pool_id,
            student.clone(),
        );
        let current_claimed: i128 = env
            .storage()
            .persistent()
            .get::<_, i128>(&claimed_key)
            .unwrap_or(0);
        let new_claimed = current_claimed + claim_amount;

        if new_claimed > pool_data.2 as i128 {
            panic!("Overdraw attempt");
        }

        env.storage().persistent().set(&claimed_key, &new_claimed);
    }
}

mod test;
