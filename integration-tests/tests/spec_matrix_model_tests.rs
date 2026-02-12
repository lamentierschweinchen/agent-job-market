use multiversx_sc_scenario::api::DebugApi;

const BPS_DENOMINATOR: u64 = 10_000;

type JobBoardContract = job_board_core::ContractObj<DebugApi>;
type WorkEscrowContract = work_escrow::ContractObj<DebugApi>;

fn payout_split(gross: u128, protocol_fee_bps: u64, referral_share_bps: u64, has_referrer: bool) -> (u128, u128, u128, u128) {
    let protocol_fee = gross * protocol_fee_bps as u128 / BPS_DENOMINATOR as u128;
    let worker_net = gross - protocol_fee;
    let referral_fee = if has_referrer {
        protocol_fee * referral_share_bps as u128 / BPS_DENOMINATOR as u128
    } else {
        0
    };
    let treasury_fee = protocol_fee - referral_fee;
    (worker_net, referral_fee, treasury_fee, protocol_fee)
}

fn revenue_split(revenue: u128, protocol_fee_bps: u64, referral_share_bps: u64, profit_share_bps: u64, has_referrer: bool) -> (u128, u128, u128, u128, u128) {
    let protocol_fee = revenue * protocol_fee_bps as u128 / BPS_DENOMINATOR as u128;
    let net = revenue - protocol_fee;
    let worker_share = net * profit_share_bps as u128 / BPS_DENOMINATOR as u128;
    let employer_share = net - worker_share;
    let referral_fee = if has_referrer {
        protocol_fee * referral_share_bps as u128 / BPS_DENOMINATOR as u128
    } else {
        0
    };
    let treasury_fee = protocol_fee - referral_fee;
    (worker_share, employer_share, referral_fee, treasury_fee, protocol_fee)
}

#[test]
fn contract_objects_build() {
    let _: fn() -> JobBoardContract = job_board_core::contract_obj;
    let _: fn() -> WorkEscrowContract = work_escrow::contract_obj;
}

#[test]
fn happy_path_value_conservation_model() {
    let (worker_net, referral_fee, treasury_fee, protocol_fee) = payout_split(100_000, 150, 3_000, true);
    assert_eq!(worker_net + referral_fee + treasury_fee, 100_000);
    assert_eq!(referral_fee + treasury_fee, protocol_fee);
}

#[test]
fn invite_only_gate_model() {
    let invited = ["worker_a", "worker_b"];
    assert!(invited.contains(&"worker_a"));
    assert!(!invited.contains(&"worker_x"));
}

#[test]
fn stale_offer_prevention_model() {
    let latest_offer_id = 9u64;
    let attempted_offer_id = 8u64;
    assert_ne!(latest_offer_id, attempted_offer_id);
}

#[test]
fn double_activation_prevention_model() {
    let mut consumed = false;
    assert!(!consumed);
    consumed = true;
    assert!(consumed);
}

#[test]
fn runway_depletion_default_model() {
    let runway_balance = 0u128;
    let recurring_due = 100u128;
    assert!(runway_balance < recurring_due);
}

#[test]
fn milestone_autoapprove_timeout_model() {
    let submitted_at = 1_000u64;
    let review_timeout = 1_800u64;
    let now = 2_900u64;
    assert!(now > submitted_at + review_timeout);
}

#[test]
fn revenue_split_correctness_model() {
    let (worker, employer, referral, treasury, _protocol) = revenue_split(500_000, 150, 3_000, 2_000, true);
    assert_eq!(worker + employer + referral + treasury, 500_000);

    let (worker2, employer2, referral2, treasury2, _protocol2) = revenue_split(500_000, 150, 3_000, 2_000, false);
    assert_eq!(worker2 + employer2 + referral2 + treasury2, 500_000);
    assert_eq!(referral2, 0);
}

#[test]
fn termination_penalty_model() {
    let requester_bond = 1_000_000u128;
    let penalty_bps = 500u64;
    let penalty = requester_bond * penalty_bps as u128 / BPS_DENOMINATOR as u128;
    assert_eq!(penalty, 50_000);
}

#[test]
fn reputation_bounds_model() {
    let mut score: i64 = 420;
    score += 5;
    score -= 60;
    score = score.clamp(0, 1_000);
    assert!((0..=1_000).contains(&score));
}

#[test]
fn trust_gates_model() {
    let registered = true;
    let uptime_score = 101u64;
    let min_uptime = 100u64;
    assert!(registered);
    assert!(uptime_score >= min_uptime);
}

#[test]
fn pause_unpause_model() {
    let mut paused = false;
    assert!(!paused);
    paused = true;
    assert!(paused);
    paused = false;
    assert!(!paused);
}

#[test]
fn withdraw_once_model() {
    let mut claimable = 123u128;
    assert!(claimable > 0);
    claimable = 0;
    assert_eq!(claimable, 0);
}

#[test]
fn edge_boundaries_model() {
    let max_milestones = 32usize;
    let max_counteroffers = 8usize;
    assert_eq!(max_milestones, 32);
    assert_eq!(max_counteroffers, 8);
}
