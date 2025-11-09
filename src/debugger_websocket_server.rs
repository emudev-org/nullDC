use axum::{
    Router,
    extract::ws::{WebSocket, WebSocketUpgrade, Message},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use futures::SinkExt;
use futures::stream::StreamExt;
use include_dir::{Dir, include_dir};
use crate::dreamcast::Dreamcast;
use std::sync::Arc;

use crate::debugger_core::{
    handle_request, JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcSuccess,
    ServerState,
};

static DEBUGGER_UI: Dir = include_dir!("$CARGO_MANIFEST_DIR/devtools/dist-native");

const JSON_RPC_VERSION: &str = "2.0";

/// Send binary data over WebSocket
async fn send_binary(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    data: Vec<u8>,
) -> Result<(), axum::Error> {
    sender.send(Message::Binary(data)).await
}

/// Start the debugger UI HTTP server on port 55543
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

async fn handle_websocket(socket: WebSocket, dreamcast_ptr: usize) {
    use std::sync::OnceLock;
    static STATE: OnceLock<Arc<ServerState>> = OnceLock::new();
    let state = STATE.get_or_init(|| Arc::new(ServerState::new())).clone();

    let (mut sender, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&text) {
                    let id = request.id.clone();
                    match handle_request(state.clone(), dreamcast_ptr, request) {
                        Ok((result, should_broadcast)) => {
                            let response = JsonRpcSuccess {
                                jsonrpc: JSON_RPC_VERSION.to_string(),
                                id,
                                result,
                            };
                            if let Ok(json) = serde_json::to_string(&response) {
                                let _ = sender.send(Message::Text(json.into())).await;
                            }

                            if should_broadcast {
                                let tick = state.build_tick(dreamcast_ptr, None);
                                let notification = JsonRpcNotification {
                                    jsonrpc: JSON_RPC_VERSION.to_string(),
                                    method: "event.tick".to_string(),
                                    params: serde_json::to_value(tick).unwrap(),
                                };

                                if let Ok(json) = serde_json::to_string(&notification) {
                                    let _ = sender.send(Message::Text(json.into())).await;
                                }
                            }
                        }
                        Err(error) => {
                            let response = JsonRpcError {
                                jsonrpc: JSON_RPC_VERSION.to_string(),
                                id,
                                error,
                            };
                            if let Ok(json) = serde_json::to_string(&response) {
                                let _ = sender.send(Message::Text(json.into())).await;
                            }
                        }
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
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
