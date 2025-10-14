// this module must only ever be compiled for wasm32-unknown-unknown target

// Broadcast channel debugger server for WASM builds
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::BroadcastChannel;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::dreamcast::Dreamcast;
use crate::debugger_core::{JsonRpcRequest, JsonRpcSuccess, JsonRpcError, JsonRpcNotification, ServerState};

// Generate a simple GUID for this instance using JS Date API
fn generate_guid() -> String {
    // Use JavaScript's Date.now() instead of SystemTime (not available in WASM)
    let timestamp = js_sys::Date::now() as u64;
    let random = (js_sys::Math::random() * 1000000.0) as u64;
    format!("{:x}{:x}", timestamp, random)
}

#[derive(Serialize, Deserialize)]
struct Announcement {
    id: String,
    name: String,
    timestamp: u64,
}

pub struct BroadcastDebugServer {
    guid: String,
    announcement_channel: BroadcastChannel,
    communication_channel: BroadcastChannel,
    announcement_interval: Option<i32>,
    dreamcast_ptr: usize,
    state: Arc<ServerState>,
}

impl BroadcastDebugServer {
    pub fn new(dreamcast: *mut Dreamcast) -> Result<Self, JsValue> {
        let guid = generate_guid();
        log::info!("Creating broadcast debug server with GUID: {}", guid);

        // Create announcement channel
        let announcement_channel = BroadcastChannel::new("nulldc-debugger-announce")?;

        // Create communication channel (per-instance)
        let comm_channel_name = format!("nulldc-debugger-{}", guid);
        let communication_channel = BroadcastChannel::new(&comm_channel_name)?;

        // Create server state
        let state = Arc::new(ServerState::new());

        Ok(Self {
            guid,
            announcement_channel,
            communication_channel,
            announcement_interval: None,
            dreamcast_ptr: dreamcast as usize,
            state,
        })
    }

    pub fn start(&mut self) -> Result<(), JsValue> {
        log::info!("Starting broadcast debug server");

        // Start announcement broadcaster
        self.start_announcements()?;

        // Setup communication channel message handler
        self.setup_communication_handler()?;

        Ok(())
    }

    fn start_announcements(&mut self) -> Result<(), JsValue> {
        let guid = self.guid.clone();
        let announcement_channel = self.announcement_channel.clone();

        // Send initial announcement immediately
        let announcement = Announcement {
            id: guid.clone(),
            name: format!("nullDC Instance {}", &guid[..8]),
            timestamp: js_sys::Date::now() as u64,
        };
        if let Ok(json) = serde_json::to_string(&announcement) {
            let _ = announcement_channel.post_message(&JsValue::from_str(&json));
        }

        let announce_callback = Closure::<dyn Fn()>::new(move || {
            log::debug!("Announcement interval fired");
            let announcement = Announcement {
                id: guid.clone(),
                name: format!("nullDC Instance {}", &guid[..8]),
                timestamp: js_sys::Date::now() as u64,
            };

            if let Ok(json) = serde_json::to_string(&announcement) {
                log::debug!("Posting announcement: {}", json);
                // Post the JSON string directly
                if let Err(e) = announcement_channel.post_message(&JsValue::from_str(&json)) {
                    log::error!("Failed to post announcement: {:?}", e);
                }
            } else {
                log::error!("Failed to serialize announcement");
            }
        });

        // Announce every 1 second
        let interval_id = web_sys::window()
            .unwrap()
            .set_interval_with_callback_and_timeout_and_arguments_0(
                announce_callback.as_ref().unchecked_ref(),
                1000,
            )?;

        // Store the interval ID so we can clear it later
        self.announcement_interval = Some(interval_id);

        // Keep the closure alive
        announce_callback.forget();

        Ok(())
    }

    fn setup_communication_handler(&self) -> Result<(), JsValue> {
        let communication_channel = self.communication_channel.clone();
        let dreamcast_ptr = self.dreamcast_ptr;
        let state = self.state.clone();

        // Handle incoming messages
        let message_callback = Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
            log::debug!("Communication channel received message");
            let data = event.data();

            // Check if it's a ping message
            if let Some(msg) = data.as_string() {
                log::debug!("Received string message: {}", msg);
                if msg == "ping" {
                    log::debug!("Responding to ping with pong");
                    // Respond with pong
                    if let Err(e) = communication_channel.post_message(&JsValue::from_str("pong")) {
                        log::error!("Failed to send pong: {:?}", e);
                    }
                    return;
                }

                // Handle JSON-RPC messages
                log::debug!("Received JSON-RPC message: {}", msg);

                // Parse and handle the RPC request
                if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&msg) {
                    let id = request.id.clone();
                    let method = request.method.clone();
                    log::debug!("Processing RPC method: {}", method);

                    // Call the shared RPC handler
                    match crate::debugger_core::handle_request(state.clone(), dreamcast_ptr, request) {
                        Ok((result, should_broadcast)) => {
                            // Send success response
                            let response = JsonRpcSuccess {
                                jsonrpc: "2.0".to_string(),
                                id,
                                result,
                            };

                            if let Ok(response_str) = serde_json::to_string(&response) {
                                if let Err(e) = communication_channel.post_message(&JsValue::from_str(&response_str)) {
                                    log::error!("Failed to send RPC response: {:?}", e);
                                }
                            }

                            // Send tick notification if needed
                            if should_broadcast {
                                let tick = state.build_tick(dreamcast_ptr, None);
                                let notification = JsonRpcNotification {
                                    jsonrpc: "2.0".to_string(),
                                    method: "event.tick".to_string(),
                                    params: serde_json::to_value(tick).unwrap(),
                                };

                                if let Ok(notification_str) = serde_json::to_string(&notification) {
                                    if let Err(e) = communication_channel.post_message(&JsValue::from_str(&notification_str)) {
                                        log::error!("Failed to send tick notification: {:?}", e);
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            // Send error response
                            let response = JsonRpcError {
                                jsonrpc: "2.0".to_string(),
                                id,
                                error,
                            };

                            if let Ok(response_str) = serde_json::to_string(&response) {
                                if let Err(e) = communication_channel.post_message(&JsValue::from_str(&response_str)) {
                                    log::error!("Failed to send RPC error: {:?}", e);
                                }
                            }
                        }
                    }
                } else {
                    log::error!("Failed to parse JSON-RPC request: {}", msg);
                }
            } else {
                log::warn!("Received non-string message");
            }
        });

        self.communication_channel
            .set_onmessage(Some(message_callback.as_ref().unchecked_ref()));

        // Keep the closure alive
        message_callback.forget();

        Ok(())
    }

    pub fn stop(&mut self) {
        log::info!("Stopping broadcast debug server");

        // Clear announcement interval
        if let Some(interval_id) = self.announcement_interval.take() {
            web_sys::window().unwrap().clear_interval_with_handle(interval_id);
        }

        // Close channels
        self.announcement_channel.close();
        self.communication_channel.close();
    }
}

impl Drop for BroadcastDebugServer {
    fn drop(&mut self) {
        self.stop();
    }
}