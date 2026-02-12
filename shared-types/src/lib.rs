#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const BPS_DENOMINATOR: u64 = 10_000;
pub const SCORE_MIN: u64 = 0;
pub const SCORE_MAX: u64 = 1_000;
pub const MAX_PAGE_SIZE: u64 = 100;
pub const MAX_METADATA_URI_LEN: usize = 512;
pub const MAX_TERMS_URI_LEN: usize = 512;
pub const MAX_REASON_URI_LEN: usize = 512;
pub const MAX_PROOF_URI_LEN: usize = 512;
pub const MAX_APPLICATION_URI_LEN: usize = 512;

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, PartialEq, Eq)]
pub enum JobVisibility {
    Public = 1,
    InviteOnly = 2,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Open,
    InNegotiation,
    Matched,
    Closed,
    Expired,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, PartialEq, Eq)]
pub enum OfferStatus {
    Proposed,
    Countered,
    Accepted,
    Rejected,
    Withdrawn,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, PartialEq, Eq)]
pub enum OfferParty {
    Employer,
    Worker,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, PartialEq, Eq)]
pub enum AgreementStatus {
    PendingFunding,
    Active,
    NoticePeriod,
    Terminated,
    Completed,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneState {
    Open,
    Submitted,
    Rejected,
    Paid,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneSettlementMode {
    Approved,
    AutoApproved,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, PartialEq, Eq)]
pub enum TerminationSide {
    Employer,
    Worker,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, PartialEq, Eq)]
pub enum TerminationReason {
    UnilateralEmployer,
    UnilateralWorker,
    EmployerDefault,
    WorkerDefault,
    NaturalCompletion,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, PartialEq, Eq)]
pub enum ReputationReason {
    Init,
    OnTimeRecurringPayment,
    RunwayLow,
    EmployerDefault,
    WorkerMilestoneSettled,
    WorkerDefault,
    UnilateralTerminate,
    Completion,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, PartialEq, Eq)]
