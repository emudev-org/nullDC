#[cfg(not(target_arch = "wasm32"))]
mod debugger_server_main;
#[cfg(not(target_arch = "wasm32"))]
mod mock_debug_server;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    debugger_server_main::start_debugger_server();

    pollster::block_on(nulldc::run());
}