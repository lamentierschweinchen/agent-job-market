#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[multiversx_sc::contract]
pub trait BondRegistryMock {
    #[init]
    fn init(&self) {
        self.owner().set(self.blockchain().get_caller());
    }

    #[upgrade]
    fn upgrade(&self) {}

    #[endpoint(setAgentName)]
    fn set_agent_name(&self, agent: ManagedAddress, name: ManagedBuffer) {
        self.require_owner();
        self.agent_name(&agent).set(name);
    }

    #[view(getAgentName)]
    fn get_agent_name(&self, agent: ManagedAddress) -> ManagedBuffer {
        if self.agent_name(&agent).is_empty() {
            ManagedBuffer::new()
        } else {
            self.agent_name(&agent).get()
        }
    }

    fn require_owner(&self) {
        require!(
            self.blockchain().get_caller() == self.owner().get(),
            "ERR_UNAUTHORIZED"
        );
    }

    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("agentName")]
    fn agent_name(&self, agent: &ManagedAddress) -> SingleValueMapper<ManagedBuffer>;
}
