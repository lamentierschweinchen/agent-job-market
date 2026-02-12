#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod bond_registry_proxy;
mod job_board_proxy;
mod uptime_proxy;

use bond_registry_proxy::BondRegistryProxy;
use job_board_proxy::JobBoardProxy;
use shared_types::{
    AcceptedOfferSummary, Agreement, AgreementFinancials, AgreementStatus, AgreementTerms,
    EscrowConfig, FundingState, Milestone, MilestoneSettlementMode, MilestoneState,
    ProtocolStats, RecurringTermsEscrow, ReputationReason, ReputationSnapshot,
    RevenueShareTermsEscrow, TerminationReason, TerminationSide, BPS_DENOMINATOR,
    MAX_PROOF_URI_LEN, MAX_REASON_URI_LEN, SCORE_MAX,
};
use uptime_proxy::UptimeProxy;

pub const ERR_PAUSED: &str = "ERR_PAUSED";
pub const ERR_UNAUTHORIZED: &str = "ERR_UNAUTHORIZED";
pub const ERR_INVALID_STATE: &str = "ERR_INVALID_STATE";
pub const ERR_NOT_REGISTERED: &str = "ERR_NOT_REGISTERED";
pub const ERR_LOW_UPTIME: &str = "ERR_LOW_UPTIME";
pub const ERR_INVALID_BPS: &str = "ERR_INVALID_BPS";
pub const ERR_INVALID_AMOUNT: &str = "ERR_INVALID_AMOUNT";
pub const ERR_INVALID_DEADLINE: &str = "ERR_INVALID_DEADLINE";
pub const ERR_OFFER_NOT_ACCEPTED: &str = "ERR_OFFER_NOT_ACCEPTED";
pub const ERR_OFFER_CONSUMED: &str = "ERR_OFFER_CONSUMED";
pub const ERR_INSUFFICIENT_RUNWAY: &str = "ERR_INSUFFICIENT_RUNWAY";
pub const ERR_MILESTONE_STATE: &str = "ERR_MILESTONE_STATE";
pub const ERR_TIMEOUT_NOT_REACHED: &str = "ERR_TIMEOUT_NOT_REACHED";
pub const ERR_NOTHING_TO_WITHDRAW: &str = "ERR_NOTHING_TO_WITHDRAW";

const EMPLOYER_SIDE: u8 = 1;
const WORKER_SIDE: u8 = 2;

const SCORE_DELTA_RECURRING: i64 = 5;
const SCORE_DELTA_MILESTONE: i64 = 3;
const SCORE_DELTA_EMPLOYER_DEFAULT: i64 = -60;
const SCORE_DELTA_UNILATERAL: i64 = -20;
const SCORE_DELTA_COMPLETION: i64 = 25;

#[multiversx_sc::contract]
pub trait WorkEscrow {
    #[init]
    fn init(
        &self,
        job_board: ManagedAddress,
        bond_registry: ManagedAddress,
        uptime: ManagedAddress,
        min_uptime_score: u64,
        treasury: ManagedAddress,
        protocol_fee_bps: u64,
        referral_share_bps: u64,
        min_employer_bond: BigUint,
        min_worker_bond: BigUint,
        min_runway_periods: u64,
        default_notice_seconds: u64,
        termination_penalty_bps: u64,
        milestone_review_timeout_seconds: u64,
        max_milestones_per_agreement: u64,
        score_start: u64,
    ) {
        require!(!job_board.is_zero(), ERR_INVALID_AMOUNT);
        require!(!bond_registry.is_zero(), ERR_INVALID_AMOUNT);
        require!(!uptime.is_zero(), ERR_INVALID_AMOUNT);
        require!(!treasury.is_zero(), ERR_INVALID_AMOUNT);
        require!(protocol_fee_bps <= BPS_DENOMINATOR, ERR_INVALID_BPS);
        require!(referral_share_bps <= BPS_DENOMINATOR, ERR_INVALID_BPS);
        require!(termination_penalty_bps <= BPS_DENOMINATOR, ERR_INVALID_BPS);
        require!(min_employer_bond > 0u64, ERR_INVALID_AMOUNT);
        require!(min_worker_bond > 0u64, ERR_INVALID_AMOUNT);
        require!(min_runway_periods > 0, ERR_INVALID_AMOUNT);
        require!(default_notice_seconds > 0, ERR_INVALID_AMOUNT);
        require!(milestone_review_timeout_seconds > 0, ERR_INVALID_AMOUNT);
        require!(max_milestones_per_agreement > 0, ERR_INVALID_AMOUNT);

        let caller = self.blockchain().get_caller();
        self.owner().set(caller);
        self.paused().set(false);
        self.job_board().set(job_board);
        self.bond_registry().set(bond_registry);
        self.uptime().set(uptime);
        self.min_uptime_score().set(min_uptime_score);
        self.treasury().set(treasury);
        self.protocol_fee_bps().set(protocol_fee_bps);
        self.referral_share_bps().set(referral_share_bps);
        self.min_employer_bond().set(min_employer_bond);
        self.min_worker_bond().set(min_worker_bond);
        self.min_runway_periods().set(min_runway_periods);
        self.default_notice_seconds().set(default_notice_seconds);
        self.termination_penalty_bps().set(termination_penalty_bps);
        self.milestone_review_timeout_seconds()
            .set(milestone_review_timeout_seconds);
        self.max_milestones_per_agreement()
            .set(max_milestones_per_agreement);
        self.score_start().set(score_start);

        self.agreement_count().set(0u64);
        self.active_agreement_count().set(0u64);
        self.completed_agreement_count().set(0u64);
        self.terminated_agreement_count().set(0u64);
        self.total_gross_payouts().set(BigUint::zero());
        self.total_protocol_fees().set(BigUint::zero());
        self.total_revenue_deposited().set(BigUint::zero());
    }

    #[upgrade]
    fn upgrade(&self) {}

