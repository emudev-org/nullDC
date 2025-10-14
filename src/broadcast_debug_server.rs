// this module must only ever be compiled for wasm32-unknown-unknown target

// Broadcast channel debugger server for WASM builds
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::BroadcastChannel;
use serde::{Deserialize, Serialize};
use crate::dreamcast::Dreamcast;

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

        Ok(Self {
            guid,
            announcement_channel,
            communication_channel,
            announcement_interval: None,
            dreamcast_ptr: dreamcast as usize,
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

        let announce_callback = Closure::<dyn Fn()>::new(move || {
            let announcement = Announcement {
                id: guid.clone(),
                name: format!("nullDC Instance {}", &guid[..8]),
                timestamp: js_sys::Date::now() as u64,
            };

            if let Ok(json) = serde_json::to_string(&announcement) {
                // Post the JSON string directly
                let _ = announcement_channel.post_message(&JsValue::from_str(&json));
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
        let _dreamcast_ptr = self.dreamcast_ptr;

        // Handle incoming messages
        let message_callback = Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
            let data = event.data();

            // Check if it's a ping message
            if let Some(msg) = data.as_string() {
                if msg == "ping" {
                    // Respond with pong
                    let _ = communication_channel.post_message(&JsValue::from_str("pong"));
                    return;
                }

                // Handle JSON-RPC messages
                // For now, we'll delegate to the mock server logic
                // In a full implementation, this would process RPC requests
                log::debug!("Received message: {}", msg);

                // TODO: Process JSON-RPC messages using the same handler as WebSocket
                // This would call into mock_debug_server::handle_request
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