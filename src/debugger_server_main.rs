#[cfg(not(target_arch = "wasm32"))]
use include_dir::{include_dir, Dir};
#[cfg(not(target_arch = "wasm32"))]
use axum::{
    routing::get,
    Router,
    response::{Response, IntoResponse},
    http::{StatusCode, header},
    extract::ws::{WebSocketUpgrade, WebSocket},
};

#[cfg(not(target_arch = "wasm32"))]
static DEBUGGER_UI: Dir = include_dir!("$CARGO_MANIFEST_DIR/debugger-ui/dist");

/// Start the debugger UI HTTP server on port 9999
/// The server runs in a background thread and serves static files
/// Also handles WebSocket connections for the debugger protocol
#[cfg(not(target_arch = "wasm32"))]
pub fn start_debugger_server() {
    use std::thread;

    thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let app = Router::new()
                .route("/ws", get(websocket_handler))
                .fallback(static_file_handler);

            let listener = tokio::net::TcpListener::bind("127.0.0.1:9999")
                .await
                .unwrap();

            log::info!("Debugger UI server started at http://127.0.0.1:9999");
            log::info!("WebSocket endpoint available at ws://127.0.0.1:9999/ws");

            axum::serve(listener, app).await.unwrap();
        });
    });
}

#[cfg(not(target_arch = "wasm32"))]
async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_websocket)
}

#[cfg(not(target_arch = "wasm32"))]
async fn handle_websocket(socket: WebSocket) {
    crate::mock_debug_server::handle_websocket_connection(socket).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn static_file_handler(uri: axum::http::Uri) -> Response {

    let path = uri.path();
    let file_path = if path == "/" {
        "index.html"
    } else {
        // Remove leading slash
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