    #[endpoint(activateAgreement)]
    fn activate_agreement(
        &self,
        job_id: u64,
        offer_id: u64,
        referrer: OptionalValue<ManagedAddress>,
    ) -> u64 {
        self.require_not_paused();
        require!(!self.offer_consumed(job_id, offer_id).get(), ERR_OFFER_CONSUMED);

        let accepted = self.fetch_accepted_offer(job_id);
        require!(accepted.offer_id == offer_id, ERR_OFFER_NOT_ACCEPTED);

        let caller = self.blockchain().get_caller();
        require!(
            caller == accepted.employer || caller == accepted.worker,
            ERR_UNAUTHORIZED
        );

        self.require_eligible_agent(&accepted.employer, self.min_uptime_score().get());
        self.require_eligible_agent(&accepted.worker, self.min_uptime_score().get());
        self.validate_terms(&accepted);

        let agreement_id = self.agreement_count().get() + 1;
        self.agreement_count().set(agreement_id);

        let now = self.blockchain().get_block_timestamp();
        let employer_bond_required = self.max_biguint(
            &self.min_employer_bond().get(),
            &accepted.terms.employer_bond_required,
        );
        let worker_bond_required = self.max_biguint(
            &self.min_worker_bond().get(),
            &accepted.terms.worker_bond_required,
        );

        let recurring_reserved = self.compute_reserved_runway(
            &accepted.terms.recurring.amount_per_period,
            self.min_runway_periods().get(),
        );

        let recurring = RecurringTermsEscrow {
            amount_per_period: accepted.terms.recurring.amount_per_period,
            period_seconds: accepted.terms.recurring.period_seconds,
            total_periods: accepted.terms.recurring.total_periods,
            paid_periods: 0,
            next_pay_ts: 0,
        };

        let terms = AgreementTerms {
            recurring,
            revenue_share: RevenueShareTermsEscrow {
                profit_share_bps: accepted.terms.revenue_share.profit_share_bps,
                protocol_fee_bps_snapshot: self.protocol_fee_bps().get(),
                referral_share_bps_snapshot: self.referral_share_bps().get(),
            },
            employer_bond_required,
            worker_bond_required,
            milestone_count: accepted.terms.milestones.len() as u64,
        };

        let resolved_referrer = match referrer {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => ManagedAddress::zero(),
        };

        let agreement = Agreement {
            id: agreement_id,
            job_id,
            offer_id,
            employer: accepted.employer.clone(),
            worker: accepted.worker.clone(),
            referrer: resolved_referrer,
            status: AgreementStatus::PendingFunding,
            created_at: now,
            activated_at: 0,
            notice_start_ts: 0,
            notice_end_ts: 0,
            requested_by_side: 0,
            default_side: 0,
            terms,
        };

        self.agreements(agreement_id).set(agreement.clone());
        self.agreement_financials(agreement_id).set(FundingState {
            runway_balance: BigUint::zero(),
            employer_bond_locked: BigUint::zero(),
            worker_bond_locked: BigUint::zero(),
            reserved_recurring_minimum: recurring_reserved,
        });
        self.offer_consumed(job_id, offer_id).set(true);
        self.agreement_by_offer(job_id, offer_id).set(agreement_id);

        for milestone in accepted.terms.milestones.iter() {
            self.milestones(agreement_id, milestone.id).set(Milestone {
                id: milestone.id,
                agreement_id,
                amount: milestone.amount.clone(),
                due_ts: milestone.due_ts,
                review_timeout_seconds: if milestone.review_timeout_seconds > 0 {
                    milestone.review_timeout_seconds
                } else {
                    self.milestone_review_timeout_seconds().get()
                },
                metadata_uri: milestone.metadata_uri.clone(),
                state: MilestoneState::Open,
                submitted_at: 0,
                review_deadline: 0,
                proof_uri: ManagedBuffer::new(),
                reason_uri: ManagedBuffer::new(),
                settlement_mode: 0,
                paid_at: 0,
            });
        }

        self.ensure_reputation_initialized(&agreement.employer, agreement_id);
        self.ensure_reputation_initialized(&agreement.worker, agreement_id);

        self.agreement_activated_event(
            agreement_id,
            job_id,
            offer_id,
            &agreement.employer,
            &agreement.worker,
            now,
        );

        agreement_id
    }

    #[endpoint(fundEmployerRunway)]
    #[payable("EGLD")]
    fn fund_employer_runway(&self, agreement_id: u64) {
        self.require_not_paused();
        let mut agreement = self.require_agreement(agreement_id);
        let caller = self.blockchain().get_caller();
        require!(caller == agreement.employer, ERR_UNAUTHORIZED);
        require!(
            agreement.status == AgreementStatus::PendingFunding
                || agreement.status == AgreementStatus::Active,
            ERR_INVALID_STATE
        );

        let payment = self.call_value().egld_value().clone_value();
        require!(payment > 0u64, ERR_INVALID_AMOUNT);

        let mut funding = self.agreement_financials(agreement_id).get();
        let mut remaining = payment.clone();

        if agreement.status == AgreementStatus::PendingFunding
            && funding.employer_bond_locked < agreement.terms.employer_bond_required
        {
            let needed = &agreement.terms.employer_bond_required - &funding.employer_bond_locked;
            let allocate = self.min_biguint(&remaining, &needed);
            funding.employer_bond_locked += &allocate;
            remaining -= &allocate;
        }

        if remaining > 0u64 {
            funding.runway_balance += &remaining;
        }

        self.agreement_financials(agreement_id).set(funding.clone());
        self.try_activate(agreement_id, &mut agreement, &funding);

        self.runway_funded_event(
            agreement_id,
            &caller,
            payment,
            funding.runway_balance,
            self.blockchain().get_block_timestamp(),
        );
    }

    #[endpoint(fundWorkerBond)]
    #[payable("EGLD")]
    fn fund_worker_bond(&self, agreement_id: u64) {
        self.require_not_paused();
        let mut agreement = self.require_agreement(agreement_id);
        let caller = self.blockchain().get_caller();
        require!(caller == agreement.worker, ERR_UNAUTHORIZED);
        require!(
            agreement.status == AgreementStatus::PendingFunding
                || agreement.status == AgreementStatus::Active,
            ERR_INVALID_STATE
        );

        let payment = self.call_value().egld_value().clone_value();
        require!(payment > 0u64, ERR_INVALID_AMOUNT);

        let mut funding = self.agreement_financials(agreement_id).get();
        funding.worker_bond_locked += &payment;
        self.agreement_financials(agreement_id).set(funding.clone());
        self.try_activate(agreement_id, &mut agreement, &funding);

        self.worker_bond_funded_event(
            agreement_id,
            &caller,
            payment,
            funding.worker_bond_locked,
            self.blockchain().get_block_timestamp(),
        );
    }

    #[endpoint(topUpRunway)]
    #[payable("EGLD")]
    fn top_up_runway(&self, agreement_id: u64) {
        self.require_not_paused();
        let agreement = self.require_agreement(agreement_id);
        let caller = self.blockchain().get_caller();
        require!(caller == agreement.employer, ERR_UNAUTHORIZED);
        require!(
            agreement.status == AgreementStatus::Active
                || agreement.status == AgreementStatus::NoticePeriod,
            ERR_INVALID_STATE
        );

        let payment = self.call_value().egld_value().clone_value();
        require!(payment > 0u64, ERR_INVALID_AMOUNT);

        let mut funding = self.agreement_financials(agreement_id).get();
        funding.runway_balance += &payment;
        self.agreement_financials(agreement_id).set(funding.clone());

        self.runway_funded_event(
            agreement_id,
            &caller,
            payment,
            funding.runway_balance,
            self.blockchain().get_block_timestamp(),
        );
    }

