use axum::{
    Router,
    extract::ws::{WebSocket, WebSocketUpgrade},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use include_dir::{Dir, include_dir};
use nulldc::dreamcast::Dreamcast;

static DEBUGGER_UI: Dir = include_dir!("$CARGO_MANIFEST_DIR/devtools/dist-native");

/// Start the debugger UI HTTP server on port 9999
/// The server runs in a background thread and serves static files
/// Also handles WebSocket connections for the debugger protocol
pub fn start_debugger_server(dreamcast: *mut Dreamcast) {
    use std::thread;

    let dc_ptr = dreamcast as usize;

    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let app = Router::new()
                .route(
                    "/ws",
                    get(move |ws: WebSocketUpgrade| websocket_handler(ws, dc_ptr)),
                )
                .fallback(static_file_handler);

            let listener = tokio::net::TcpListener::bind("127.0.0.1:55543")
                .await
                .unwrap();

            println!(
                "Debugger UI server started at http://{}",
                listener.local_addr().unwrap()
            );

            axum::serve(listener, app).await.unwrap();
        });
    });
}

async fn websocket_handler(ws: WebSocketUpgrade, dc_ptr: usize) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, dc_ptr))
}

async fn handle_websocket(socket: WebSocket, dc_ptr: usize) {
    crate::mock_debug_server::handle_websocket_connection(socket, dc_ptr).await;
}

async fn static_file_handler(uri: axum::http::Uri) -> Response {
    let path = uri.path();

    // If path does not start with /assets/, always return index.html
    let file_path = if path == "/" || !path.starts_with("/assets/") {
        "index.html"
    } else {
        // Strip the leading slash for assets
        &path[1..]
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

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CACHE_CONTROL, "no-cache")
            .body(axum::body::Body::from(file.contents()))
            .unwrap()
    } else {
        (StatusCode::NOT_FOUND, "404 Not Found").into_response()
    }
}

/// No-op for WASM builds (debugger uses BroadcastChannel instead)
#[cfg(target_arch = "wasm32")]
pub fn start_debugger_server() {
    // No HTTP server needed for WASM - uses BroadcastChannel
}
