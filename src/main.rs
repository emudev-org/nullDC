mod debugger_server_main;

fn main() {
    // Start debugger UI server (no-op on WASM)
    debugger_server_main::start_debugger_server();

    pollster::block_on(nulldc::run());
}