    #[endpoint(claimRecurringPay)]
    fn claim_recurring_pay(&self, agreement_id: u64) {
        self.require_not_paused();
        let mut agreement = self.require_agreement(agreement_id);
        let caller = self.blockchain().get_caller();
        require!(caller == agreement.worker, ERR_UNAUTHORIZED);
        require!(agreement.status == AgreementStatus::Active, ERR_INVALID_STATE);

        require!(
            agreement.terms.recurring.total_periods > 0
                && agreement.terms.recurring.amount_per_period > 0u64,
            ERR_INVALID_STATE
        );
        require!(
            agreement.terms.recurring.paid_periods < agreement.terms.recurring.total_periods,
            ERR_INVALID_STATE
        );

        let now = self.blockchain().get_block_timestamp();
        require!(now >= agreement.terms.recurring.next_pay_ts, ERR_INVALID_DEADLINE);

        let gross = agreement.terms.recurring.amount_per_period.clone();
        let mut funding = self.agreement_financials(agreement_id).get();

        if funding.runway_balance < gross {
            self.handle_employer_default(agreement_id, &mut agreement);
            return;
        }

        funding.runway_balance -= &gross;
        agreement.terms.recurring.paid_periods += 1;
        agreement.terms.recurring.next_pay_ts += agreement.terms.recurring.period_seconds;

        let (protocol_fee, _referral_fee, worker_net) =
            self.credit_worker_payout(&agreement, &gross, agreement_id);

        self.agreement_financials(agreement_id).set(funding.clone());
        self.agreements(agreement_id).set(agreement.clone());

        self.record_agreement_totals(agreement_id, &gross, &protocol_fee);
        self.apply_reputation_delta(
            &agreement.employer,
            SCORE_DELTA_RECURRING,
            ReputationReason::OnTimeRecurringPayment,
            agreement_id,
        );

        self.pay_claimed_event(
            agreement_id,
            agreement.terms.recurring.paid_periods,
            gross,
            protocol_fee,
            worker_net,
            now,
        );

        self.try_complete(agreement_id, &mut agreement, &mut funding);
    }

    #[endpoint(submitMilestone)]
    fn submit_milestone(&self, agreement_id: u64, milestone_id: u64, proof_uri: ManagedBuffer) {
        self.require_not_paused();
        let agreement = self.require_agreement(agreement_id);
        let caller = self.blockchain().get_caller();
        require!(caller == agreement.worker, ERR_UNAUTHORIZED);
        require!(agreement.status == AgreementStatus::Active, ERR_INVALID_STATE);
        // Proof payload is persisted on-chain as canonical bytes.
        require!(proof_uri.len() <= MAX_PROOF_URI_LEN, ERR_INVALID_AMOUNT);

        let mut milestone = self.require_milestone(agreement_id, milestone_id);
        require!(milestone.state == MilestoneState::Open, ERR_MILESTONE_STATE);

        let now = self.blockchain().get_block_timestamp();
        milestone.state = MilestoneState::Submitted;
        milestone.submitted_at = now;
        milestone.review_deadline = now + milestone.review_timeout_seconds;
        milestone.proof_uri = proof_uri;

        self.milestones(agreement_id, milestone_id).set(milestone);
        self.milestone_submitted_event(agreement_id, milestone_id, &caller, now);
    }

    #[endpoint(approveMilestone)]
    fn approve_milestone(&self, agreement_id: u64, milestone_id: u64) {
        self.require_not_paused();
        let mut agreement = self.require_agreement(agreement_id);
        let caller = self.blockchain().get_caller();
        require!(caller == agreement.employer, ERR_UNAUTHORIZED);
        require!(agreement.status == AgreementStatus::Active, ERR_INVALID_STATE);

        let mut milestone = self.require_milestone(agreement_id, milestone_id);
        require!(milestone.state == MilestoneState::Submitted, ERR_MILESTONE_STATE);

        let mut funding = self.agreement_financials(agreement_id).get();
        self.require_milestone_runway(&funding, &milestone.amount);

        funding.runway_balance -= &milestone.amount;
        milestone.state = MilestoneState::Paid;
        milestone.settlement_mode = MilestoneSettlementMode::Approved as u8;
        milestone.paid_at = self.blockchain().get_block_timestamp();
        self.milestones(agreement_id, milestone_id).set(milestone.clone());

        let gross = milestone.amount.clone();
        let (protocol_fee, _, worker_net) = self.credit_worker_payout(&agreement, &gross, agreement_id);

        self.agreement_financials(agreement_id).set(funding.clone());
        self.record_agreement_totals(agreement_id, &gross, &protocol_fee);

        self.apply_reputation_delta(
            &agreement.worker,
            SCORE_DELTA_MILESTONE,
            ReputationReason::WorkerMilestoneSettled,
            agreement_id,
        );

        self.milestone_settled_event(
            agreement_id,
            milestone_id,
            MilestoneSettlementMode::Approved as u8,
            gross,
            protocol_fee,
            worker_net,
            self.blockchain().get_block_timestamp(),
        );

        self.try_complete(agreement_id, &mut agreement, &mut funding);
    }

    #[endpoint(rejectMilestone)]
    fn reject_milestone(&self, agreement_id: u64, milestone_id: u64, reason_uri: ManagedBuffer) {
        self.require_not_paused();
        let agreement = self.require_agreement(agreement_id);
        let caller = self.blockchain().get_caller();
        require!(caller == agreement.employer, ERR_UNAUTHORIZED);
        // Rejection reason payload is persisted on-chain as canonical bytes.
        require!(reason_uri.len() <= MAX_REASON_URI_LEN, ERR_INVALID_AMOUNT);

        let mut milestone = self.require_milestone(agreement_id, milestone_id);
        require!(milestone.state == MilestoneState::Submitted, ERR_MILESTONE_STATE);
        require!(
            self.blockchain().get_block_timestamp() <= milestone.review_deadline,
            ERR_TIMEOUT_NOT_REACHED
        );

        milestone.state = MilestoneState::Rejected;
        milestone.reason_uri = reason_uri;
        self.milestones(agreement_id, milestone_id).set(milestone);

        self.milestone_rejected_event(
            agreement_id,
            milestone_id,
            &caller,
            self.blockchain().get_block_timestamp(),
        );
    }