pub enum JobCloseReason {
    Cancelled,
    Expired,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct Job<M: ManagedTypeApi> {
    pub id: u64,
    pub employer: ManagedAddress<M>,
    pub metadata_uri: ManagedBuffer<M>,
    pub visibility: JobVisibility,
    pub application_deadline_ts: u64,
    pub min_worker_uptime: u64,
    pub comp_mode_mask: u8,
    pub status: JobStatus,
    pub created_at: u64,
    pub accepted_offer_id: u64,
    pub application_count: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct Application<M: ManagedTypeApi> {
    pub id: u64,
    pub job_id: u64,
    pub applicant: ManagedAddress<M>,
    pub application_uri: ManagedBuffer<M>,
    pub created_at: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, Clone)]
pub struct MilestoneSpec<M: ManagedTypeApi> {
    pub id: u64,
    pub amount: BigUint<M>,
    pub due_ts: u64,
    pub review_timeout_seconds: u64,
    pub metadata_uri: ManagedBuffer<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct RecurringTerms<M: ManagedTypeApi> {
    pub amount_per_period: BigUint<M>,
    pub period_seconds: u64,
    pub total_periods: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct RevenueShareTerms {
    pub profit_share_bps: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct OfferTerms<M: ManagedTypeApi> {
    pub recurring: RecurringTerms<M>,
    pub revenue_share: RevenueShareTerms,
    pub employer_bond_required: BigUint<M>,
    pub worker_bond_required: BigUint<M>,
    pub milestones: ManagedVec<M, MilestoneSpec<M>>,
    pub terms_uri: ManagedBuffer<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct OfferTermsInput<M: ManagedTypeApi> {
    pub recurring: RecurringTerms<M>,
    pub revenue_share: RevenueShareTerms,
    pub employer_bond_required: BigUint<M>,
    pub worker_bond_required: BigUint<M>,
    pub milestones: ManagedVec<M, MilestoneSpec<M>>,
    pub terms_uri: ManagedBuffer<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct Offer<M: ManagedTypeApi> {
    pub id: u64,
    pub job_id: u64,
    pub application_id: u64,
    pub proposer: ManagedAddress<M>,
    pub counterparty: ManagedAddress<M>,
    pub party: OfferParty,
    pub parent_offer_id: u64,
    pub round_index: u64,
    pub terms: OfferTerms<M>,
    pub status: OfferStatus,
    pub created_at: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct AcceptedOfferSummary<M: ManagedTypeApi> {
    pub job_id: u64,
    pub offer_id: u64,
    pub employer: ManagedAddress<M>,
    pub worker: ManagedAddress<M>,
    pub terms: OfferTerms<M>,
    pub accepted_at: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct BoardStats {
    pub total_jobs: u64,
    pub open_jobs: u64,
    pub matched_jobs: u64,
    pub total_applications: u64,
    pub total_offers: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct JobBoardConfig<M: ManagedTypeApi> {
    pub owner: ManagedAddress<M>,
    pub bond_registry: ManagedAddress<M>,
    pub uptime: ManagedAddress<M>,
    pub min_uptime_score: u64,
    pub max_counteroffers_per_application: u64,
    pub max_invites_per_job: u64,
    pub paused: bool,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct Agreement<M: ManagedTypeApi> {
    pub id: u64,
    pub job_id: u64,
    pub offer_id: u64,
    pub employer: ManagedAddress<M>,
    pub worker: ManagedAddress<M>,
    pub referrer: ManagedAddress<M>,
    pub status: AgreementStatus,
    pub created_at: u64,
    pub activated_at: u64,
    pub notice_start_ts: u64,
    pub notice_end_ts: u64,
    pub requested_by_side: u8,
    pub default_side: u8,
    pub terms: AgreementTerms<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct AgreementTerms<M: ManagedTypeApi> {
    pub recurring: RecurringTermsEscrow<M>,
    pub revenue_share: RevenueShareTermsEscrow,
    pub employer_bond_required: BigUint<M>,
    pub worker_bond_required: BigUint<M>,
    pub milestone_count: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct RecurringTermsEscrow<M: ManagedTypeApi> {
    pub amount_per_period: BigUint<M>,
    pub period_seconds: u64,
    pub total_periods: u64,
    pub paid_periods: u64,
    pub next_pay_ts: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct RevenueShareTermsEscrow {
    pub profit_share_bps: u64,
    pub protocol_fee_bps_snapshot: u64,
    pub referral_share_bps_snapshot: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct FundingState<M: ManagedTypeApi> {
    pub runway_balance: BigUint<M>,
    pub employer_bond_locked: BigUint<M>,
    pub worker_bond_locked: BigUint<M>,
    pub reserved_recurring_minimum: BigUint<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct Milestone<M: ManagedTypeApi> {
    pub id: u64,
    pub agreement_id: u64,
    pub amount: BigUint<M>,
    pub due_ts: u64,
    pub review_timeout_seconds: u64,
    pub metadata_uri: ManagedBuffer<M>,
    pub state: MilestoneState,
    pub submitted_at: u64,
    pub review_deadline: u64,
    pub proof_uri: ManagedBuffer<M>,
    pub reason_uri: ManagedBuffer<M>,
    pub settlement_mode: u8,
    pub paid_at: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct ReputationSnapshot {
    pub score: u64,
    pub agreements_started: u64,
    pub agreements_completed: u64,
    pub defaults_as_employer: u64,
    pub defaults_as_worker: u64,
    pub on_time_recurring_payments: u64,
    pub milestones_settled: u64,
    pub terminations_initiated: u64,
    pub last_updated_ts: u64,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct AgreementFinancials<M: ManagedTypeApi> {
    pub funding: FundingState<M>,
    pub worker_claimable: BigUint<M>,
    pub employer_claimable: BigUint<M>,
    pub referrer_claimable: BigUint<M>,
    pub treasury_claimable: BigUint<M>,
    pub total_gross_paid: BigUint<M>,
    pub total_fees_paid: BigUint<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct ProtocolStats<M: ManagedTypeApi> {
    pub total_agreements: u64,
    pub active_agreements: u64,
    pub completed_agreements: u64,
    pub terminated_agreements: u64,
    pub total_gross_payouts: BigUint<M>,
    pub total_protocol_fees: BigUint<M>,
    pub total_revenue_deposited: BigUint<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct EscrowConfig<M: ManagedTypeApi> {
    pub owner: ManagedAddress<M>,
    pub job_board: ManagedAddress<M>,
    pub bond_registry: ManagedAddress<M>,
    pub uptime: ManagedAddress<M>,
    pub treasury: ManagedAddress<M>,
    pub min_uptime_score: u64,
    pub protocol_fee_bps: u64,
    pub referral_share_bps: u64,
    pub min_employer_bond: BigUint<M>,
    pub min_worker_bond: BigUint<M>,
    pub min_runway_periods: u64,
    pub default_notice_seconds: u64,
    pub termination_penalty_bps: u64,
    pub milestone_review_timeout_seconds: u64,
    pub max_milestones_per_agreement: u64,
    pub score_start: u64,
    pub paused: bool,
}
