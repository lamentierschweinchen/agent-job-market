#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod bond_registry_proxy;
mod uptime_proxy;

use bond_registry_proxy::BondRegistryProxy;
use shared_types::{
    AcceptedOfferSummary, Application, BoardStats, Job, JobBoardConfig, JobCloseReason, JobStatus,
    JobVisibility, MilestoneSpec, Offer, OfferParty, OfferStatus, OfferTerms, OfferTermsInput,
    BPS_DENOMINATOR, MAX_APPLICATION_URI_LEN, MAX_METADATA_URI_LEN, MAX_PAGE_SIZE,
    MAX_TERMS_URI_LEN,
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
pub const ERR_NOT_INVITED: &str = "ERR_NOT_INVITED";
pub const ERR_ALREADY_APPLIED: &str = "ERR_ALREADY_APPLIED";
pub const ERR_COUNTER_LIMIT: &str = "ERR_COUNTER_LIMIT";
pub const ERR_STALE_OFFER: &str = "ERR_STALE_OFFER";
pub const ERR_ALREADY_MATCHED: &str = "ERR_ALREADY_MATCHED";

const MAX_INVITE_LOOP_GUARD: usize = 1024;
const MAX_MILESTONES_PER_OFFER: usize = 32;

#[multiversx_sc::contract]
pub trait JobBoardCore {
    #[init]
    fn init(
        &self,
        bond_registry: ManagedAddress,
        uptime: ManagedAddress,
        min_uptime_score: u64,
        max_counteroffers_per_application: u64,
        max_invites_per_job: u64,
    ) {
        require!(!bond_registry.is_zero(), ERR_INVALID_AMOUNT);
        require!(!uptime.is_zero(), ERR_INVALID_AMOUNT);
        require!(max_counteroffers_per_application > 0, ERR_INVALID_AMOUNT);
        require!(max_invites_per_job > 0, ERR_INVALID_AMOUNT);

        let caller = self.blockchain().get_caller();
        self.owner().set(&caller);
        self.paused().set(false);
        self.bond_registry().set(&bond_registry);
        self.uptime().set(&uptime);
        self.min_uptime_score().set(min_uptime_score);
        self.max_counteroffers_per_application()
            .set(max_counteroffers_per_application);
        self.max_invites_per_job().set(max_invites_per_job);

        self.job_count().set(0u64);
        self.total_application_count().set(0u64);
        self.total_offer_count().set(0u64);
        self.open_job_count().set(0u64);
        self.matched_job_count().set(0u64);
    }

    #[upgrade]
    fn upgrade(&self) {}

    #[endpoint(createJob)]
    fn create_job(
        &self,
        metadata_uri: ManagedBuffer,
        visibility: JobVisibility,
        application_deadline_ts: u64,
        min_worker_uptime: u64,
        comp_mode_mask: u8,
        invited: MultiValueEncoded<ManagedAddress>,
    ) -> u64 {
        self.require_not_paused();
        let caller = self.blockchain().get_caller();
        self.require_eligible_agent(&caller, self.min_uptime_score().get());

        // The buffer is stored on-chain as canonical metadata payload bytes.
        require!(!metadata_uri.is_empty(), ERR_INVALID_AMOUNT);
        require!(metadata_uri.len() <= MAX_METADATA_URI_LEN, ERR_INVALID_AMOUNT);
        require!(
            application_deadline_ts > self.blockchain().get_block_timestamp(),
            ERR_INVALID_DEADLINE
        );

        let mut invites_vec: ManagedVec<Self::Api, ManagedAddress> = ManagedVec::new();
        for addr in invited.into_iter() {
            invites_vec.push(addr);
        }
        require!(
            invites_vec.len() <= self.max_invites_per_job().get() as usize,
            ERR_INVALID_AMOUNT
        );
        require!(invites_vec.len() <= MAX_INVITE_LOOP_GUARD, ERR_INVALID_AMOUNT);
        if visibility == JobVisibility::Public {
            require!(invites_vec.is_empty(), ERR_INVALID_STATE);
        }

        require!(comp_mode_mask > 0 && comp_mode_mask <= 0b111, ERR_INVALID_STATE);

        let job_id = self.job_count().get() + 1;
        self.job_count().set(job_id);

        let now = self.blockchain().get_block_timestamp();
        let job = Job {
            id: job_id,
            employer: caller.clone(),
            metadata_uri,
            visibility,
            application_deadline_ts,
            min_worker_uptime,
            comp_mode_mask,
            status: JobStatus::Open,
            created_at: now,
            accepted_offer_id: 0,
            application_count: 0,
        };

        self.jobs(job_id).set(job);
        self.job_index().push(&job_id);
        self.job_ids_by_employer(&caller).push(&job_id);
        self.open_job_count().update(|v| *v += 1);

        if visibility == JobVisibility::InviteOnly {
            for addr in invites_vec.iter() {
                self.invites(job_id).insert(addr.clone_value());
            }
        }

        self.job_created_event(job_id, &caller, visibility, now);
        job_id
    }

    #[endpoint(apply)]
    fn apply(&self, job_id: u64, application_uri: ManagedBuffer) -> u64 {
        self.require_not_paused();
        // Application payload is persisted on-chain; URL formatting is not enforced.
        require!(!application_uri.is_empty(), ERR_INVALID_AMOUNT);
        require!(application_uri.len() <= MAX_APPLICATION_URI_LEN, ERR_INVALID_AMOUNT);

        let caller = self.blockchain().get_caller();
        let mut job = self.require_job(job_id);
        require!(
            job.status == JobStatus::Open || job.status == JobStatus::InNegotiation,
            ERR_INVALID_STATE
        );
        require!(
            self.blockchain().get_block_timestamp() <= job.application_deadline_ts,
            ERR_INVALID_DEADLINE
        );

        let required_uptime = if job.min_worker_uptime > self.min_uptime_score().get() {
            job.min_worker_uptime
        } else {
            self.min_uptime_score().get()
        };
        self.require_eligible_agent(&caller, required_uptime);

        if job.visibility == JobVisibility::InviteOnly {
            require!(self.invites(job_id).contains(&caller), ERR_NOT_INVITED);
        }

        require!(!self.has_applied(job_id, &caller).get(), ERR_ALREADY_APPLIED);

        let application_id = self.application_count(job_id).get() + 1;
        self.application_count(job_id).set(application_id);
        self.has_applied(job_id, &caller).set(true);

        let now = self.blockchain().get_block_timestamp();
        let application = Application {
            id: application_id,
            job_id,
            applicant: caller.clone(),
            application_uri,
            created_at: now,
        };
        self.applications(job_id, application_id).set(application);

        job.application_count += 1;
        if job.status == JobStatus::Open {
            job.status = JobStatus::InNegotiation;
            self.open_job_count().update(|v| {
                if *v > 0 {
                    *v -= 1;
                }
            });
        }
        self.jobs(job_id).set(job);

        self.total_application_count().update(|v| *v += 1);
        self.application_submitted_event(job_id, application_id, &caller, now);

        application_id
    }

    #[endpoint(proposeOffer)]
    fn propose_offer(&self, job_id: u64, application_id: u64, terms: OfferTermsInput<Self::Api>) -> u64 {
        self.require_not_paused();

        let job = self.require_job(job_id);
        let application = self.require_application(job_id, application_id);
        require!(
            job.status != JobStatus::Matched
                && job.status != JobStatus::Closed
                && job.status != JobStatus::Expired,
            ERR_INVALID_STATE
        );

        let caller = self.blockchain().get_caller();
        require!(
            caller == job.employer || caller == application.applicant,
            ERR_UNAUTHORIZED
        );

        self.validate_offer_terms(&terms);
        self.require_no_active_latest_offer(job_id, application_id);

        let offer_id = self.next_offer_id(job_id);
        let party = if caller == job.employer {
            OfferParty::Employer
        } else {
            OfferParty::Worker
        };
        let counterparty = if caller == job.employer {
            application.applicant
        } else {
            job.employer
        };

        let offer = Offer {
            id: offer_id,
            job_id,
            application_id,
            proposer: caller.clone(),
            counterparty,
            party,
            parent_offer_id: 0,
            round_index: 0,
            terms: self.terms_input_to_terms(terms),
            status: OfferStatus::Proposed,
            created_at: self.blockchain().get_block_timestamp(),
        };

        self.offers(job_id, offer_id).set(offer);
        self.offer_count(job_id, application_id).update(|v| *v += 1);
        self.offers_by_application(job_id, application_id).push(&offer_id);
        self.latest_offer(job_id, application_id).set(offer_id);
        self.total_offer_count().update(|v| *v += 1);

        let ts = self.blockchain().get_block_timestamp();
        self.offer_proposed_event(job_id, offer_id, application_id, &caller, 0, ts);
        offer_id
    }

    #[endpoint(counterOffer)]
    fn counter_offer(
        &self,
        job_id: u64,
        offer_id: u64,
        terms: OfferTermsInput<Self::Api>,
    ) -> u64 {
        self.require_not_paused();
        self.validate_offer_terms(&terms);

        let mut prev_offer = self.require_offer(job_id, offer_id);
        require!(
            prev_offer.status == OfferStatus::Proposed || prev_offer.status == OfferStatus::Countered,
            ERR_INVALID_STATE
        );

        let caller = self.blockchain().get_caller();
        require!(caller == prev_offer.counterparty, ERR_UNAUTHORIZED);

        let latest = self.latest_offer(job_id, prev_offer.application_id).get();
        require!(latest == offer_id, ERR_STALE_OFFER);

        let counter_count = self.counter_count(job_id, prev_offer.application_id).get();
        require!(
            counter_count < self.max_counteroffers_per_application().get(),
            ERR_COUNTER_LIMIT
        );

        prev_offer.status = OfferStatus::Countered;
        self.offers(job_id, offer_id).set(prev_offer.clone());

        let new_offer_id = self.next_offer_id(job_id);
        let party = if caller == prev_offer.proposer {
            prev_offer.party
        } else if caller == prev_offer.counterparty {
            match prev_offer.party {
                OfferParty::Employer => OfferParty::Worker,
                OfferParty::Worker => OfferParty::Employer,
            }
        } else {
            prev_offer.party
        };

        let new_offer = Offer {
            id: new_offer_id,
            job_id,
            application_id: prev_offer.application_id,
            proposer: caller.clone(),
            counterparty: prev_offer.proposer,
            party,
            parent_offer_id: offer_id,
            round_index: prev_offer.round_index + 1,
            terms: self.terms_input_to_terms(terms),
            status: OfferStatus::Proposed,
            created_at: self.blockchain().get_block_timestamp(),
        };

        self.offers(job_id, new_offer_id).set(new_offer);
        self.offer_count(job_id, prev_offer.application_id)
            .update(|v| *v += 1);
        self.offers_by_application(job_id, prev_offer.application_id)
            .push(&new_offer_id);
        self.latest_offer(job_id, prev_offer.application_id)
            .set(new_offer_id);
        self.counter_count(job_id, prev_offer.application_id)
            .set(counter_count + 1);
        self.total_offer_count().update(|v| *v += 1);

        let ts = self.blockchain().get_block_timestamp();
        self.offer_proposed_event(
            job_id,
            new_offer_id,
            prev_offer.application_id,
            &caller,
            offer_id,
            ts,
        );

        new_offer_id
    }

    #[endpoint(rejectOffer)]
    fn reject_offer(&self, job_id: u64, offer_id: u64) {
        self.require_not_paused();
        let mut offer = self.require_offer(job_id, offer_id);
        require!(
            offer.status == OfferStatus::Proposed || offer.status == OfferStatus::Countered,
            ERR_INVALID_STATE
        );

        let latest = self.latest_offer(job_id, offer.application_id).get();
        require!(latest == offer_id, ERR_STALE_OFFER);

        let caller = self.blockchain().get_caller();
        require!(caller == offer.counterparty, ERR_UNAUTHORIZED);

        offer.status = OfferStatus::Rejected;
        self.offers(job_id, offer_id).set(offer);

        let ts = self.blockchain().get_block_timestamp();
        self.offer_rejected_event(job_id, offer_id, &caller, ts);
    }

    #[endpoint(withdrawOffer)]
    fn withdraw_offer(&self, job_id: u64, offer_id: u64) {
        self.require_not_paused();
        let mut offer = self.require_offer(job_id, offer_id);
        require!(
            offer.status == OfferStatus::Proposed || offer.status == OfferStatus::Countered,
            ERR_INVALID_STATE
        );

        let latest = self.latest_offer(job_id, offer.application_id).get();
        require!(latest == offer_id, ERR_STALE_OFFER);

        let caller = self.blockchain().get_caller();
        require!(caller == offer.proposer, ERR_UNAUTHORIZED);

        offer.status = OfferStatus::Withdrawn;
        self.offers(job_id, offer_id).set(offer);

        let ts = self.blockchain().get_block_timestamp();
        self.offer_withdrawn_event(job_id, offer_id, &caller, ts);
    }

    #[endpoint(acceptOffer)]
    fn accept_offer(&self, job_id: u64, offer_id: u64) {
        self.require_not_paused();
        let mut job = self.require_job(job_id);
        require!(job.status != JobStatus::Matched, ERR_ALREADY_MATCHED);

        let mut offer = self.require_offer(job_id, offer_id);
        require!(
            offer.status == OfferStatus::Proposed || offer.status == OfferStatus::Countered,
            ERR_INVALID_STATE
        );

        let latest = self.latest_offer(job_id, offer.application_id).get();
        require!(latest == offer_id, ERR_STALE_OFFER);

        let caller = self.blockchain().get_caller();
        require!(caller == offer.counterparty, ERR_UNAUTHORIZED);

        offer.status = OfferStatus::Accepted;
        self.offers(job_id, offer_id).set(offer);

        self.accepted_offer_id(job_id).set(offer_id);
        self.accepted_offer_timestamp(job_id)
            .set(self.blockchain().get_block_timestamp());

        job.status = JobStatus::Matched;
        job.accepted_offer_id = offer_id;
        self.jobs(job_id).set(job);
        self.matched_job_count().update(|v| *v += 1);

        let ts = self.blockchain().get_block_timestamp();
        self.offer_accepted_event(job_id, offer_id, &caller, ts);
    }

    #[endpoint(cancelJob)]
    fn cancel_job(&self, job_id: u64) {
        self.require_not_paused();
        let mut job = self.require_job(job_id);
        let caller = self.blockchain().get_caller();
        require!(caller == job.employer, ERR_UNAUTHORIZED);

        require!(
            !(job.status == JobStatus::Matched && job.accepted_offer_id != 0),
            ERR_ALREADY_MATCHED
        );
        require!(
            job.status != JobStatus::Closed && job.status != JobStatus::Expired,
            ERR_INVALID_STATE
        );

        if job.status == JobStatus::Open {
            self.open_job_count().update(|v| {
                if *v > 0 {
                    *v -= 1;
                }
            });
        }
        if job.status == JobStatus::Matched {
            self.matched_job_count().update(|v| {
                if *v > 0 {
                    *v -= 1;
                }
            });
        }

        job.status = JobStatus::Closed;
        self.jobs(job_id).set(job);

        let ts = self.blockchain().get_block_timestamp();
        self.job_closed_event(job_id, &caller, JobCloseReason::Cancelled, ts);
    }

    #[endpoint(expireJob)]
    fn expire_job(&self, job_id: u64) {
        self.require_not_paused();
        let mut job = self.require_job(job_id);
        require!(
            job.status == JobStatus::Open || job.status == JobStatus::InNegotiation,
            ERR_INVALID_STATE
        );
        require!(
            self.blockchain().get_block_timestamp() > job.application_deadline_ts,
            ERR_INVALID_DEADLINE
        );

        if job.status == JobStatus::Open {
            self.open_job_count().update(|v| {
                if *v > 0 {
                    *v -= 1;
                }
            });
        }

        job.status = JobStatus::Expired;
        self.jobs(job_id).set(job);

        let caller = self.blockchain().get_caller();
        let ts = self.blockchain().get_block_timestamp();
        self.job_closed_event(job_id, &caller, JobCloseReason::Expired, ts);
    }

    #[endpoint(setMinUptimeScore)]
    fn set_min_uptime_score(&self, value: u64) {
        self.require_owner();
        self.min_uptime_score().set(value);
    }

    #[endpoint(setMaxCounteroffersPerApplication)]
    fn set_max_counteroffers_per_application(&self, value: u64) {
        self.require_owner();
        require!(value > 0, ERR_INVALID_AMOUNT);
        self.max_counteroffers_per_application().set(value);
    }

    #[endpoint(setMaxInvitesPerJob)]
    fn set_max_invites_per_job(&self, value: u64) {
        self.require_owner();
        require!(value > 0, ERR_INVALID_AMOUNT);
        self.max_invites_per_job().set(value);
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

    #[view(getJob)]
    fn get_job(&self, job_id: u64) -> OptionalValue<Job<Self::Api>> {
        if self.jobs(job_id).is_empty() {
            OptionalValue::None
        } else {
            OptionalValue::Some(self.jobs(job_id).get())
        }
    }

    #[view(getApplication)]
    fn get_application(&self, job_id: u64, application_id: u64) -> OptionalValue<Application<Self::Api>> {
        if self.applications(job_id, application_id).is_empty() {
            OptionalValue::None
        } else {
            OptionalValue::Some(self.applications(job_id, application_id).get())
        }
    }

    #[view(getOffer)]
    fn get_offer(&self, job_id: u64, offer_id: u64) -> OptionalValue<Offer<Self::Api>> {
        if self.offers(job_id, offer_id).is_empty() {
            OptionalValue::None
        } else {
            OptionalValue::Some(self.offers(job_id, offer_id).get())
        }
    }

    #[view(getApplications)]
    fn get_applications(&self, job_id: u64, from: u64, size: u64) -> MultiValueEncoded<Application<Self::Api>> {
        let mut out = MultiValueEncoded::new();
        let count = self.application_count(job_id).get();
        if count == 0 {
            return out;
        }

        let effective_size = core::cmp::min(size, MAX_PAGE_SIZE);
        if effective_size == 0 {
            return out;
        }

        let mut idx = from + 1;
        let mut emitted = 0;
        while idx <= count && emitted < effective_size {
            if !self.applications(job_id, idx).is_empty() {
                out.push(self.applications(job_id, idx).get());
            }
            idx += 1;
            emitted += 1;
        }
        out
    }

    #[view(getOffers)]
    fn get_offers(
        &self,
        job_id: u64,
        application_id: u64,
        from: u64,
        size: u64,
    ) -> MultiValueEncoded<Offer<Self::Api>> {
        let mut out = MultiValueEncoded::new();
        let list = self.offers_by_application(job_id, application_id);
        let total = list.len() as u64;
        if total == 0 {
            return out;
        }

        let effective_size = core::cmp::min(size, MAX_PAGE_SIZE);
        if effective_size == 0 {
            return out;
        }

        let mut emitted = 0;
        let mut idx = from + 1;
        while idx <= total && emitted < effective_size {
            let offer_id = list.get(idx as usize);
            if !self.offers(job_id, offer_id).is_empty() {
                out.push(self.offers(job_id, offer_id).get());
            }
            idx += 1;
            emitted += 1;
        }
        out
    }

    #[view(getAcceptedOffer)]
    fn get_accepted_offer(&self, job_id: u64) -> OptionalValue<AcceptedOfferSummary<Self::Api>> {
        if self.accepted_offer_id(job_id).is_empty() {
            return OptionalValue::None;
        }
        let offer_id = self.accepted_offer_id(job_id).get();
        if offer_id == 0 || self.offers(job_id, offer_id).is_empty() || self.jobs(job_id).is_empty() {
            return OptionalValue::None;
        }

        let offer = self.offers(job_id, offer_id).get();
        let job = self.jobs(job_id).get();
        let worker = self.application_applicant(job_id, offer.application_id);

        OptionalValue::Some(AcceptedOfferSummary {
            job_id,
            offer_id,
            employer: job.employer,
            worker,
            terms: offer.terms,
            accepted_at: self.accepted_offer_timestamp(job_id).get(),
        })
    }

    #[view(isInviteAllowed)]
    fn is_invite_allowed(&self, job_id: u64, addr: ManagedAddress) -> bool {
        if self.jobs(job_id).is_empty() {
            return false;
        }
        let job = self.jobs(job_id).get();
        if job.visibility == JobVisibility::Public {
            return true;
        }
        self.invites(job_id).contains(&addr)
    }

    #[view(getBoardStats)]
    fn get_board_stats(&self) -> BoardStats {
        BoardStats {
            total_jobs: self.job_count().get(),
            open_jobs: self.open_job_count().get(),
            matched_jobs: self.matched_job_count().get(),
            total_applications: self.total_application_count().get(),
            total_offers: self.total_offer_count().get(),
        }
    }

    #[view(getConfig)]
    fn get_config(&self) -> JobBoardConfig<Self::Api> {
        JobBoardConfig {
            owner: self.owner().get(),
            bond_registry: self.bond_registry().get(),
            uptime: self.uptime().get(),
            min_uptime_score: self.min_uptime_score().get(),
            max_counteroffers_per_application: self.max_counteroffers_per_application().get(),
            max_invites_per_job: self.max_invites_per_job().get(),
            paused: self.paused().get(),
        }
    }

    fn require_job(&self, job_id: u64) -> Job<Self::Api> {
        require!(!self.jobs(job_id).is_empty(), ERR_INVALID_STATE);
        self.jobs(job_id).get()
    }

    fn require_application(&self, job_id: u64, application_id: u64) -> Application<Self::Api> {
        require!(
            !self.applications(job_id, application_id).is_empty(),
            ERR_INVALID_STATE
        );
        self.applications(job_id, application_id).get()
    }

    fn require_offer(&self, job_id: u64, offer_id: u64) -> Offer<Self::Api> {
        require!(!self.offers(job_id, offer_id).is_empty(), ERR_INVALID_STATE);
        self.offers(job_id, offer_id).get()
    }

    fn require_owner(&self) {
        require!(self.blockchain().get_caller() == self.owner().get(), ERR_UNAUTHORIZED);
    }

    fn require_not_paused(&self) {
        require!(!self.paused().get(), ERR_PAUSED);
    }

    fn application_applicant(&self, job_id: u64, application_id: u64) -> ManagedAddress {
        self.require_application(job_id, application_id).applicant
    }

    fn next_offer_id(&self, job_id: u64) -> u64 {
        let id = self.job_offer_seq(job_id).get() + 1;
        self.job_offer_seq(job_id).set(id);
        id
    }

    fn terms_input_to_terms(&self, terms: OfferTermsInput<Self::Api>) -> OfferTerms<Self::Api> {
        OfferTerms {
            recurring: terms.recurring,
            revenue_share: terms.revenue_share,
            employer_bond_required: terms.employer_bond_required,
            worker_bond_required: terms.worker_bond_required,
            milestones: terms.milestones,
            terms_uri: terms.terms_uri,
        }
    }

    fn validate_offer_terms(&self, terms: &OfferTermsInput<Self::Api>) {
        require!(
            terms.revenue_share.profit_share_bps <= BPS_DENOMINATOR,
            ERR_INVALID_BPS
        );
        require!(terms.terms_uri.len() <= MAX_TERMS_URI_LEN, ERR_INVALID_AMOUNT);
        require!(
            terms.milestones.len() <= MAX_MILESTONES_PER_OFFER,
            ERR_INVALID_AMOUNT
        );

        if terms.recurring.amount_per_period > 0u64 {
            require!(terms.recurring.period_seconds > 0, ERR_INVALID_AMOUNT);
            require!(terms.recurring.total_periods > 0, ERR_INVALID_AMOUNT);
        } else {
            require!(terms.recurring.total_periods == 0, ERR_INVALID_AMOUNT);
        }

        for m in terms.milestones.iter() {
            self.validate_milestone_spec(&m);
        }
    }

    fn validate_milestone_spec(&self, milestone: &MilestoneSpec<Self::Api>) {
        require!(milestone.amount > 0u64, ERR_INVALID_AMOUNT);
        require!(milestone.review_timeout_seconds > 0, ERR_INVALID_AMOUNT);
        require!(
            milestone.metadata_uri.len() <= MAX_METADATA_URI_LEN,
            ERR_INVALID_AMOUNT
        );
    }

    fn require_no_active_latest_offer(&self, job_id: u64, application_id: u64) {
        if self.latest_offer(job_id, application_id).is_empty() {
            return;
        }
        let latest_offer_id = self.latest_offer(job_id, application_id).get();
        if latest_offer_id == 0 || self.offers(job_id, latest_offer_id).is_empty() {
            return;
        }
        let latest_offer = self.offers(job_id, latest_offer_id).get();
        require!(
            latest_offer.status != OfferStatus::Proposed
                && latest_offer.status != OfferStatus::Countered,
            ERR_STALE_OFFER
        );
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

    #[event("jobCreated")]
    fn job_created_event(
        &self,
        #[indexed] job_id: u64,
        #[indexed] employer: &ManagedAddress,
        #[indexed] visibility: JobVisibility,
        timestamp: u64,
    );

    #[event("applicationSubmitted")]
    fn application_submitted_event(
        &self,
        #[indexed] job_id: u64,
        #[indexed] application_id: u64,
        #[indexed] applicant: &ManagedAddress,
        timestamp: u64,
    );

    #[event("offerProposed")]
    fn offer_proposed_event(
        &self,
        #[indexed] job_id: u64,
        #[indexed] offer_id: u64,
        #[indexed] application_id: u64,
        #[indexed] proposer: &ManagedAddress,
        #[indexed] parent_offer_id: u64,
        timestamp: u64,
    );

    #[event("offerRejected")]
    fn offer_rejected_event(
        &self,
        #[indexed] job_id: u64,
        #[indexed] offer_id: u64,
        #[indexed] by: &ManagedAddress,
        timestamp: u64,
    );

    #[event("offerWithdrawn")]
    fn offer_withdrawn_event(
        &self,
        #[indexed] job_id: u64,
        #[indexed] offer_id: u64,
        #[indexed] by: &ManagedAddress,
        timestamp: u64,
    );

    #[event("offerAccepted")]
    fn offer_accepted_event(
        &self,
        #[indexed] job_id: u64,
        #[indexed] offer_id: u64,
        #[indexed] accepter: &ManagedAddress,
        timestamp: u64,
    );

    #[event("jobClosed")]
    fn job_closed_event(
        &self,
        #[indexed] job_id: u64,
        #[indexed] closer: &ManagedAddress,
        #[indexed] close_reason: JobCloseReason,
        timestamp: u64,
    );

    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("paused")]
    fn paused(&self) -> SingleValueMapper<bool>;

    #[storage_mapper("bondRegistry")]
    fn bond_registry(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("uptime")]
    fn uptime(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("minUptimeScore")]
    fn min_uptime_score(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("maxCounteroffersPerApplication")]
    fn max_counteroffers_per_application(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("maxInvitesPerJob")]
    fn max_invites_per_job(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("jobCount")]
    fn job_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("totalApplicationCount")]
    fn total_application_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("totalOfferCount")]
    fn total_offer_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("openJobCount")]
    fn open_job_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("matchedJobCount")]
    fn matched_job_count(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("jobs")]
    fn jobs(&self, job_id: u64) -> SingleValueMapper<Job<Self::Api>>;

    #[storage_mapper("jobIdsByEmployer")]
    fn job_ids_by_employer(&self, employer: &ManagedAddress) -> VecMapper<u64>;

    #[storage_mapper("invites")]
    fn invites(&self, job_id: u64) -> UnorderedSetMapper<ManagedAddress>;

    #[storage_mapper("applicationCount")]
    fn application_count(&self, job_id: u64) -> SingleValueMapper<u64>;

    #[storage_mapper("applications")]
    fn applications(&self, job_id: u64, application_id: u64) -> SingleValueMapper<Application<Self::Api>>;

    #[storage_mapper("hasApplied")]
    fn has_applied(&self, job_id: u64, applicant: &ManagedAddress) -> SingleValueMapper<bool>;

    #[storage_mapper("offerCount")]
    fn offer_count(&self, job_id: u64, application_id: u64) -> SingleValueMapper<u64>;

    #[storage_mapper("offers")]
    fn offers(&self, job_id: u64, offer_id: u64) -> SingleValueMapper<Offer<Self::Api>>;

    #[storage_mapper("latestOffer")]
    fn latest_offer(&self, job_id: u64, application_id: u64) -> SingleValueMapper<u64>;

    #[storage_mapper("acceptedOfferId")]
    fn accepted_offer_id(&self, job_id: u64) -> SingleValueMapper<u64>;

    #[storage_mapper("acceptedOfferTimestamp")]
    fn accepted_offer_timestamp(&self, job_id: u64) -> SingleValueMapper<u64>;

    #[storage_mapper("counterCount")]
    fn counter_count(&self, job_id: u64, application_id: u64) -> SingleValueMapper<u64>;

    #[storage_mapper("jobIndex")]
    fn job_index(&self) -> VecMapper<u64>;

    #[storage_mapper("jobOfferSeq")]
    fn job_offer_seq(&self, job_id: u64) -> SingleValueMapper<u64>;

    #[storage_mapper("offersByApplication")]
    fn offers_by_application(&self, job_id: u64, application_id: u64) -> VecMapper<u64>;
}