    #[endpoint(autoApproveMilestone)]
    fn auto_approve_milestone(&self, agreement_id: u64, milestone_id: u64) {
        self.require_not_paused();
        let mut agreement = self.require_agreement(agreement_id);
        require!(agreement.status == AgreementStatus::Active, ERR_INVALID_STATE);

        let mut milestone = self.require_milestone(agreement_id, milestone_id);
        require!(milestone.state == MilestoneState::Submitted, ERR_MILESTONE_STATE);
        require!(
            self.blockchain().get_block_timestamp() > milestone.review_deadline,
            ERR_TIMEOUT_NOT_REACHED
        );

        let mut funding = self.agreement_financials(agreement_id).get();
        self.require_milestone_runway(&funding, &milestone.amount);

        funding.runway_balance -= &milestone.amount;
        milestone.state = MilestoneState::Paid;
        milestone.settlement_mode = MilestoneSettlementMode::AutoApproved as u8;
        milestone.paid_at = self.blockchain().get_block_timestamp();
        self.milestones(agreement_id, milestone_id).set(milestone.clone());

        let gross = milestone.amount.clone();
        let (protocol_fee, _, worker_net) = self.credit_worker_payout(&agreement, &gross, agreement_id);

        self.agreement_financials(agreement_id).set(funding.clone());
        self.record_agreement_totals(agreement_id, &gross, &protocol_fee);

        self.apply_reputation_delta(
            &agreement.worker,
            SCORE_DELTA_MILESTONE,
            ReputationReason::WorkerMilestoneSettled,
            agreement_id,
        );

        self.milestone_settled_event(
            agreement_id,
            milestone_id,
            MilestoneSettlementMode::AutoApproved as u8,
            gross,
            protocol_fee,
            worker_net,
            self.blockchain().get_block_timestamp(),
        );

        self.try_complete(agreement_id, &mut agreement, &mut funding);
    }

    #[endpoint(depositRevenue)]
    #[payable("EGLD")]
    fn deposit_revenue(&self, agreement_id: u64) {
        self.require_not_paused();
        let agreement = self.require_agreement(agreement_id);
        require!(
            agreement.status == AgreementStatus::Active
                || agreement.status == AgreementStatus::NoticePeriod,
            ERR_INVALID_STATE
        );

        let gross = self.call_value().egld_value().clone_value();
        require!(gross > 0u64, ERR_INVALID_AMOUNT);

        let protocol_fee = self.mul_bps(
            &gross,
            agreement.terms.revenue_share.protocol_fee_bps_snapshot,
        );
        let net_after_fee = &gross - &protocol_fee;
        let worker_share = self.mul_bps(
            &net_after_fee,
            agreement.terms.revenue_share.profit_share_bps,
        );
        let employer_share = &net_after_fee - &worker_share;

        let mut referral_fee = BigUint::zero();
        if !agreement.referrer.is_zero() {
            referral_fee = self.mul_bps(
                &protocol_fee,
                agreement.terms.revenue_share.referral_share_bps_snapshot,
            );
            self.add_claimable(&agreement.referrer, &referral_fee);
        }
        let treasury_fee = &protocol_fee - &referral_fee;

        self.add_claimable(&agreement.worker, &worker_share);
        self.add_claimable(&agreement.employer, &employer_share);
        self.add_claimable(&self.treasury().get(), &treasury_fee);

        self.total_revenue_deposited().update(|v| *v += &gross);
        self.total_protocol_fees().update(|v| *v += &protocol_fee);

        self.revenue_deposited_event(
            agreement_id,
            &self.blockchain().get_caller(),
            gross,
            worker_share,
            employer_share,
            protocol_fee,
            self.blockchain().get_block_timestamp(),
        );
    }

    #[endpoint(requestTerminate)]
    fn request_terminate(&self, agreement_id: u64, side: TerminationSide) {
        self.require_not_paused();
        let mut agreement = self.require_agreement(agreement_id);
        require!(agreement.status == AgreementStatus::Active, ERR_INVALID_STATE);

        let caller = self.blockchain().get_caller();
        match side {
            TerminationSide::Employer => require!(caller == agreement.employer, ERR_UNAUTHORIZED),
            TerminationSide::Worker => require!(caller == agreement.worker, ERR_UNAUTHORIZED),
        }

        agreement.status = AgreementStatus::NoticePeriod;
        agreement.notice_start_ts = self.blockchain().get_block_timestamp();
        agreement.notice_end_ts = agreement.notice_start_ts + self.default_notice_seconds().get();
        agreement.requested_by_side = if matches!(side, TerminationSide::Employer) {
            EMPLOYER_SIDE
        } else {
            WORKER_SIDE
        };

        self.active_agreement_count().update(|v| {
            if *v > 0 {
                *v -= 1;
            }
        });

        self.agreements(agreement_id).set(agreement.clone());

        self.termination_requested_event(
            agreement_id,
            agreement.requested_by_side,
            agreement.notice_end_ts,
            self.blockchain().get_block_timestamp(),
        );
    }

    #[endpoint(finalizeTerminate)]
    fn finalize_terminate(&self, agreement_id: u64) {
        self.require_not_paused();
        let mut agreement = self.require_agreement(agreement_id);
        require!(
            agreement.status == AgreementStatus::NoticePeriod
                || (agreement.status == AgreementStatus::Active && agreement.default_side != 0),
            ERR_INVALID_STATE
        );

        let now = self.blockchain().get_block_timestamp();
        let default_shortcut = agreement.default_side != 0;
        if !default_shortcut {
            require!(now >= agreement.notice_end_ts, ERR_TIMEOUT_NOT_REACHED);
        }

        let mut funding = self.agreement_financials(agreement_id).get();

        let penalty_from_side = if agreement.default_side != 0 {
            agreement.default_side
        } else {
            agreement.requested_by_side
        };

        let (penalty_source, counterparty) = if penalty_from_side == EMPLOYER_SIDE {
            (&mut funding.employer_bond_locked, &agreement.worker)
        } else {
            (&mut funding.worker_bond_locked, &agreement.employer)
        };

        let penalty = self.mul_bps(&penalty_source.clone(), self.termination_penalty_bps().get());
        if penalty > 0u64 {
            *penalty_source -= &penalty;
            self.add_claimable(counterparty, &penalty);
        }

        let employer_refund = funding.employer_bond_locked.clone();
        let worker_refund = funding.worker_bond_locked.clone();

        if employer_refund > 0u64 {
            self.add_claimable(&agreement.employer, &employer_refund);
        }
        if worker_refund > 0u64 {
            self.add_claimable(&agreement.worker, &worker_refund);
        }

        funding.employer_bond_locked = BigUint::zero();
        funding.worker_bond_locked = BigUint::zero();
        self.agreement_financials(agreement_id).set(funding);

        let reason = if agreement.default_side == EMPLOYER_SIDE {
            TerminationReason::EmployerDefault
        } else if agreement.default_side == WORKER_SIDE {
            TerminationReason::WorkerDefault
        } else if agreement.requested_by_side == EMPLOYER_SIDE {
            self.apply_reputation_delta(
                &agreement.employer,
                SCORE_DELTA_UNILATERAL,
                ReputationReason::UnilateralTerminate,
                agreement_id,
            );
            TerminationReason::UnilateralEmployer
        } else {
            self.apply_reputation_delta(
                &agreement.worker,
                SCORE_DELTA_UNILATERAL,
                ReputationReason::UnilateralTerminate,
                agreement_id,
            );
            TerminationReason::UnilateralWorker
        };

        agreement.status = AgreementStatus::Terminated;
        self.agreements(agreement_id).set(agreement.clone());
        self.terminated_agreement_count().update(|v| *v += 1);

        self.agreement_terminated_event(
            agreement_id,
            reason as u8,
            penalty,
            employer_refund,
            worker_refund,
            now,
        );
    }

