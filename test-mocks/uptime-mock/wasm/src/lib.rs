#![no_std]

multiversx_sc_wasm_adapter::allocator!();
multiversx_sc_wasm_adapter::panic_handler!();

multiversx_sc_wasm_adapter::endpoints! {
    uptime_mock
    (
        init => init
        setLifetimeInfo => set_lifetime_info
        getLifetimeInfo => get_lifetime_info
    )
}

multiversx_sc_wasm_adapter::async_callback_empty! {}
