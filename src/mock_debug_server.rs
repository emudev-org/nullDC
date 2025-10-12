use axum::extract::ws::{Message, WebSocket};
use futures::SinkExt;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<HashMap<String, bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<HashMap<String, serde_json::Value>>,
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
    id: u32,
    event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<u64>,
    kind: String, // "code" or "event"
    enabled: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DisassemblyLine {
    address: u64,
    bytes: String,
    disassembly: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemorySlice {
    #[serde(rename = "baseAddress")]
    base_address: u64,
    data: Vec<u8>,
    validity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventLogEntry {
    #[serde(rename = "eventId")]
    event_id: String,
    timestamp: u64,
    subsystem: String,
    severity: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WatchDescriptor {
    id: u32,
    expression: String,
    value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DebuggerTick {
    #[serde(rename = "tickId")]
    tick_id: u32,
    timestamp: u64,
    #[serde(rename = "executionState")]
    execution_state: ExecutionState,
    registers: HashMap<String, Vec<RegisterValue>>,
    breakpoints: HashMap<String, BreakpointDescriptor>,
    #[serde(rename = "eventLog")]
    event_log: Vec<EventLogEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    watches: Option<Vec<WatchDescriptor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    callstacks: Option<HashMap<String, Vec<CallstackFrame>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutionState {
    state: String, // "running" or "paused"
    #[serde(rename = "breakpointId", skip_serializing_if = "Option::is_none")]
    breakpoint_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BreakpointCategoryState {
    muted: bool,
    soloed: bool,
}

// Server watch structure
#[derive(Debug, Clone)]
struct ServerWatch {
    id: u32,
    expression: String,
}

// Server state
struct ServerState {
    breakpoints: Arc<Mutex<HashMap<u32, BreakpointDescriptor>>>,
    watches: Arc<Mutex<HashMap<u32, ServerWatch>>>,
    register_values: Arc<Mutex<HashMap<String, String>>>,
    event_log: Arc<Mutex<Vec<EventLogEntry>>>,
    category_states: Arc<Mutex<HashMap<String, BreakpointCategoryState>>>,
    is_running: Arc<Mutex<bool>>,
    tick_id: Arc<Mutex<u32>>,
    next_event_id: Arc<Mutex<u64>>,
    next_watch_id: Arc<Mutex<u32>>,
    next_breakpoint_id: Arc<Mutex<u32>>,
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
        register_values.insert(
            "dc.sh4.icache.icache_ctrl".to_string(),
            "0x00000003".to_string(),
        );
        register_values.insert(
            "dc.sh4.dcache.dcache_ctrl".to_string(),
            "0x00000003".to_string(),
        );
        register_values.insert("dc.holly.holly_id".to_string(), "0x00050000".to_string());
        register_values.insert("dc.holly.dmac_ctrl".to_string(), "0x00000001".to_string());
        register_values.insert("dc.holly.dmac.dmaor".to_string(), "0x8201".to_string());
        register_values.insert("dc.holly.dmac.chcr0".to_string(), "0x00000001".to_string());
        register_values.insert(
            "dc.holly.ta.ta_list_base".to_string(),
            "0x0C000000".to_string(),
        );
        register_values.insert(
            "dc.holly.ta.ta_status".to_string(),
            "0x00000000".to_string(),
        );
        register_values.insert(
            "dc.holly.core.pvr_ctrl".to_string(),
            "0x00000001".to_string(),
        );
        register_values.insert(
            "dc.holly.core.pvr_status".to_string(),
            "0x00010000".to_string(),
        );
        register_values.insert("dc.aica.aica_ctrl".to_string(), "0x00000002".to_string());
        register_values.insert("dc.aica.aica_status".to_string(), "0x00000001".to_string());
        register_values.insert("dc.aica.arm7.pc".to_string(), "0x00200010".to_string());
        register_values.insert("dc.aica.channels.ch0_vol".to_string(), "0x7F".to_string());
        register_values.insert("dc.aica.channels.ch1_vol".to_string(), "0x6A".to_string());
        register_values.insert("dc.aica.dsp.step".to_string(), "0x000".to_string());
        register_values.insert("dc.aica.dsp.dsp_acc".to_string(), "0x1F".to_string());
        register_values.insert("dc.sysclk".to_string(), "200MHz".to_string());
        register_values.insert("dc.asic_rev".to_string(), "0x0001".to_string());

        // Initialize default watches
        let mut watches = HashMap::new();
        let default_expressions = vec!["dc.sh4.cpu.pc", "dc.sh4.dmac.dmaor"];
        let mut next_watch_id = 1;
        for expr in default_expressions {
            watches.insert(
                next_watch_id,
                ServerWatch {
                    id: next_watch_id,
                    expression: expr.to_string(),
                },
            );
            next_watch_id += 1;
        }

        // Initialize category states
        let mut category_states = HashMap::new();
        for category in &["events", "sh4", "arm7", "dsp"] {
            category_states.insert(
                category.to_string(),
                BreakpointCategoryState {
                    muted: false,
                    soloed: false,
                },
            );
        }

        // Initialize event log with some entries
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let event_log = vec![EventLogEntry {
            event_id: "1".to_string(),
            timestamp,
            subsystem: "sh4".to_string(),
            severity: "info".to_string(),
            message: "SH4 initialized".to_string(),
            metadata: None,
        }];

        Self {
            breakpoints: Arc::new(Mutex::new(HashMap::new())),
            watches: Arc::new(Mutex::new(watches)),
            register_values: Arc::new(Mutex::new(register_values)),
            event_log: Arc::new(Mutex::new(event_log)),
            category_states: Arc::new(Mutex::new(category_states)),
            is_running: Arc::new(Mutex::new(true)),
            tick_id: Arc::new(Mutex::new(0)),
            next_event_id: Arc::new(Mutex::new(2)),
            next_watch_id: Arc::new(Mutex::new(next_watch_id)),
            next_breakpoint_id: Arc::new(Mutex::new(1)),
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

    fn evaluate_watch_expression(&self, dreamcast_ptr: usize, expression: &str) -> String {
        // Expression format: "dc.sh4.cpu.PC" or just "PC" (defaults to dc.sh4.cpu)
        // Split into path and register name
        let parts: Vec<&str> = expression.split('.').collect();

        if parts.is_empty() {
            return "0x00000000".to_string();
        }

        // If expression is a full path like "dc.sh4.cpu.R0"
        let (path, name) = if parts.len() > 1 {
            let name = parts.last().unwrap();
            let path = parts[..parts.len() - 1].join(".");
            (path, name.to_string())
        } else {
            // Default to dc.sh4.cpu if just register name
            ("dc.sh4.cpu".to_string(), parts[0].to_string())
        };

        // Try to get value from actual emulator if available
        if dreamcast_ptr != 0 && path == "dc.sh4.cpu" {
            let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
            if let Some(value) = nulldc::dreamcast::get_sh4_register(dreamcast, &name) {
                return format!("0x{:08X}", value);
            }
        }

        // Fall back to mock values
        self.get_register_value(&path, &name)
    }

    fn build_device_tree(&self) -> Vec<DeviceNodeDescriptor> {
        vec![DeviceNodeDescriptor {
            path: "dc".to_string(),
            label: "Dreamcast".to_string(),
            kind: "beloved console".to_string(),
            description: Some("Sega Dreamcast system bus".to_string()),
            registers: Some(vec![
                RegisterValue {
                    name: "SYSCLK".to_string(),
                    value: self.get_register_value("dc", "SYSCLK"),
                    width: 0,
                    flags: None,
                    metadata: None,
                },
                RegisterValue {
                    name: "ASIC_REV".to_string(),
                    value: self.get_register_value("dc", "ASIC_REV"),
                    width: 16,
                    flags: None,
                    metadata: None,
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
                            flags: None,
                            metadata: None,
                        },
                        RegisterValue {
                            name: "SR".to_string(),
                            value: self.get_register_value("dc.sh4", "SR"),
                            width: 32,
                            flags: None,
                            metadata: None,
                        },
                        RegisterValue {
                            name: "FPSCR".to_string(),
                            value: self.get_register_value("dc.sh4", "FPSCR"),
                            width: 32,
                            flags: None,
                            metadata: None,
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
                            registers: Some({
                                let mut regs = vec![
                                    RegisterValue {
                                        name: "PC".to_string(),
                                        value: self.get_register_value("dc.sh4.cpu", "PC"),
                                        width: 32,
                                        flags: None,
                                        metadata: None,
                                    },
                                    RegisterValue {
                                        name: "PR".to_string(),
                                        value: self.get_register_value("dc.sh4.cpu", "PR"),
                                        width: 32,
                                        flags: None,
                                        metadata: None,
                                    },
                                    RegisterValue {
                                        name: "SR".to_string(),
                                        value: self.get_register_value("dc.sh4.cpu", "SR"),
                                        width: 32,
                                        flags: None,
                                        metadata: None,
                                    },
                                    RegisterValue {
                                        name: "GBR".to_string(),
                                        value: self.get_register_value("dc.sh4.cpu", "GBR"),
                                        width: 32,
                                        flags: None,
                                        metadata: None,
                                    },
                                    RegisterValue {
                                        name: "VBR".to_string(),
                                        value: self.get_register_value("dc.sh4.cpu", "VBR"),
                                        width: 32,
                                        flags: None,
                                        metadata: None,
                                    },
                                    RegisterValue {
                                        name: "MACH".to_string(),
                                        value: self.get_register_value("dc.sh4.cpu", "MACH"),
                                        width: 32,
                                        flags: None,
                                        metadata: None,
                                    },
                                    RegisterValue {
                                        name: "MACL".to_string(),
                                        value: self.get_register_value("dc.sh4.cpu", "MACL"),
                                        width: 32,
                                        flags: None,
                                        metadata: None,
                                    },
                                    RegisterValue {
                                        name: "FPSCR".to_string(),
                                        value: self.get_register_value("dc.sh4.cpu", "FPSCR"),
                                        width: 32,
                                        flags: None,
                                        metadata: None,
                                    },
                                    RegisterValue {
                                        name: "FPUL".to_string(),
                                        value: self.get_register_value("dc.sh4.cpu", "FPUL"),
                                        width: 32,
                                        flags: None,
                                        metadata: None,
                                    },
                                ];
                                // Add R0-R15
                                for i in 0..16 {
                                    regs.push(RegisterValue {
                                        name: format!("R{}", i),
                                        value: self
                                            .get_register_value("dc.sh4.cpu", &format!("R{}", i)),
                                        width: 32,
                                        flags: None,
                                        metadata: None,
                                    });
                                }
                                regs
                            }),
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
                                    flags: None,
                                    metadata: None,
                                },
                                RegisterValue {
                                    name: "ICACHE_CTRL".to_string(),
                                    value: self.get_register_value("dc.sh4.icache", "ICACHE_CTRL"),
                                    width: 32,
                                    flags: None,
                                    metadata: None,
                                },
                            ]),
                            events: None,
                            children: None,
                        },
                        DeviceNodeDescriptor {
                            path: "dc.sh4.dcache".to_string(),
                            label: "D-Cache".to_string(),
                            kind: "peripheral".to_string(),
                            description: Some("Data cache".to_string()),
                            registers: Some(vec![
                                RegisterValue {
                                    name: "DCRAM".to_string(),
                                    value: "8KB".to_string(),
                                    width: 0,
                                    flags: None,
                                    metadata: None,
                                },
                                RegisterValue {
                                    name: "DCACHE_CTRL".to_string(),
                                    value: self.get_register_value("dc.sh4.dcache", "DCACHE_CTRL"),
                                    width: 32,
                                    flags: None,
                                    metadata: None,
                                },
                            ]),
                            events: None,
                            children: None,
                        },
                        DeviceNodeDescriptor {
                            path: "dc.sh4.tlb".to_string(),
                            label: "TLB".to_string(),
                            kind: "peripheral".to_string(),
                            description: Some("Translation lookaside buffer".to_string()),
                            registers: Some(vec![
                                RegisterValue {
                                    name: "UTLB_ENTRIES".to_string(),
                                    value: "64".to_string(),
                                    width: 0,
                                    flags: None,
                                    metadata: None,
                                },
                                RegisterValue {
                                    name: "ITLB_ENTRIES".to_string(),
                                    value: "4".to_string(),
                                    width: 0,
                                    flags: None,
                                    metadata: None,
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
                            flags: None,
                            metadata: None,
                        },
                        RegisterValue {
                            name: "AICA_STATUS".to_string(),
                            value: self.get_register_value("dc.aica", "AICA_STATUS"),
                            width: 32,
                            flags: None,
                            metadata: None,
                        },
                    ]),
                    events: Some(vec![
                        "dc.aica.interrupt".to_string(),
                        "dc.aica.timer".to_string(),
                    ]),
                    children: Some(vec![
                        DeviceNodeDescriptor {
                            path: "dc.aica.arm7".to_string(),
                            label: "ARM7".to_string(),
                            kind: "processor".to_string(),
                            description: Some("ARM7TDMI sound CPU".to_string()),
                            registers: Some(vec![RegisterValue {
                                name: "PC".to_string(),
                                value: self.get_register_value("dc.aica.arm7", "PC"),
                                width: 32,
                                flags: None,
                                metadata: None,
                            }]),
                            events: None,
                            children: None,
                        },
                        DeviceNodeDescriptor {
                            path: "dc.aica.dsp".to_string(),
                            label: "DSP".to_string(),
                            kind: "coprocessor".to_string(),
                            description: None,
                            registers: Some(vec![
                                RegisterValue {
                                    name: "STEP".to_string(),
                                    value: self.get_register_value("dc.aica.dsp", "STEP"),
                                    width: 16,
                                    flags: None,
                                    metadata: None,
                                },
                                RegisterValue {
                                    name: "DSP_ACC".to_string(),
                                    value: self.get_register_value("dc.aica.dsp", "DSP_ACC"),
                                    width: 16,
                                    flags: None,
                                    metadata: None,
                                },
                            ]),
                            events: Some(vec!["dc.aica.dsp.step".to_string()]),
                            children: None,
                        },
                    ]),
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
    type OperandFn = fn(u8, u8, u8, u8, u16) -> String;

    let sh4_instructions: Vec<(&str, OperandFn, u64)> = vec![
        ("mov.l", |r1, r2, _, _, _| format!("@r{}+, r{}", r1, r2), 2),
        ("mov", |r1, r2, _, _, _| format!("r{}, r{}", r1, r2), 2),
        ("sts.l", |r1, _, _, _, _| format!("pr, @-r{}", r1), 2),
        ("add", |r1, r2, _, _, _| format!("r{}, r{}", r1, r2), 2),
        ("cmp/eq", |r1, r2, _, _, _| format!("r{}, r{}", r1, r2), 2),
        ("bf", |_, _, _, _, offset| format!("0x{:x}", offset), 2),
        ("jmp", |r, _, _, _, _| format!("@r{}", r), 2),
        ("nop", |_, _, _, _, _| String::new(), 2),
    ];

    let arm7_instructions: Vec<(&str, OperandFn, u64)> = vec![
        ("mov", |r1, _, _, val, _| format!("r{}, #{}", r1, val), 4),
        (
            "ldr",
            |r1, r2, _, _, offset| format!("r{}, [r{}, #{}]", r1, r2, offset),
            4,
        ),
        ("str", |r1, r2, _, _, _| format!("r{}, [r{}]", r1, r2), 4),
        (
            "add",
            |r1, r2, r3, _, _| format!("r{}, r{}, r{}", r1, r2, r3),
            4,
        ),
        (
            "sub",
            |r1, r2, r3, _, _| format!("r{}, r{}, r{}", r1, r2, r3),
            4,
        ),
        ("bx", |r, _, _, _, _| format!("r{}", r), 4),
        ("bl", |_, _, _, _, offset| format!("0x{:x}", offset), 4),
        ("nop", |_, _, _, _, _| String::new(), 4),
    ];

    let selected = if target == "arm7" {
        &arm7_instructions
    } else {
        &sh4_instructions
    };

    let mut lines = Vec::new();
    let mut current_addr = address;

    for _ in 0..count {
        let hash = sha256_byte(&format!("{}:{:x}", target, current_addr));
        let instr_index = (hash as usize) % selected.len();
        let (mnemonic, operand_fn, bytes_len) = selected[instr_index];

        let r1 = (hash >> 4) % 16;
        let r2 = (hash >> 2) % 16;
        let r3 = hash % 16;
        let val = (hash.wrapping_mul(3)) & 0xff;
        let offset = (hash.wrapping_mul(7) as u16) & 0xfff;

        let operands = operand_fn(r1, r2, r3, val, offset);
        let disassembly = if operands.is_empty() {
            mnemonic.to_string()
        } else {
            format!("{} {}", mnemonic, operands)
        };

        let byte_values: Vec<String> = (0..bytes_len)
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
            disassembly,
        });

        current_addr += bytes_len;
    }

    lines
}

fn build_memory_slice(
    dreamcast_ptr: usize,
    target: &str,
    address: Option<u64>,
    length: Option<usize>,
) -> MemorySlice {
    let default_base = match target {
        "sh4" => 0x8c000000u64,
        "arm7" => 0x00200000u64,
        "dsp" => 0x00000000u64,
        _ => 0x8c000000u64,
    };

    let base_address = address.unwrap_or(default_base);
    let effective_length = length.unwrap_or(64);

    // Try to read from actual emulator memory if pointer is valid
    let bytes: Vec<u8> = if dreamcast_ptr != 0 {
        let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
        nulldc::dreamcast::read_memory_slice(dreamcast, base_address, effective_length)
    } else {
        // Fall back to mock data if no emulator context
        (0..effective_length)
            .map(|i| sha256_byte(&format!("{}:{:x}", target, base_address + i as u64)))
            .collect()
    };

    MemorySlice {
        base_address,
        data: bytes,
        validity: "ok".to_string(),
    }
}

fn collect_registers_from_tree(tree: &[DeviceNodeDescriptor]) -> Vec<(String, Vec<RegisterValue>)> {
    let mut result = Vec::new();
    for node in tree {
        if let Some(ref registers) = node.registers {
            if !registers.is_empty() {
                result.push((node.path.clone(), registers.clone()));
            }
        }
        if let Some(ref children) = node.children {
            result.extend(collect_registers_from_tree(children));
        }
    }
    result
}

fn handle_request(
    state: Arc<ServerState>,
    dreamcast_ptr: usize,
    request: JsonRpcRequest,
) -> Result<(serde_json::Value, bool), JsonRpcErrorObject> {
    let params = request.params.unwrap_or(json!({}));

    match request.method.as_str() {
        "debugger.describe" => {
            let device_tree = state.build_device_tree();
            Ok((
                json!({
                    "emulator": {
                        "name": "mockServer",
                        "version": "unspecified",
                        "build": "native"
                    },
                    "deviceTree": device_tree,
                }),
                true, // Send initial tick
            ))
        }

        "state.getMemorySlice" => {
            let target = params["target"].as_str().unwrap_or("sh4");
            let address = params["address"].as_u64();
            let length = params["length"].as_u64().map(|l| l as usize);

            let slice = build_memory_slice(dreamcast_ptr, target, address, length);
            Ok((serde_json::to_value(slice).unwrap(), false))
        }

        "state.getDisassembly" => {
            let target = params["target"].as_str().unwrap_or("sh4");
            let address = params["address"].as_u64().unwrap_or(0);
            let count = params["count"].as_u64().unwrap_or(128) as usize;

            let lines = if dreamcast_ptr != 0 && target == "sh4" {
                // Use actual disassembler for SH4
                let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
                let disasm_lines = nulldc::dreamcast::disassemble_sh4(dreamcast, address, count);

                // Convert to JSON-compatible format
                disasm_lines
                    .into_iter()
                    .map(|line| DisassemblyLine {
                        address: line.address,
                        bytes: line.bytes,
                        disassembly: line.disassembly,
                    })
                    .collect::<Vec<_>>()
            } else {
                // Fall back to mock data
                generate_disassembly(target, address, count)
            };

            Ok((json!({ "lines": lines }), false))
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

            Ok((json!({ "target": target, "frames": frames }), false))
        }

        "state.watch" => {
            let expressions = params["expressions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>();

            let mut watches = state.watches.lock().unwrap();
            let mut next_id = state.next_watch_id.lock().unwrap();

            for expr in expressions {
                let id = *next_id;
                watches.insert(
                    id,
                    ServerWatch {
                        id,
                        expression: expr,
                    },
                );
                *next_id += 1;
            }

            Ok((json!({}), true))
        }

        "state.unwatch" => {
            let watch_ids = params["watchIds"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_u64())
                .map(|id| id as u32)
                .collect::<Vec<_>>();

            let mut watches = state.watches.lock().unwrap();
            for id in watch_ids {
                watches.remove(&id);
            }

            Ok((json!({}), true))
        }

        "state.editWatch" => {
            let watch_id = params["watchId"].as_u64().map(|id| id as u32);
            let value = params["value"].as_str().unwrap_or("");

            if let Some(id) = watch_id {
                let expression = {
                    let watches = state.watches.lock().unwrap();
                    watches.get(&id).map(|w| w.expression.clone())
                };

                if let Some(expr) = expression {
                    // Parse expression to get path and register name
                    let parts: Vec<&str> = expr.split('.').collect();
                    let (path, name) = if parts.len() > 1 {
                        let name = parts.last().unwrap();
                        let path = parts[..parts.len() - 1].join(".");
                        (path, name.to_string())
                    } else {
                        ("dc.sh4.cpu".to_string(), parts[0].to_string())
                    };

                    state.set_register_value(&path, &name, value.to_string());
                    return Ok((json!({}), true));
                }
            }

            Err(JsonRpcErrorObject {
                code: -32602,
                message: "Watch not found or cannot edit".to_string(),
                data: None,
            })
        }

        "state.modifyWatchExpression" => {
            let watch_id = params["watchId"].as_u64().map(|id| id as u32);
            let new_expression = params["newExpression"].as_str().unwrap_or("");

            if let Some(id) = watch_id {
                let mut watches = state.watches.lock().unwrap();
                if let Some(watch) = watches.get_mut(&id) {
                    watch.expression = new_expression.to_string();
                    return Ok((json!({}), true));
                }
            }

            Err(JsonRpcErrorObject {
                code: -32602,
                message: "Watch not found".to_string(),
                data: None,
            })
        }

        "control.pause" => {
            if dreamcast_ptr != 0 {
                let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
                nulldc::dreamcast::set_dreamcast_running(dreamcast, false);
            } else {
                *state.is_running.lock().unwrap() = false;
            }
            Ok((json!({}), true))
        }

        "control.step" | "control.stepOver" | "control.stepOut" => {
            if dreamcast_ptr != 0 {
                let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
                // Use step_dreamcast for single instruction stepping
                nulldc::dreamcast::step_dreamcast(dreamcast);
                // Ensure we're paused after step
                nulldc::dreamcast::set_dreamcast_running(dreamcast, false);
            } else {
                // Mock implementation
                *state.is_running.lock().unwrap() = false;
                let target = params["target"].as_str().unwrap_or("sh4");

                if target.contains("sh4") {
                    let pc_value = state.get_register_value("dc.sh4.cpu", "PC");
                    if let Some(stripped) = pc_value.strip_prefix("0x") {
                        if let Ok(pc) = u64::from_str_radix(stripped, 16) {
                            let base = 0x8C0000A0;
                            let offset = pc - base;
                            let new_offset = (offset + 2) % 16;
                            let new_pc = base + new_offset;
                            state.set_register_value(
                                "dc.sh4.cpu",
                                "PC",
                                format!("0x{:08X}", new_pc),
                            );
                        }
                    }
                }
            }

            Ok((json!({}), true))
        }

        "control.runUntil" => {
            if dreamcast_ptr != 0 {
                let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
                nulldc::dreamcast::set_dreamcast_running(dreamcast, true);
            } else {
                *state.is_running.lock().unwrap() = true;
            }
            Ok((json!({}), true))
        }

        "breakpoints.add" => {
            let event = params["event"].as_str().unwrap_or("");
            let address = params["address"].as_u64();
            let kind = params["kind"].as_str().unwrap_or("code");
            let enabled = params["enabled"].as_bool().unwrap_or(true);

            let mut next_id = state.next_breakpoint_id.lock().unwrap();
            let id = *next_id;
            *next_id += 1;

            let breakpoint = BreakpointDescriptor {
                id,
                event: event.to_string(),
                address,
                kind: kind.to_string(),
                enabled,
            };

            state.breakpoints.lock().unwrap().insert(id, breakpoint);
            Ok((json!({}), true))
        }

        "breakpoints.remove" => {
            let id = params["id"].as_u64().map(|id| id as u32);
            if let Some(id) = id {
                let removed = state.breakpoints.lock().unwrap().remove(&id).is_some();
                if !removed {
                    return Err(JsonRpcErrorObject {
                        code: -32000,
                        message: format!("Breakpoint {} not found", id),
                        data: None,
                    });
                }
            }
            Ok((json!({}), true))
        }

        "breakpoints.toggle" => {
            let id = params["id"].as_u64().map(|id| id as u32);
            let enabled = params["enabled"].as_bool().unwrap_or(true);

            if let Some(id) = id {
                let mut breakpoints = state.breakpoints.lock().unwrap();
                if let Some(bp) = breakpoints.get_mut(&id) {
                    bp.enabled = enabled;
                    return Ok((json!({}), true));
                }
            }

            Err(JsonRpcErrorObject {
                code: -32000,
                message: "Breakpoint not found".to_string(),
                data: None,
            })
        }

        "breakpoints.setCategoryStates" => {
            let categories = params["categories"].as_object();
            if let Some(categories) = categories {
                let mut category_states = state.category_states.lock().unwrap();
                for (category, state_value) in categories {
                    if let (Some(muted), Some(soloed)) = (
                        state_value["muted"].as_bool(),
                        state_value["soloed"].as_bool(),
                    ) {
                        category_states
                            .insert(category.clone(), BreakpointCategoryState { muted, soloed });
                    }
                }
            }
            Ok((json!({}), true))
        }

        _ => Err(JsonRpcErrorObject {
            code: -32601,
            message: format!("Method not found: {}", request.method),
            data: None,
        }),
    }
}

pub async fn handle_websocket_connection(socket: WebSocket, dreamcast_ptr: usize) {
    use std::sync::OnceLock;
    static STATE: OnceLock<Arc<ServerState>> = OnceLock::new();
    let state = STATE.get_or_init(|| Arc::new(ServerState::new())).clone();

    // Convert usize back to *mut Dreamcast when needed to access emulator state
    // let _dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
    // TODO: Use dreamcast pointer to read/write actual emulator state instead of mock data

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
                                // Build and send tick
                                let device_tree = state.build_device_tree();
                                let all_registers = collect_registers_from_tree(&device_tree);

                                let mut registers_by_id: HashMap<String, Vec<RegisterValue>> =
                                    HashMap::new();
                                for (path, registers) in all_registers {
                                    registers_by_id.insert(path, registers);
                                }

                                // Override with actual register values from emulator if available
                                if dreamcast_ptr != 0 {
                                    let dreamcast =
                                        dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
                                    let sh4_registers = vec![
                                        ("PC", 32),
                                        ("PR", 32),
                                        ("SR", 32),
                                        ("GBR", 32),
                                        ("VBR", 32),
                                        ("MACH", 32),
                                        ("MACL", 32),
                                        ("FPSCR", 32),
                                        ("FPUL", 32),
                                    ];

                                    let mut cpu_regs = Vec::new();
                                    for (name, width) in &sh4_registers {
                                        if let Some(value) =
                                            nulldc::dreamcast::get_sh4_register(dreamcast, name)
                                        {
                                            cpu_regs.push(RegisterValue {
                                                name: name.to_string(),
                                                value: format!("0x{:08X}", value),
                                                width: *width,
                                                flags: None,
                                                metadata: None,
                                            });
                                        }
                                    }

                                    // Add general purpose registers R0-R15
                                    for i in 0..16 {
                                        let reg_name = format!("R{}", i);
                                        if let Some(value) = nulldc::dreamcast::get_sh4_register(
                                            dreamcast, &reg_name,
                                        ) {
                                            cpu_regs.push(RegisterValue {
                                                name: reg_name,
                                                value: format!("0x{:08X}", value),
                                                width: 32,
                                                flags: None,
                                                metadata: None,
                                            });
                                        }
                                    }

                                    registers_by_id.insert("dc.sh4.cpu".to_string(), cpu_regs);
                                }

                                let mut breakpoints_by_id: HashMap<String, BreakpointDescriptor> =
                                    HashMap::new();
                                for (id, bp) in state.breakpoints.lock().unwrap().iter() {
                                    breakpoints_by_id.insert(id.to_string(), bp.clone());
                                }

                                let watches: Vec<WatchDescriptor> = state
                                    .watches
                                    .lock()
                                    .unwrap()
                                    .values()
                                    .map(|w| WatchDescriptor {
                                        id: w.id,
                                        expression: w.expression.clone(),
                                        value: json!(state.evaluate_watch_expression(
                                            dreamcast_ptr,
                                            &w.expression
                                        )),
                                    })
                                    .collect();

                                let mut callstacks: HashMap<String, Vec<CallstackFrame>> =
                                    HashMap::new();

                                // SH4 callstack
                                let sh4_pc_value = state.get_register_value("dc.sh4.cpu", "PC");
                                let sh4_pc = if let Some(stripped) = sh4_pc_value.strip_prefix("0x")
                                {
                                    u64::from_str_radix(stripped, 16).unwrap_or(0x8c0000a0)
                                } else {
                                    0x8c0000a0
                                };
                                let sh4_frames: Vec<CallstackFrame> = (0..16)
                                    .map(|i| CallstackFrame {
                                        index: i,
                                        pc: if i == 0 {
                                            sh4_pc
                                        } else {
                                            0x8c000000 + (i - 1) as u64 * 4
                                        },
                                        sp: Some(0x0cfe0000 - i as u64 * 16),
                                        symbol: Some(format!("SH4_func_{}", i)),
                                        location: Some(format!("sh4.c:{}", 100 + i)),
                                    })
                                    .collect();
                                callstacks.insert("sh4".to_string(), sh4_frames);

                                // ARM7 callstack
                                let arm7_pc_value = state.get_register_value("dc.aica.arm7", "PC");
                                let arm7_pc =
                                    if let Some(stripped) = arm7_pc_value.strip_prefix("0x") {
                                        u64::from_str_radix(stripped, 16).unwrap_or(0x00200010)
                                    } else {
                                        0x00200010
                                    };
                                let arm7_frames: Vec<CallstackFrame> = (0..16)
                                    .map(|i| CallstackFrame {
                                        index: i,
                                        pc: if i == 0 {
                                            arm7_pc
                                        } else {
                                            0x00200000 + (i - 1) as u64 * 4
                                        },
                                        sp: Some(0x00280000 - i as u64 * 16),
                                        symbol: Some(format!("ARM7_func_{}", i)),
                                        location: Some(format!("arm7.c:{}", 100 + i)),
                                    })
                                    .collect();
                                callstacks.insert("arm7".to_string(), arm7_frames);

                                let current_tick_id = {
                                    let mut tick_id = state.tick_id.lock().unwrap();
                                    let id = *tick_id;
                                    *tick_id += 1;
                                    id
                                };
                                let timestamp = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis()
                                    as u64;
                                // Get execution state from actual emulator or mock
                                let is_running = if dreamcast_ptr != 0 {
                                    let dreamcast =
                                        dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
                                    nulldc::dreamcast::is_dreamcast_running(dreamcast)
                                } else {
                                    *state.is_running.lock().unwrap()
                                };

                                let tick = DebuggerTick {
                                    tick_id: current_tick_id,
                                    timestamp,
                                    execution_state: ExecutionState {
                                        state: if is_running {
                                            "running".to_string()
                                        } else {
                                            "paused".to_string()
                                        },
                                        breakpoint_id: None,
                                    },
                                    registers: registers_by_id,
                                    breakpoints: breakpoints_by_id,
                                    event_log: state.event_log.lock().unwrap().clone(),
                                    watches: if watches.is_empty() {
                                        None
                                    } else {
                                        Some(watches)
                                    },
                                    callstacks: Some(callstacks),
                                };

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