    #[endpoint(withdrawClaimable)]
    fn withdraw_claimable(&self) {
        let caller = self.blockchain().get_caller();
        let amount = self.claimable(&caller).get();
        require!(amount > 0u64, ERR_NOTHING_TO_WITHDRAW);

        self.claimable(&caller).set(BigUint::zero());
        self.send().direct_egld(&caller, &amount);

        self.fee_withdrawn_event(&caller, amount, self.blockchain().get_block_timestamp());
    }

    #[endpoint(setProtocolFeeBps)]
    fn set_protocol_fee_bps(&self, value: u64) {
        self.require_owner();
        require!(value <= BPS_DENOMINATOR, ERR_INVALID_BPS);
        self.protocol_fee_bps().set(value);
    }

    #[endpoint(setReferralShareBps)]
    fn set_referral_share_bps(&self, value: u64) {
        self.require_owner();
        require!(value <= BPS_DENOMINATOR, ERR_INVALID_BPS);
        self.referral_share_bps().set(value);
    }

    #[endpoint(setTreasury)]
    fn set_treasury(&self, addr: ManagedAddress) {
        self.require_owner();
        require!(!addr.is_zero(), ERR_INVALID_AMOUNT);
        self.treasury().set(addr);
    }

    #[endpoint(setMinUptimeScore)]
    fn set_min_uptime_score(&self, value: u64) {
        self.require_owner();
        self.min_uptime_score().set(value);
    }

    #[endpoint(setRiskParams)]
    fn set_risk_params(
        &self,
        min_employer_bond: BigUint,
        min_worker_bond: BigUint,
        min_runway_periods: u64,
        default_notice_seconds: u64,
        termination_penalty_bps: u64,
        milestone_review_timeout_seconds: u64,
        max_milestones_per_agreement: u64,
        score_start: u64,
    ) {
        self.require_owner();
        require!(min_employer_bond > 0u64, ERR_INVALID_AMOUNT);
        require!(min_worker_bond > 0u64, ERR_INVALID_AMOUNT);
        require!(min_runway_periods > 0, ERR_INVALID_AMOUNT);
        require!(default_notice_seconds > 0, ERR_INVALID_AMOUNT);
        require!(termination_penalty_bps <= BPS_DENOMINATOR, ERR_INVALID_BPS);
        require!(milestone_review_timeout_seconds > 0, ERR_INVALID_AMOUNT);
        require!(max_milestones_per_agreement > 0, ERR_INVALID_AMOUNT);

        self.min_employer_bond().set(min_employer_bond);
        self.min_worker_bond().set(min_worker_bond);
        self.min_runway_periods().set(min_runway_periods);
        self.default_notice_seconds().set(default_notice_seconds);
        self.termination_penalty_bps().set(termination_penalty_bps);
        self.milestone_review_timeout_seconds()
            .set(milestone_review_timeout_seconds);
        self.max_milestones_per_agreement()
            .set(max_milestones_per_agreement);
        self.score_start().set(score_start);
    }

    #[endpoint(setPaused)]
    fn set_paused(&self, paused: bool) {
        self.require_owner();
        self.paused().set(paused);
    }

    #[endpoint(setOwner)]
    fn set_owner(&self, new_owner: ManagedAddress) {
        self.require_owner();
        require!(!new_owner.is_zero(), ERR_INVALID_AMOUNT);
        self.owner().set(new_owner);
    }

    #[view(getAgreement)]
    fn get_agreement(&self, agreement_id: u64) -> OptionalValue<Agreement<Self::Api>> {
        if self.agreements(agreement_id).is_empty() {
            OptionalValue::None
        } else {
            OptionalValue::Some(self.agreements(agreement_id).get())
        }
    }

    #[view(getAgreementFinancials)]
    fn get_agreement_financials(&self, agreement_id: u64) -> AgreementFinancials<Self::Api> {
        let agreement = self.require_agreement(agreement_id);
        let funding = self.agreement_financials(agreement_id).get();

        AgreementFinancials {
            funding,
            worker_claimable: self.claimable(&agreement.worker).get(),
            employer_claimable: self.claimable(&agreement.employer).get(),
            referrer_claimable: if agreement.referrer.is_zero() {
                BigUint::zero()
            } else {
                self.claimable(&agreement.referrer).get()
            },
            treasury_claimable: self.claimable(&self.treasury().get()).get(),
            total_gross_paid: self.agreement_total_gross_paid(agreement_id).get(),
            total_fees_paid: self.agreement_total_fees_paid(agreement_id).get(),
        }
    }

    #[view(getMilestone)]
    fn get_milestone(&self, agreement_id: u64, milestone_id: u64) -> OptionalValue<Milestone<Self::Api>> {
        if self.milestones(agreement_id, milestone_id).is_empty() {
            OptionalValue::None
        } else {
            OptionalValue::Some(self.milestones(agreement_id, milestone_id).get())
        }
    }

    #[view(getAgentReputation)]
    fn get_agent_reputation(&self, agent: ManagedAddress) -> ReputationSnapshot {
        self.load_reputation(&agent)
    }

    #[view(getProtocolStats)]
    fn get_protocol_stats(&self) -> ProtocolStats<Self::Api> {
        ProtocolStats {
            total_agreements: self.agreement_count().get(),
            active_agreements: self.active_agreement_count().get(),
            completed_agreements: self.completed_agreement_count().get(),
            terminated_agreements: self.terminated_agreement_count().get(),
            total_gross_payouts: self.total_gross_payouts().get(),
            total_protocol_fees: self.total_protocol_fees().get(),
            total_revenue_deposited: self.total_revenue_deposited().get(),
        }
    }

