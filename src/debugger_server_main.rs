#[cfg(not(target_arch = "wasm32"))]
use include_dir::{include_dir, Dir};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;

#[cfg(not(target_arch = "wasm32"))]
static DEBUGGER_UI: Dir = include_dir!("$CARGO_MANIFEST_DIR/debugger-ui/dist");

/// Start the debugger UI HTTP server on port 9999
/// The server runs in a background thread and serves static files
#[cfg(not(target_arch = "wasm32"))]
pub fn start_debugger_server() {
    thread::spawn(|| {
        let server = tiny_http::Server::http("127.0.0.1:9999").unwrap();
        log::info!("Debugger UI server started at http://127.0.0.1:9999");

        for request in server.incoming_requests() {
            let url_path = request.url();

            // Handle root path
            let file_path = if url_path == "/" {
                "index.html"
            } else {
                // Remove leading slash
                &url_path[1..]
            };

            // Try to find the file in the embedded directory
            if let Some(file) = DEBUGGER_UI.get_file(file_path) {
                let content_type = match file_path.split('.').last() {
                    Some("html") => "text/html",
                    Some("js") => "application/javascript",
                    Some("css") => "text/css",
                    Some("png") => "image/png",
                    Some("jpg") | Some("jpeg") => "image/jpeg",
                    Some("svg") => "image/svg+xml",
                    Some("wasm") => "application/wasm",
                    _ => "application/octet-stream",
                };

                let response = tiny_http::Response::from_data(file.contents())
                    .with_header(
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap()
                    )
                    .with_header(
                        tiny_http::Header::from_bytes(&b"Cache-Control"[..], &b"no-cache"[..]).unwrap()
                    );

                let _ = request.respond(response);
            } else {
                // File not found
                let response = tiny_http::Response::from_string("404 Not Found")
                    .with_status_code(404);
                let _ = request.respond(response);
            }
        }
    });
}

/// No-op for WASM builds (debugger uses BroadcastChannel instead)
#[cfg(target_arch = "wasm32")]
pub fn start_debugger_server() {
    // No HTTP server needed for WASM - uses BroadcastChannel
}
