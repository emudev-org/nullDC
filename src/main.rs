mod debugger_server_main;
mod mock_debug_server;

fn main() {
    // Start debugger UI server with integrated WebSocket support (no-op on WASM)
    debugger_server_main::start_debugger_server();

    pollster::block_on(nulldc::run());
}