    #[view(getConfig)]
    fn get_config(&self) -> EscrowConfig<Self::Api> {
        EscrowConfig {
            owner: self.owner().get(),
            job_board: self.job_board().get(),
            bond_registry: self.bond_registry().get(),
            uptime: self.uptime().get(),
            treasury: self.treasury().get(),
            min_uptime_score: self.min_uptime_score().get(),
            protocol_fee_bps: self.protocol_fee_bps().get(),
            referral_share_bps: self.referral_share_bps().get(),
            min_employer_bond: self.min_employer_bond().get(),
            min_worker_bond: self.min_worker_bond().get(),
            min_runway_periods: self.min_runway_periods().get(),
            default_notice_seconds: self.default_notice_seconds().get(),
            termination_penalty_bps: self.termination_penalty_bps().get(),
            milestone_review_timeout_seconds: self.milestone_review_timeout_seconds().get(),
            max_milestones_per_agreement: self.max_milestones_per_agreement().get(),
            score_start: self.score_start().get(),
            paused: self.paused().get(),
        }
    }

    #[view(getClaimable)]
    fn get_claimable(&self, agent: ManagedAddress) -> BigUint {
        self.claimable(&agent).get()
    }

    #[view(isOfferConsumed)]
    fn is_offer_consumed(&self, job_id: u64, offer_id: u64) -> bool {
        self.offer_consumed(job_id, offer_id).get()
    }

    fn require_owner(&self) {
        require!(self.blockchain().get_caller() == self.owner().get(), ERR_UNAUTHORIZED);
    }

    fn require_not_paused(&self) {
        require!(!self.paused().get(), ERR_PAUSED);
    }

    fn require_agreement(&self, agreement_id: u64) -> Agreement<Self::Api> {
        require!(!self.agreements(agreement_id).is_empty(), ERR_INVALID_STATE);
        self.agreements(agreement_id).get()
    }

    fn require_milestone(&self, agreement_id: u64, milestone_id: u64) -> Milestone<Self::Api> {
        require!(
            !self.milestones(agreement_id, milestone_id).is_empty(),
            ERR_INVALID_STATE
        );
        self.milestones(agreement_id, milestone_id).get()
    }

    fn fetch_accepted_offer(&self, job_id: u64) -> AcceptedOfferSummary<Self::Api> {
        let accepted: OptionalValue<AcceptedOfferSummary<Self::Api>> = self
            .tx()
            .to(self.job_board().get())
            .typed(JobBoardProxy)
            .get_accepted_offer(job_id)
            .returns(ReturnsResult)
            .sync_call_readonly();

        match accepted {
            OptionalValue::Some(value) => value,
            OptionalValue::None => sc_panic!(ERR_OFFER_NOT_ACCEPTED),
        }
    }

    fn validate_terms(&self, accepted: &AcceptedOfferSummary<Self::Api>) {
        require!(
            accepted.terms.revenue_share.profit_share_bps <= BPS_DENOMINATOR,
            ERR_INVALID_BPS
        );
        require!(
            accepted.terms.milestones.len() as u64 <= self.max_milestones_per_agreement().get(),
            ERR_INVALID_AMOUNT
        );

        if accepted.terms.recurring.amount_per_period > 0u64 {
            require!(accepted.terms.recurring.period_seconds > 0, ERR_INVALID_AMOUNT);
            require!(accepted.terms.recurring.total_periods > 0, ERR_INVALID_AMOUNT);
        } else {
            require!(accepted.terms.recurring.total_periods == 0, ERR_INVALID_AMOUNT);
        }

        for m in accepted.terms.milestones.iter() {
            require!(m.amount > 0u64, ERR_INVALID_AMOUNT);
            require!(m.review_timeout_seconds > 0, ERR_INVALID_AMOUNT);
        }
    }

    fn try_activate(
        &self,
        agreement_id: u64,
        agreement: &mut Agreement<Self::Api>,
        funding: &FundingState<Self::Api>,
    ) {
        if agreement.status != AgreementStatus::PendingFunding {
            return;
        }

        if funding.employer_bond_locked < agreement.terms.employer_bond_required {
            return;
        }
        if funding.worker_bond_locked < agreement.terms.worker_bond_required {
            return;
        }
        if funding.runway_balance < funding.reserved_recurring_minimum {
            return;
        }

        agreement.status = AgreementStatus::Active;
        agreement.activated_at = self.blockchain().get_block_timestamp();
        if agreement.terms.recurring.total_periods > 0 {
            agreement.terms.recurring.next_pay_ts =
                agreement.activated_at + agreement.terms.recurring.period_seconds;
        }

        self.agreements(agreement_id).set(agreement.clone());
        self.active_agreement_count().update(|v| *v += 1);
    }

    fn try_complete(
        &self,
        agreement_id: u64,
        agreement: &mut Agreement<Self::Api>,
        funding: &mut FundingState<Self::Api>,
    ) {
        if agreement.status != AgreementStatus::Active {
            return;
        }

        let recurring_done = agreement.terms.recurring.total_periods == 0
            || agreement.terms.recurring.paid_periods >= agreement.terms.recurring.total_periods;

        if !recurring_done {
            return;
        }

        for milestone_id in 1..=agreement.terms.milestone_count {
            if self.milestones(agreement_id, milestone_id).is_empty() {
                continue;
            }
            let milestone = self.milestones(agreement_id, milestone_id).get();
            if milestone.state != MilestoneState::Paid {
                return;
            }
        }

        let employer_refund = funding.employer_bond_locked.clone();
        let worker_refund = funding.worker_bond_locked.clone();

        if employer_refund > 0u64 {
            self.add_claimable(&agreement.employer, &employer_refund);
        }
        if worker_refund > 0u64 {
            self.add_claimable(&agreement.worker, &worker_refund);
        }

        funding.employer_bond_locked = BigUint::zero();
        funding.worker_bond_locked = BigUint::zero();

        agreement.status = AgreementStatus::Completed;
        self.agreement_financials(agreement_id).set(funding.clone());
        self.agreements(agreement_id).set(agreement.clone());

        self.active_agreement_count().update(|v| {
            if *v > 0 {
                *v -= 1;
            }
        });
        self.completed_agreement_count().update(|v| *v += 1);

        self.apply_reputation_delta(
            &agreement.employer,
            SCORE_DELTA_COMPLETION,
            ReputationReason::Completion,
            agreement_id,
        );
        self.apply_reputation_delta(
            &agreement.worker,
            SCORE_DELTA_COMPLETION,
            ReputationReason::Completion,
            agreement_id,
        );
    }

