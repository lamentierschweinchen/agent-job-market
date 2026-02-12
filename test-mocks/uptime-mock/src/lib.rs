#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct LifetimeInfo {
    pub total_heartbeats: u64,
    pub lifetime_score: u64,
    pub time_since_last: u64,
    pub time_remaining: u64,
}

#[multiversx_sc::contract]
pub trait UptimeMock {
    #[init]
    fn init(&self) {
        self.owner().set(self.blockchain().get_caller());
    }

    #[upgrade]
    fn upgrade(&self) {}

    #[endpoint(setLifetimeInfo)]
    fn set_lifetime_info(
        &self,
        agent: ManagedAddress,
        total_heartbeats: u64,
        lifetime_score: u64,
        time_since_last: u64,
        time_remaining: u64,
    ) {
        self.require_owner();
        let info = LifetimeInfo {
            total_heartbeats,
            lifetime_score,
            time_since_last,
            time_remaining,
        };
        self.lifetime_info(&agent).set(info);
    }

    #[view(getLifetimeInfo)]
    fn get_lifetime_info(&self, agent: ManagedAddress) -> MultiValue4<u64, u64, u64, u64> {
        let info = if self.lifetime_info(&agent).is_empty() {
            LifetimeInfo {
                total_heartbeats: 0,
                lifetime_score: 0,
                time_since_last: 0,
                time_remaining: 0,
            }
        } else {
            self.lifetime_info(&agent).get()
        };
        (
            info.total_heartbeats,
            info.lifetime_score,
            info.time_since_last,
            info.time_remaining,
        )
            .into()
    }

    fn require_owner(&self) {
        require!(
            self.blockchain().get_caller() == self.owner().get(),
            "ERR_UNAUTHORIZED"
        );
    }

    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("lifetimeInfo")]
    fn lifetime_info(&self, agent: &ManagedAddress) -> SingleValueMapper<LifetimeInfo>;
}
