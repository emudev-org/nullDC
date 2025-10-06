use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use sha2::{Sha256, Digest};
use uuid::Uuid;
use axum::extract::ws::{WebSocket, Message};
use futures::stream::StreamExt;
use futures::SinkExt;

const JSON_RPC_VERSION: &str = "2.0";

// JSON-RPC structures
#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: serde_json::Value,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcSuccess {
    jsonrpc: String,
    id: serde_json::Value,
    result: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    jsonrpc: String,
    id: serde_json::Value,
    error: JsonRpcErrorObject,
}

#[derive(Debug, Serialize)]
struct JsonRpcErrorObject {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcNotification {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
}

// Debugger schema structures
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegisterValue {
    name: String,
    value: String,
    width: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceNodeDescriptor {
    path: String,
    label: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    registers: Option<Vec<RegisterValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    events: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<DeviceNodeDescriptor>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BreakpointDescriptor {
    id: String,
    location: String,
    kind: String,
    enabled: bool,
    #[serde(rename = "hitCount")]
    hit_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThreadInfo {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    core: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backtrace: Option<Vec<BacktraceFrame>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BacktraceFrame {
    index: u32,
    pc: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DisassemblyLine {
    address: u64,
    bytes: String,
    mnemonic: String,
    operands: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "isCurrent")]
    is_current: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemorySlice {
    #[serde(rename = "baseAddress")]
    base_address: u64,
    #[serde(rename = "wordSize")]
    word_size: u32,
    encoding: String,
    data: String,
    validity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WaveformChunk {
    #[serde(rename = "channelId")]
    channel_id: String,
    #[serde(rename = "sampleRate")]
    sample_rate: u32,
    format: String,
    samples: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrameLogEntry {
    timestamp: u64,
    subsystem: String,
    severity: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CallstackFrame {
    index: u32,
    pc: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    sp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>,
}

// Client context
#[derive(Clone)]
struct ClientContext {
    session_id: String,
    topics: Arc<Mutex<HashSet<String>>>,
    watches: Arc<Mutex<HashSet<String>>>,
}

// Server state
struct ServerState {
    clients: Arc<Mutex<Vec<ClientContext>>>,
    breakpoints: Arc<Mutex<HashMap<String, BreakpointDescriptor>>>,
    watches: Arc<Mutex<HashSet<String>>>,
    register_values: Arc<Mutex<HashMap<String, String>>>,
    frame_log: Arc<Mutex<Vec<FrameLogEntry>>>,
}

impl ServerState {
    fn new() -> Self {
        let mut register_values = HashMap::new();

        // Initialize register values
        register_values.insert("dc.sh4.cpu.pc".to_string(), "0x8C0000A0".to_string());
        register_values.insert("dc.sh4.cpu.pr".to_string(), "0x8C0000A2".to_string());
        register_values.insert("dc.sh4.vbr".to_string(), "0x8C000000".to_string());
        register_values.insert("dc.sh4.sr".to_string(), "0x40000000".to_string());
        register_values.insert("dc.sh4.fpscr".to_string(), "0x00040001".to_string());
        register_values.insert("dc.sh4.icache.icache_ctrl".to_string(), "0x00000003".to_string());
        register_values.insert("dc.sh4.dcache.dcache_ctrl".to_string(), "0x00000003".to_string());
        register_values.insert("dc.holly.holly_id".to_string(), "0x00050000".to_string());
        register_values.insert("dc.holly.dmac_ctrl".to_string(), "0x00000001".to_string());
        register_values.insert("dc.holly.dmac.dmaor".to_string(), "0x8201".to_string());
        register_values.insert("dc.holly.dmac.chcr0".to_string(), "0x00000001".to_string());
        register_values.insert("dc.holly.ta.ta_list_base".to_string(), "0x0C000000".to_string());
        register_values.insert("dc.holly.ta.ta_status".to_string(), "0x00000000".to_string());
        register_values.insert("dc.holly.core.pvr_ctrl".to_string(), "0x00000001".to_string());
        register_values.insert("dc.holly.core.pvr_status".to_string(), "0x00010000".to_string());
        register_values.insert("dc.aica.aica_ctrl".to_string(), "0x00000002".to_string());
        register_values.insert("dc.aica.aica_status".to_string(), "0x00000001".to_string());
        register_values.insert("dc.aica.channels.ch0_vol".to_string(), "0x7F".to_string());
        register_values.insert("dc.aica.channels.ch1_vol".to_string(), "0x6A".to_string());
        register_values.insert("dc.aica.dsp.step".to_string(), "0x020".to_string());
        register_values.insert("dc.aica.dsp.dsp_acc".to_string(), "0x1F".to_string());
        register_values.insert("dc.sysclk".to_string(), "200MHz".to_string());
        register_values.insert("dc.asic_rev".to_string(), "0x0001".to_string());

        Self {
            clients: Arc::new(Mutex::new(Vec::new())),
            breakpoints: Arc::new(Mutex::new(HashMap::new())),
            watches: Arc::new(Mutex::new(HashSet::new())),
            register_values: Arc::new(Mutex::new(register_values)),
            frame_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_register_value(&self, path: &str, name: &str) -> String {
        let key = format!("{}.{}", path, name.to_lowercase());
        self.register_values
            .lock()
            .unwrap()
            .get(&key)
            .cloned()
            .unwrap_or_else(|| "0x00000000".to_string())
    }

    fn set_register_value(&self, path: &str, name: &str, value: String) {
        let key = format!("{}.{}", path, name.to_lowercase());
        self.register_values.lock().unwrap().insert(key, value);
    }

    fn build_device_tree(&self) -> Vec<DeviceNodeDescriptor> {
        vec![DeviceNodeDescriptor {
            path: "dc".to_string(),
            label: "Dreamcast".to_string(),
            kind: "bus".to_string(),
            description: Some("Sega Dreamcast system bus".to_string()),
            registers: Some(vec![
                RegisterValue {
                    name: "SYSCLK".to_string(),
                    value: self.get_register_value("dc", "SYSCLK"),
                    width: 0,
                },
                RegisterValue {
                    name: "ASIC_REV".to_string(),
                    value: self.get_register_value("dc", "ASIC_REV"),
                    width: 16,
                },
            ]),
            events: None,
            children: Some(vec![
                DeviceNodeDescriptor {
                    path: "dc.sh4".to_string(),
                    label: "SH4".to_string(),
                    kind: "processor".to_string(),
                    description: Some("Hitachi SH-4 main CPU".to_string()),
                    registers: Some(vec![
                        RegisterValue {
                            name: "VBR".to_string(),
                            value: self.get_register_value("dc.sh4", "VBR"),
                            width: 32,
                        },
                        RegisterValue {
                            name: "SR".to_string(),
                            value: self.get_register_value("dc.sh4", "SR"),
                            width: 32,
                        },
                        RegisterValue {
                            name: "FPSCR".to_string(),
                            value: self.get_register_value("dc.sh4", "FPSCR"),
                            width: 32,
                        },
                    ]),
                    events: Some(vec![
                        "dc.sh4.interrupt".to_string(),
                        "dc.sh4.exception".to_string(),
                        "dc.sh4.tlb_miss".to_string(),
                    ]),
                    children: Some(vec![
                        DeviceNodeDescriptor {
                            path: "dc.sh4.cpu".to_string(),
                            label: "Core".to_string(),
                            kind: "processor".to_string(),
                            description: Some("Integer pipeline".to_string()),
                            registers: Some(vec![
                                RegisterValue {
                                    name: "PC".to_string(),
                                    value: self.get_register_value("dc.sh4.cpu", "PC"),
                                    width: 32,
                                },
                                RegisterValue {
                                    name: "PR".to_string(),
                                    value: self.get_register_value("dc.sh4.cpu", "PR"),
                                    width: 32,
                                },
                            ]),
                            events: None,
                            children: None,
                        },
                        DeviceNodeDescriptor {
                            path: "dc.sh4.icache".to_string(),
                            label: "I-Cache".to_string(),
                            kind: "peripheral".to_string(),
                            description: Some("Instruction cache".to_string()),
                            registers: Some(vec![
                                RegisterValue {
                                    name: "ICRAM".to_string(),
                                    value: "16KB".to_string(),
                                    width: 0,
                                },
                                RegisterValue {
                                    name: "ICACHE_CTRL".to_string(),
                                    value: self.get_register_value("dc.sh4.icache", "ICACHE_CTRL"),
                                    width: 32,
                                },
                            ]),
                            events: None,
                            children: None,
                        },
                    ]),
                },
                DeviceNodeDescriptor {
                    path: "dc.aica".to_string(),
                    label: "AICA".to_string(),
                    kind: "coprocessor".to_string(),
                    description: Some("Sound processor".to_string()),
                    registers: Some(vec![
                        RegisterValue {
                            name: "AICA_CTRL".to_string(),
                            value: self.get_register_value("dc.aica", "AICA_CTRL"),
                            width: 32,
                        },
                    ]),
                    events: Some(vec!["dc.aica.interrupt".to_string()]),
                    children: None,
                },
            ]),
        }]
    }
}

fn sha256_byte(input: &str) -> u8 {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    result[0]
}

fn generate_disassembly(target: &str, address: u64, count: usize) -> Vec<DisassemblyLine> {
    type OperandFn = Box<dyn Fn(u8, u8, u8, u8, u16) -> String>;

    let sh4_instructions: Vec<(&str, OperandFn, u64)> = vec![
        ("mov.l", Box::new(|r1, r2, _, _, _| format!("@r{}+, r{}", r1, r2)), 2),
        ("mov", Box::new(|r1, r2, _, _, _| format!("r{}, r{}", r1, r2)), 2),
        ("add", Box::new(|r1, r2, _, _, _| format!("r{}, r{}", r1, r2)), 2),
        ("nop", Box::new(|_, _, _, _, _| "".to_string()), 2),
    ];

    let mut lines = Vec::new();
    let mut current_addr = address;

    for _ in 0..count {
        let hash = sha256_byte(&format!("{}:{:x}", target, current_addr));
        let instr_index = (hash as usize) % sh4_instructions.len();
        let (mnemonic, operand_fn, bytes_len) = &sh4_instructions[instr_index];

        let r1 = (hash >> 4) % 16;
        let r2 = (hash >> 2) % 16;
        let r3 = hash % 16;

        let operands = operand_fn(r1, r2, r3, 0, 0);

        let byte_values: Vec<String> = (0..*bytes_len)
            .map(|b| {
                format!(
                    "{:02X}",
                    sha256_byte(&format!("{}:{:x}:{}", target, current_addr, b))
                )
            })
            .collect();

        lines.push(DisassemblyLine {
            address: current_addr,
            bytes: byte_values.join(" "),
            mnemonic: mnemonic.to_string(),
            operands,
            comment: None,
            is_current: Some(false),
        });

        current_addr += *bytes_len as u64;
    }

    lines
}

fn build_memory_slice(
    target: &str,
    address: Option<u64>,
    length: Option<usize>,
    encoding: Option<String>,
    word_size: Option<u32>,
) -> MemorySlice {
    let default_base = match target {
        "sh4" => 0x8c000000u64,
        "arm7" => 0x00200000u64,
        "dsp" => 0x00000000u64,
        _ => 0x8c000000u64,
    };

    let base_address = address.unwrap_or(default_base);
    let effective_length = length.unwrap_or(64);
    let effective_word_size = word_size.unwrap_or(4);
    let effective_encoding = encoding.unwrap_or_else(|| "hex".to_string());

    let bytes: Vec<u8> = (0..effective_length)
        .map(|i| sha256_byte(&format!("{}:{:x}", target, base_address + i as u64)))
        .collect();

    MemorySlice {
        base_address,
        word_size: effective_word_size,
        encoding: effective_encoding,
        data: bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>(),
        validity: "ok".to_string(),
    }
}

fn build_waveform(channel_id: &str, window: usize) -> WaveformChunk {
    let samples: Vec<f32> = (0..window)
        .map(|i| ((i as f32 / window as f32) * std::f32::consts::PI * 4.0).sin())
        .collect();

    WaveformChunk {
        channel_id: channel_id.to_string(),
        sample_rate: 44100,
        format: "pcm_f32".to_string(),
        samples,
        label: Some(format!("Channel {}", channel_id)),
    }
}

fn handle_request(
    state: Arc<ServerState>,
    context: &ClientContext,
    request: JsonRpcRequest,
) -> Result<serde_json::Value, JsonRpcErrorObject> {
    let params = request.params.unwrap_or(json!({}));

    match request.method.as_str() {
        "debugger.handshake" => Ok(json!({
            "sessionId": context.session_id,
            "capabilities": ["watches", "step", "breakpoints", "frame-log", "waveforms"]
        })),

        "debugger.describe" => {
            let threads = vec![ThreadInfo {
                id: "thread-main".to_string(),
                name: Some("Main Thread".to_string()),
                state: "running".to_string(),
                core: Some("SH4".to_string()),
                priority: Some(0),
                backtrace: Some(vec![BacktraceFrame {
                    index: 0,
                    pc: 0x8c0000a0,
                    symbol: Some("_start".to_string()),
                    location: Some("crt0.S:42".to_string()),
                }]),
            }];

            Ok(json!({
                "emulator": {
                    "name": "nullDC",
                    "version": "dev",
                    "build": "native"
                },
                "devices": state.build_device_tree(),
                "breakpoints": state.breakpoints.lock().unwrap().values().cloned().collect::<Vec<_>>(),
                "threads": threads
            }))
        }

        "debugger.subscribe" => {
            let topics = params["topics"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            for topic in &topics {
                context.topics.lock().unwrap().insert(topic.clone());
            }

            Ok(json!({ "acknowledged": topics }))
        }

        "debugger.unsubscribe" => {
            let topics = params["topics"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            for topic in &topics {
                context.topics.lock().unwrap().remove(topic);
            }

            Ok(json!({ "acknowledged": topics }))
        }

        "state.getRegisters" => {
            let path = params["path"].as_str().unwrap_or("dc.sh4.cpu");
            let registers = vec![
                RegisterValue {
                    name: "PC".to_string(),
                    value: state.get_register_value(path, "PC"),
                    width: 32,
                },
                RegisterValue {
                    name: "R0".to_string(),
                    value: "0x00000000".to_string(),
                    width: 32,
                },
            ];

            Ok(json!({ "path": path, "registers": registers }))
        }

        "state.getMemorySlice" => {
            let target = params["target"].as_str().unwrap_or("sh4");
            let address = params["address"].as_u64();
            let length = params["length"].as_u64().map(|l| l as usize);
            let encoding = params["encoding"].as_str().map(|s| s.to_string());
            let word_size = params["wordSize"].as_u64().map(|w| w as u32);

            let slice = build_memory_slice(target, address, length, encoding, word_size);
            Ok(serde_json::to_value(slice).unwrap())
        }

        "state.getDisassembly" => {
            let target = params["target"].as_str().unwrap_or("sh4");
            let address = params["address"].as_u64().unwrap_or(0);
            let count = params["count"].as_u64().unwrap_or(128) as usize;

            let lines = generate_disassembly(target, address, count);
            Ok(json!({ "lines": lines }))
        }

        "state.getCallstack" => {
            let target = params["target"].as_str().unwrap_or("sh4");
            let max_frames = params["maxFrames"].as_u64().unwrap_or(16).min(64) as usize;

            let frames: Vec<CallstackFrame> = (0..max_frames)
                .map(|i| CallstackFrame {
                    index: i as u32,
                    pc: 0x8c000000 + (i * 4) as u64,
                    sp: Some(0x0cfe0000 - (i * 16) as u64),
                    symbol: Some(format!("{}_func_{}", target.to_uppercase(), i)),
                    location: Some(format!("{}.c:{}", target, 100 + i)),
                })
                .collect();

            Ok(json!({ "target": target, "frames": frames }))
        }

        "state.watch" => {
            let expressions = params["expressions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            for expr in &expressions {
                context.watches.lock().unwrap().insert(expr.clone());
                state.watches.lock().unwrap().insert(expr.clone());
            }

            let all_watches: Vec<String> = state.watches.lock().unwrap().iter().cloned().collect();
            Ok(json!({ "accepted": expressions, "all": all_watches }))
        }

        "state.unwatch" => {
            let expressions = params["expressions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            for expr in &expressions {
                context.watches.lock().unwrap().remove(expr);
                state.watches.lock().unwrap().remove(expr);
            }

            let all_watches: Vec<String> = state.watches.lock().unwrap().iter().cloned().collect();
            Ok(json!({ "accepted": expressions, "all": all_watches }))
        }

        "control.step" => {
            let target = params["target"].as_str().unwrap_or("sh4");
            Ok(json!({ "target": target, "state": "halted" }))
        }

        "control.runUntil" => {
            let target = params["target"].as_str().unwrap_or("sh4");
            Ok(json!({ "target": target, "state": "running", "reason": "breakpoint" }))
        }

        "breakpoints.add" => {
            let location = params["location"].as_str().unwrap_or("");
            let kind = params["kind"].as_str().unwrap_or("code");
            let enabled = params["enabled"].as_bool().unwrap_or(true);
            let id = format!("bp-{}", Uuid::new_v4().to_string()[..8].to_string());

            let breakpoint = BreakpointDescriptor {
                id: id.clone(),
                location: location.to_string(),
                kind: kind.to_string(),
                enabled,
                hit_count: 0,
            };

            state
                .breakpoints
                .lock()
                .unwrap()
                .insert(id, breakpoint.clone());

            let all: Vec<_> = state
                .breakpoints
                .lock()
                .unwrap()
                .values()
                .cloned()
                .collect();

            Ok(json!({ "breakpoint": breakpoint, "all": all }))
        }

        "breakpoints.remove" => {
            let id = params["id"].as_str().unwrap_or("");
            let removed = state.breakpoints.lock().unwrap().remove(id).is_some();
            let all: Vec<_> = state
                .breakpoints
                .lock()
                .unwrap()
                .values()
                .cloned()
                .collect();

            Ok(json!({ "removed": removed, "all": all }))
        }

        "breakpoints.toggle" => {
            let id = params["id"].as_str().unwrap_or("");
            let enabled = params["enabled"].as_bool().unwrap_or(true);

            let mut breakpoints = state.breakpoints.lock().unwrap();
            if let Some(bp) = breakpoints.get_mut(id) {
                bp.enabled = enabled;
                let updated = bp.clone();
                let all: Vec<_> = breakpoints.values().cloned().collect();
                Ok(json!({ "breakpoint": updated, "all": all }))
            } else {
                Err(JsonRpcErrorObject {
                    code: -32000,
                    message: format!("Breakpoint {} not found", id),
                    data: None,
                })
            }
        }

        "breakpoints.list" => {
            let all: Vec<_> = state
                .breakpoints
                .lock()
                .unwrap()
                .values()
                .cloned()
                .collect();
            Ok(json!({ "breakpoints": all }))
        }

        "audio.requestWaveform" => {
            let channel_id = params["channelId"].as_str().unwrap_or("0");
            let window = params["window"].as_u64().unwrap_or(256) as usize;
            let waveform = build_waveform(channel_id, window);
            Ok(serde_json::to_value(waveform).unwrap())
        }

        "logs.fetchFrameLog" => {
            let frame = params["frame"].as_u64().unwrap_or(0);
            let entries = state.frame_log.lock().unwrap().clone();
            Ok(json!({ "frame": frame, "entries": entries }))
        }

        _ => Err(JsonRpcErrorObject {
            code: -32601,
            message: format!("Method not found: {}", request.method),
            data: None,
        }),
    }
}

use std::sync::OnceLock;

static STATE: OnceLock<Arc<ServerState>> = OnceLock::new();

fn get_or_init_state() -> Arc<ServerState> {
    STATE.get_or_init(|| {
        let state = Arc::new(ServerState::new());

        // Start broadcast tick - simulate execution
        let tick_state = state.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(std::time::Duration::from_secs(1));

                // Update PC register
                let pc_key = "dc.sh4.cpu.pc";
                if let Some(pc_value) = tick_state.register_values.lock().unwrap().get(pc_key) {
                    if let Some(stripped) = pc_value.strip_prefix("0x") {
                        if let Ok(pc) = u64::from_str_radix(stripped, 16) {
                            let new_pc = format!("0x{:08X}", pc + 2);
                            tick_state.register_values.lock().unwrap().insert(pc_key.to_string(), new_pc);
                        }
                    }
                }
            }
        });

        state
    }).clone()
}

pub async fn handle_websocket_connection(socket: WebSocket) {
    let state = get_or_init_state();
    let (mut sender, mut receiver) = socket.split();

    let context = ClientContext {
        session_id: Uuid::new_v4().to_string(),
        topics: Arc::new(Mutex::new(HashSet::new())),
        watches: Arc::new(Mutex::new(HashSet::new())),
    };

    state.clients.lock().unwrap().push(context.clone());

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(request) = serde_json::from_str::<JsonRpcRequest>(&text) {
                    let id = request.id.clone();
                    match handle_request(state.clone(), &context, request) {
                        Ok(result) => {
                            let response = JsonRpcSuccess {
                                jsonrpc: JSON_RPC_VERSION.to_string(),
                                id,
                                result,
                            };
                            if let Ok(json) = serde_json::to_string(&response) {
                                let _ = sender.send(Message::Text(json.into())).await;
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

    // Remove client from list
    state.clients.lock().unwrap().retain(|c| c.session_id != context.session_id);
}