    fn handle_employer_default(&self, agreement_id: u64, agreement: &mut Agreement<Self::Api>) {
        agreement.default_side = EMPLOYER_SIDE;
        agreement.status = AgreementStatus::NoticePeriod;
        agreement.notice_start_ts = self.blockchain().get_block_timestamp();
        agreement.notice_end_ts = agreement.notice_start_ts;
        agreement.requested_by_side = WORKER_SIDE;
        self.agreements(agreement_id).set(agreement.clone());

        self.active_agreement_count().update(|v| {
            if *v > 0 {
                *v -= 1;
            }
        });

        self.apply_reputation_delta(
            &agreement.employer,
            SCORE_DELTA_EMPLOYER_DEFAULT,
            ReputationReason::EmployerDefault,
            agreement_id,
        );

        self.agreement_defaulted_event(
            agreement_id,
            EMPLOYER_SIDE,
            self.blockchain().get_block_timestamp(),
        );
    }

    fn credit_worker_payout(
        &self,
        agreement: &Agreement<Self::Api>,
        gross: &BigUint,
        agreement_id: u64,
    ) -> (BigUint, BigUint, BigUint) {
        let protocol_fee = self.mul_bps(gross, agreement.terms.revenue_share.protocol_fee_bps_snapshot);
        let worker_net = gross - &protocol_fee;

        let mut referral_fee = BigUint::zero();
        if !agreement.referrer.is_zero() {
            referral_fee = self.mul_bps(
                &protocol_fee,
                agreement.terms.revenue_share.referral_share_bps_snapshot,
            );
            self.add_claimable(&agreement.referrer, &referral_fee);
        }
        let treasury_fee = &protocol_fee - &referral_fee;

        self.add_claimable(&agreement.worker, &worker_net);
        self.add_claimable(&self.treasury().get(), &treasury_fee);

        self.total_protocol_fees().update(|v| *v += &protocol_fee);
        self.total_gross_payouts().update(|v| *v += gross);

        self.revenue_ledger(agreement_id).update(|v| {
            *v += 1u64;
        });

        (protocol_fee, referral_fee, worker_net)
    }

    fn require_milestone_runway(&self, funding: &FundingState<Self::Api>, amount: &BigUint) {
        require!(funding.runway_balance >= *amount, ERR_INSUFFICIENT_RUNWAY);
        let remain = &funding.runway_balance - amount;
        require!(
            remain >= funding.reserved_recurring_minimum,
            ERR_INSUFFICIENT_RUNWAY
        );
    }

    fn ensure_reputation_initialized(&self, agent: &ManagedAddress, agreement_id: u64) {
        let mut rep = self.load_reputation(agent);
        if self.reputation(agent).is_empty() {
            self.reputation_changed_event(
                agent,
                rep.score,
                rep.score,
                ReputationReason::Init as u8,
                agreement_id,
                self.blockchain().get_block_timestamp(),
            );
        }
        rep.agreements_started += 1;
        rep.last_updated_ts = self.blockchain().get_block_timestamp();
        self.reputation(agent).set(rep);
    }

    fn apply_reputation_delta(
        &self,
        agent: &ManagedAddress,
        delta: i64,
        reason: ReputationReason,
        agreement_id: u64,
    ) {
        let mut rep = self.load_reputation(agent);
        let prev = rep.score;

        if delta >= 0 {
            let up = delta as u64;
            rep.score = core::cmp::min(SCORE_MAX, rep.score.saturating_add(up));
        } else {
            let down = (-delta) as u64;
            rep.score = rep.score.saturating_sub(down);
        }

        match reason {
            ReputationReason::OnTimeRecurringPayment => rep.on_time_recurring_payments += 1,
            ReputationReason::WorkerMilestoneSettled => rep.milestones_settled += 1,
            ReputationReason::EmployerDefault => rep.defaults_as_employer += 1,
            ReputationReason::WorkerDefault => rep.defaults_as_worker += 1,
            ReputationReason::UnilateralTerminate => rep.terminations_initiated += 1,
            ReputationReason::Completion => rep.agreements_completed += 1,
            _ => {}
        }

        rep.last_updated_ts = self.blockchain().get_block_timestamp();
        self.reputation(agent).set(rep.clone());

        self.reputation_changed_event(
            agent,
            prev,
            rep.score,
            reason as u8,
            agreement_id,
            self.blockchain().get_block_timestamp(),
        );
    }

    fn load_reputation(&self, agent: &ManagedAddress) -> ReputationSnapshot {
        if self.reputation(agent).is_empty() {
            ReputationSnapshot {
                score: self.score_start().get(),
                agreements_started: 0,
                agreements_completed: 0,
                defaults_as_employer: 0,
                defaults_as_worker: 0,
                on_time_recurring_payments: 0,
                milestones_settled: 0,
                terminations_initiated: 0,
                last_updated_ts: 0,
            }
        } else {
            self.reputation(agent).get()
        }
    }

    fn add_claimable(&self, account: &ManagedAddress, amount: &BigUint) {
        if amount == &BigUint::zero() {
            return;
        }
        self.claimable(account).update(|v| *v += amount);
    }

    fn record_agreement_totals(&self, agreement_id: u64, gross: &BigUint, fees: &BigUint) {
        self.agreement_total_gross_paid(agreement_id)
            .update(|v| *v += gross);
        self.agreement_total_fees_paid(agreement_id)
            .update(|v| *v += fees);
    }

    fn require_eligible_agent(&self, agent: &ManagedAddress, min_uptime: u64) {
        let agent_name: ManagedBuffer = self
            .tx()
            .to(self.bond_registry().get())
            .typed(BondRegistryProxy)
            .get_agent_name(agent.clone())
            .returns(ReturnsResult)
            .sync_call_readonly();
        require!(!agent_name.is_empty(), ERR_NOT_REGISTERED);

        let lifetime_info: MultiValue4<u64, u64, u64, u64> = self
            .tx()
            .to(self.uptime().get())
            .typed(UptimeProxy)
            .get_lifetime_info(agent.clone())
            .returns(ReturnsResult)
            .sync_call_readonly();
        let (_, uptime_score, _, _) = lifetime_info.into_tuple();
        require!(uptime_score >= min_uptime, ERR_LOW_UPTIME);
    }

    fn mul_bps(&self, amount: &BigUint, bps: u64) -> BigUint {
        amount * bps / BPS_DENOMINATOR
    }

    fn max_biguint(&self, a: &BigUint, b: &BigUint) -> BigUint {
        if a > b {
            a.clone()
        } else {
            b.clone()
        }
    }

    fn min_biguint(&self, a: &BigUint, b: &BigUint) -> BigUint {
        if a < b {
            a.clone()
        } else {
            b.clone()
        }
    }

    fn compute_reserved_runway(&self, amount_per_period: &BigUint, periods: u64) -> BigUint {
        amount_per_period * periods
    }

