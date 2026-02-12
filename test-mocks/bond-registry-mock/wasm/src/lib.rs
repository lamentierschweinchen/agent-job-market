#![no_std]

multiversx_sc_wasm_adapter::allocator!();
multiversx_sc_wasm_adapter::panic_handler!();

multiversx_sc_wasm_adapter::endpoints! {
    bond_registry_mock
    (
        init => init
        setAgentName => set_agent_name
        getAgentName => get_agent_name
    )
}

multiversx_sc_wasm_adapter::async_callback_empty! {}
