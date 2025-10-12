#[cfg(not(target_arch = "wasm32"))]
mod debugger_server_main;
#[cfg(not(target_arch = "wasm32"))]
mod mock_debug_server;

use nulldc::dreamcast::{Dreamcast, init_dreamcast};

fn main() {
    let dreamcast = Box::into_raw(Box::new(Dreamcast::default()));

    init_dreamcast(dreamcast);
    #[cfg(not(target_arch = "wasm32"))]
    debugger_server_main::start_debugger_server(dreamcast);

    pollster::block_on(nulldc::run(dreamcast));
}