    #[event("agreementActivated")]
    fn agreement_activated_event(
        &self,
        #[indexed] agreement_id: u64,
        #[indexed] job_id: u64,
        #[indexed] offer_id: u64,
        #[indexed] employer: &ManagedAddress,
        #[indexed] worker: &ManagedAddress,
        timestamp: u64,
    );

    #[event("runwayFunded")]
    fn runway_funded_event(
        &self,
        #[indexed] agreement_id: u64,
        #[indexed] employer: &ManagedAddress,
        #[indexed] amount: BigUint,
        #[indexed] runway_balance_after: BigUint,
        timestamp: u64,
    );

    #[event("workerBondFunded")]
    fn worker_bond_funded_event(
        &self,
        #[indexed] agreement_id: u64,
        #[indexed] worker: &ManagedAddress,
        #[indexed] amount: BigUint,
        #[indexed] worker_bond_after: BigUint,
        timestamp: u64,
    );

    #[event("payClaimed")]
    fn pay_claimed_event(
        &self,
        #[indexed] agreement_id: u64,
        #[indexed] period_index: u64,
        #[indexed] gross: BigUint,
        #[indexed] fee: BigUint,
        #[indexed] worker_net: BigUint,
        timestamp: u64,
    );

    #[event("milestoneSubmitted")]
    fn milestone_submitted_event(
        &self,
        #[indexed] agreement_id: u64,
        #[indexed] milestone_id: u64,
        #[indexed] worker: &ManagedAddress,
        timestamp: u64,
    );

    #[event("milestoneRejected")]
    fn milestone_rejected_event(
        &self,
        #[indexed] agreement_id: u64,
        #[indexed] milestone_id: u64,
        #[indexed] employer: &ManagedAddress,
        timestamp: u64,
    );

    #[event("milestoneSettled")]
    fn milestone_settled_event(
        &self,
        #[indexed] agreement_id: u64,
        #[indexed] milestone_id: u64,
        #[indexed] mode: u8,
        #[indexed] gross: BigUint,
        #[indexed] fee: BigUint,
        #[indexed] worker_net: BigUint,
        timestamp: u64,
    );

    #[event("revenueDeposited")]
    fn revenue_deposited_event(
        &self,
        #[indexed] agreement_id: u64,
        #[indexed] payer: &ManagedAddress,
        #[indexed] gross: BigUint,
        #[indexed] worker_share: BigUint,
        #[indexed] employer_share: BigUint,
        #[indexed] fee_total: BigUint,
        timestamp: u64,
    );

    #[event("terminationRequested")]
    fn termination_requested_event(
        &self,
        #[indexed] agreement_id: u64,
        #[indexed] side: u8,
        #[indexed] notice_end_ts: u64,
        timestamp: u64,
    );

    #[event("agreementTerminated")]
    fn agreement_terminated_event(
        &self,
        #[indexed] agreement_id: u64,
        #[indexed] reason: u8,
        #[indexed] penalty: BigUint,
        #[indexed] employer_refund: BigUint,
        #[indexed] worker_refund: BigUint,
        timestamp: u64,
    );

    #[event("reputationChanged")]
    fn reputation_changed_event(
        &self,
        #[indexed] agent: &ManagedAddress,
        #[indexed] prev_score: u64,
        #[indexed] new_score: u64,
        #[indexed] reason_code: u8,
        #[indexed] agreement_id: u64,
        timestamp: u64,
    );

    #[event("feeWithdrawn")]
    fn fee_withdrawn_event(
        &self,
        #[indexed] account: &ManagedAddress,
        #[indexed] amount: BigUint,
        timestamp: u64,
    );

    #[event("agreementDefaulted")]
    fn agreement_defaulted_event(
        &self,
        #[indexed] agreement_id: u64,
        #[indexed] side_at_fault: u8,
        timestamp: u64,
    );

    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("paused")]
    fn paused(&self) -> SingleValueMapper<bool>;

    #[storage_mapper("jobBoard")]
    fn job_board(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("bondRegistry")]
    fn bond_registry(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("uptime")]
    fn uptime(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("treasury")]
    fn treasury(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("minUptimeScore")]
    fn min_uptime_score(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("protocolFeeBps")]
    fn protocol_fee_bps(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("referralShareBps")]
    fn referral_share_bps(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("minEmployerBond")]
    fn min_employer_bond(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("minWorkerBond")]
    fn min_worker_bond(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("minRunwayPeriods")]
    fn min_runway_periods(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("defaultNoticeSeconds")]
    fn default_notice_seconds(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("terminationPenaltyBps")]
    fn termination_penalty_bps(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("milestoneReviewTimeoutSeconds")]
    fn milestone_review_timeout_seconds(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("maxMilestonesPerAgreement")]
    fn max_milestones_per_agreement(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("scoreStart")]
    fn score_start(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("agreementCount")]
    fn agreement_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("activeAgreementCount")]
    fn active_agreement_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("completedAgreementCount")]
    fn completed_agreement_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("terminatedAgreementCount")]
    fn terminated_agreement_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("agreements")]
    fn agreements(&self, agreement_id: u64) -> SingleValueMapper<Agreement<Self::Api>>;

    #[storage_mapper("agreementFinancials")]
    fn agreement_financials(&self, agreement_id: u64) -> SingleValueMapper<FundingState<Self::Api>>;

    #[storage_mapper("milestones")]
    fn milestones(&self, agreement_id: u64, milestone_id: u64) -> SingleValueMapper<Milestone<Self::Api>>;

    #[storage_mapper("claimable")]
    fn claimable(&self, account: &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[storage_mapper("offerConsumed")]
    fn offer_consumed(&self, job_id: u64, offer_id: u64) -> SingleValueMapper<bool>;

    #[storage_mapper("agreementByOffer")]
    fn agreement_by_offer(&self, job_id: u64, offer_id: u64) -> SingleValueMapper<u64>;

    #[storage_mapper("reputation")]
    fn reputation(&self, agent: &ManagedAddress) -> SingleValueMapper<ReputationSnapshot>;

    #[storage_mapper("totalGrossPayouts")]
    fn total_gross_payouts(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("totalProtocolFees")]
    fn total_protocol_fees(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("totalRevenueDeposited")]
    fn total_revenue_deposited(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("agreementTotalGrossPaid")]
    fn agreement_total_gross_paid(&self, agreement_id: u64) -> SingleValueMapper<BigUint>;

    #[storage_mapper("agreementTotalFeesPaid")]
    fn agreement_total_fees_paid(&self, agreement_id: u64) -> SingleValueMapper<BigUint>;

    #[storage_mapper("revenueLedger")]
    fn revenue_ledger(&self, agreement_id: u64) -> SingleValueMapper<u64>;
}
