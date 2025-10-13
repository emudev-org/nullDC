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

#[allow(dead_code)]
mod panel_ids {
    pub const DOCUMENTATION: &str = "documentation";
    pub const SH4_SIM: &str = "sh4-sim";
    pub const EVENTS: &str = "events";
    pub const EVENTS_BREAKPOINTS: &str = "events-breakpoints";
    pub const SH4_DISASSEMBLY: &str = "sh4-disassembly";
    pub const SH4_MEMORY: &str = "sh4-memory";
    pub const SH4_BREAKPOINTS: &str = "sh4-breakpoints";
    pub const SH4_BSC_REGISTERS: &str = "bsc-registers";
    pub const SH4_CCN_REGISTERS: &str = "ccn-registers";
    pub const SH4_CPG_REGISTERS: &str = "cpg-registers";
    pub const SH4_DMAC_REGISTERS: &str = "dmac-registers";
    pub const SH4_INTC_REGISTERS: &str = "intc-registers";
    pub const SH4_RTC_REGISTERS: &str = "rtc-registers";
    pub const SH4_SCI_REGISTERS: &str = "sci-registers";
    pub const SH4_SCIF_REGISTERS: &str = "scif-registers";
    pub const SH4_TMU_REGISTERS: &str = "tmu-registers";
    pub const SH4_UBC_REGISTERS: &str = "ubc-registers";
    pub const SH4_SQ_CONTENTS: &str = "sq-contents";
    pub const SH4_ICACHE_CONTENTS: &str = "icache-contents";
    pub const SH4_OCACHE_CONTENTS: &str = "ocache-contents";
    pub const SH4_OCRAM_CONTENTS: &str = "ocram-contents";
    pub const SH4_TLB_CONTENTS: &str = "tlb-contents";
    pub const ARM7_DISASSEMBLY: &str = "arm7-disassembly";
    pub const ARM7_MEMORY: &str = "arm7-memory";
    pub const ARM7_BREAKPOINTS: &str = "arm7-breakpoints";
    pub const CLX2_TA: &str = "holly-ta";
    pub const CLX2_CORE: &str = "holly-core";
    pub const SGC: &str = "sgc";
    pub const DSP_DISASSEMBLY: &str = "dsp-disassembly";
    pub const DSP_BREAKPOINTS: &str = "dsp-breakpoints";
    pub const DSP_PLAYGROUND: &str = "dsp-playground";
    pub const DEVICE_TREE: &str = "device-tree";
    pub const WATCHES: &str = "watches";
    pub const SH4_CALLSTACK: &str = "sh4-callstack";
    pub const ARM7_CALLSTACK: &str = "arm7-callstack";
}

const DEFAULT_WATCH_EXPRESSIONS: &[&str] = &["dc.sh4.cpu.pc", "dc.sh4.dmac.dmaor"];
const EVENT_LOG_LIMIT: usize = 60;
const CAPABILITIES: &[&str] = &["watches", "step", "breakpoints", "frame-log"];

type FrameEventGenerator = fn(u64) -> (&'static str, &'static str, String);

fn frame_event_ta(counter: u64) -> (&'static str, &'static str, String) {
    (
        "ta",
        "info",
        format!("TA/END_LIST tile {}", (counter % 32) as usize),
    )
}

fn frame_event_core(counter: u64) -> (&'static str, &'static str, String) {
    let phase = match counter % 3 {
        0 => "START_RENDER",
        1 => "QUEUE_SUBMISSION",
        _ => "END_RENDER",
    };
    (
        "core",
        if phase == "QUEUE_SUBMISSION" {
            "trace"
        } else {
            "info"
        },
        format!("CORE/{}", phase),
    )
}

fn frame_event_dsp(counter: u64) -> (&'static str, &'static str, String) {
    (
        "dsp",
        "trace",
        format!("DSP/STEP pipeline advanced ({})", counter % 8),
    )
}

fn frame_event_aica(counter: u64) -> (&'static str, &'static str, String) {
    (
        "aica",
        "info",
        format!("AICA/SGC/STEP channel {}", (counter % 2) as usize),
    )
}

fn frame_event_sh4(counter: u64) -> (&'static str, &'static str, String) {
    (
        "sh4",
        "warn",
        format!("SH4/INTERRUPT IRQ{} asserted", (counter % 6) + 1),
    )
}

fn frame_event_holly(counter: u64) -> (&'static str, &'static str, String) {
    (
        "holly",
        "info",
        format!("HOLLY/START_RENDER pass {}", (counter % 4) + 1),
    )
}

const FRAME_EVENT_GENERATORS: &[FrameEventGenerator] = &[
    frame_event_ta,
    frame_event_core,
    frame_event_dsp,
    frame_event_aica,
    frame_event_sh4,
    frame_event_holly,
];

fn create_frame_event_with_id(event_id: u64) -> EventLogEntry {
    let generator = FRAME_EVENT_GENERATORS[(event_id as usize) % FRAME_EVENT_GENERATORS.len()];
    let (subsystem, severity, message) = generator(event_id);
    EventLogEntry {
        event_id: event_id.to_string(),
        timestamp: current_timestamp_ms(),
        subsystem: subsystem.to_string(),
        severity: severity.to_string(),
        message,
        metadata: None,
    }
}

fn initial_event_log() -> (Vec<EventLogEntry>, u64) {
    let mut log = Vec::new();
    for id in 1..=6 {
        log.push(create_frame_event_with_id(id));
    }
    (log, 7)
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_millis(0))
        .as_millis() as u64
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    registers: Option<Vec<RegisterValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    events: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actions: Option<Vec<String>>,
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
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DebuggerTick {
    #[serde(rename = "tickId")]
    tick_id: u64,
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
    tick_id: Arc<Mutex<u64>>,
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
        register_values.insert("dc.sh4.cpu.gbr".to_string(), "0x8C000100".to_string());
        register_values.insert("dc.sh4.cpu.mach".to_string(), "0x00000000".to_string());
        register_values.insert("dc.sh4.cpu.macl".to_string(), "0x00000000".to_string());
        register_values.insert("dc.sh4.cpu.fpul".to_string(), "0x00000000".to_string());
        register_values.insert(
            "dc.sh4.icache.icache_ctrl".to_string(),
            "0x00000003".to_string(),
        );
        register_values.insert(
            "dc.sh4.dcache.dcache_ctrl".to_string(),
            "0x00000003".to_string(),
        );
        register_values.insert("dc.sh4.dmac.dmaor".to_string(), "0x8201".to_string());
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
        let mut next_watch_id = 1;
        for expr in DEFAULT_WATCH_EXPRESSIONS {
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

        // Initialize event log with sample entries
        let (event_log, next_event_id) = initial_event_log();

        Self {
            breakpoints: Arc::new(Mutex::new(HashMap::new())),
            watches: Arc::new(Mutex::new(watches)),
            register_values: Arc::new(Mutex::new(register_values)),
            event_log: Arc::new(Mutex::new(event_log)),
            category_states: Arc::new(Mutex::new(category_states)),
            is_running: Arc::new(Mutex::new(true)),
            tick_id: Arc::new(Mutex::new(0)),
            next_event_id: Arc::new(Mutex::new(next_event_id)),
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
        if dreamcast_ptr != 0 {
            let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
            if path == "dc.sh4.cpu" {
                if let Some(value) = nulldc::dreamcast::get_sh4_register(dreamcast, &name) {
                    return format!("0x{:08X}", value);
                }
            } else if path == "dc.aica.arm7" {
                if let Some(value) = nulldc::dreamcast::get_arm_register(dreamcast, &name) {
                    return format!("0x{:08X}", value);
                }
            }
        }

        // Fall back to mock values
        self.get_register_value(&path, &name)
    }

    fn build_device_tree(&self) -> Vec<DeviceNodeDescriptor> {
        let register = |path: &str, name: &str, width: u32| RegisterValue {
            name: name.to_string(),
            value: self.get_register_value(path, name),
            width,
            flags: None,
            metadata: None,
        };

        let mut sh4_core_registers = vec![
            register("dc.sh4.cpu", "PC", 32),
            register("dc.sh4.cpu", "PR", 32),
            register("dc.sh4", "VBR", 32),
            register("dc.sh4", "SR", 32),
            register("dc.sh4", "FPSCR", 32),
            register("dc.sh4.cpu", "GBR", 32),
            register("dc.sh4.cpu", "MACH", 32),
            register("dc.sh4.cpu", "MACL", 32),
            register("dc.sh4.cpu", "FPUL", 32),
        ];
        for idx in 0..16 {
            sh4_core_registers.push(register("dc.sh4.cpu", &format!("R{}", idx), 32));
        }

        let sh4_pbus_children = vec![
            DeviceNodeDescriptor {
                path: "dc.sh4.bsc".to_string(),
                label: "BSC".to_string(),
                description: Some("Bus State Controller".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_BSC_REGISTERS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.ccn".to_string(),
                label: "CCN".to_string(),
                description: Some("Cache Controller".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_CCN_REGISTERS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.cpg".to_string(),
                label: "CPG".to_string(),
                description: Some("Clock Pulse Generator".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_CPG_REGISTERS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.dmac".to_string(),
                label: "DMAC".to_string(),
                description: Some("Direct Memory Access Controller".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_DMAC_REGISTERS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.intc".to_string(),
                label: "INTC".to_string(),
                description: Some("Interrupt Controller".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_INTC_REGISTERS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.rtc".to_string(),
                label: "RTC".to_string(),
                description: Some("Real Time Clock".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_RTC_REGISTERS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.sci".to_string(),
                label: "SCI".to_string(),
                description: Some("Serial Communications Interface".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_SCI_REGISTERS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.scif".to_string(),
                label: "SCIF".to_string(),
                description: Some("Serial Communications Interface w/ FIFO".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_SCIF_REGISTERS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.tmu".to_string(),
                label: "TMU".to_string(),
                description: Some("Timer Unit".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_TMU_REGISTERS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.ubc".to_string(),
                label: "UBC".to_string(),
                description: Some("User Break Controller".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_UBC_REGISTERS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.sq".to_string(),
                label: "SQ".to_string(),
                description: Some("Store Queues".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_SQ_CONTENTS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.icache".to_string(),
                label: "ICACHE".to_string(),
                description: Some("Instruction Cache".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_ICACHE_CONTENTS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.ocache".to_string(),
                label: "OCACHE".to_string(),
                description: Some("Operand Cache".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_OCACHE_CONTENTS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.ocram".to_string(),
                label: "OCRAM".to_string(),
                description: Some("Operand RAM".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_OCRAM_CONTENTS.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.tlb".to_string(),
                label: "TLB".to_string(),
                description: Some("Translation Lookaside Buffer".to_string()),
                registers: None,
                events: None,
                actions: Some(vec![panel_ids::SH4_TLB_CONTENTS.to_string()]),
                children: None,
            },
        ];

        let sh4_children = vec![
            DeviceNodeDescriptor {
                path: "dc.sh4.cpu".to_string(),
                label: "Core".to_string(),
                description: Some("SuperH4 CPU Core".to_string()),
                registers: Some(sh4_core_registers),
                events: None,
                actions: Some(vec![
                    panel_ids::SH4_DISASSEMBLY.to_string(),
                    panel_ids::SH4_MEMORY.to_string(),
                ]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.sh4.pbus".to_string(),
                label: "PBUS".to_string(),
                description: Some("Peripherals Bus".to_string()),
                registers: None,
                events: None,
                actions: None,
                children: Some(sh4_pbus_children),
            },
        ];

        let holly_children = vec![
            DeviceNodeDescriptor {
                path: "dc.holly.dmac".to_string(),
                label: "DMA Controller".to_string(),
                description: Some("peripheral".to_string()),
                registers: Some(vec![
                    register("dc.holly.dmac", "DMAOR", 16),
                    register("dc.holly.dmac", "CHCR0", 32),
                ]),
                events: Some(vec![
                    "dc.holly.dmac.transfer_start".to_string(),
                    "dc.holly.dmac.transfer_end".to_string(),
                ]),
                actions: None,
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.holly.ta".to_string(),
                label: "TA".to_string(),
                description: Some("Tile Accelerator".to_string()),
                registers: Some(vec![
                    register("dc.holly.ta", "TA_LIST_BASE", 32),
                    register("dc.holly.ta", "TA_STATUS", 32),
                ]),
                events: Some(vec![
                    "dc.holly.ta.list_init".to_string(),
                    "dc.holly.ta.list_end".to_string(),
                    "dc.holly.ta.opaque_complete".to_string(),
                    "dc.holly.ta.translucent_complete".to_string(),
                ]),
                actions: Some(vec![panel_ids::CLX2_TA.to_string()]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.holly.core".to_string(),
                label: "CORE".to_string(),
                description: Some("Depth and Shading Engine".to_string()),
                registers: Some(vec![
                    register("dc.holly.core", "PVR_CTRL", 32),
                    register("dc.holly.core", "PVR_STATUS", 32),
                ]),
                events: Some(vec![
                    "dc.holly.core.render_start".to_string(),
                    "dc.holly.core.render_end".to_string(),
                    "dc.holly.core.vblank".to_string(),
                ]),
                actions: Some(vec![panel_ids::CLX2_CORE.to_string()]),
                children: None,
            },
        ];

        let sgc_channels = vec![
            DeviceNodeDescriptor {
                path: "dc.aica.sgc.0".to_string(),
                label: "Channel 0".to_string(),
                description: Some("SGC Channel 0".to_string()),
                registers: Some(vec![register("dc.aica.channels", "CH0_VOL", 8)]),
                events: Some(vec![
                    "dc.aica.channel.0.key_on".to_string(),
                    "dc.aica.channel.0.key_off".to_string(),
                    "dc.aica.channel.0.loop".to_string(),
                ]),
                actions: None,
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.aica.sgc.1".to_string(),
                label: "Channel 1".to_string(),
                description: Some("SGC Channel 1".to_string()),
                registers: Some(vec![register("dc.aica.channels", "CH1_VOL", 8)]),
                events: Some(vec![
                    "dc.aica.channel.0.key_on".to_string(),
                    "dc.aica.channel.0.key_off".to_string(),
                    "dc.aica.channel.0.loop".to_string(),
                ]),
                actions: None,
                children: None,
            },
        ];

        let aica_children = vec![
            DeviceNodeDescriptor {
                path: "dc.aica.arm7".to_string(),
                label: "ARM7".to_string(),
                description: Some("ARM7DI CPU Core".to_string()),
                registers: Some(vec![register("dc.aica.arm7", "PC", 32)]),
                events: None,
                actions: Some(vec![
                    panel_ids::ARM7_DISASSEMBLY.to_string(),
                    panel_ids::ARM7_MEMORY.to_string(),
                ]),
                children: None,
            },
            DeviceNodeDescriptor {
                path: "dc.aica.sgc".to_string(),
                label: "SGC".to_string(),
                description: Some("Sound Generation Core".to_string()),
                registers: None,
                events: Some(vec![
                    "dc.aica.channels.key_on".to_string(),
                    "dc.aica.channels.key_off".to_string(),
                    "dc.aica.channels.loop".to_string(),
                ]),
                actions: Some(vec![panel_ids::SGC.to_string()]),
                children: Some(sgc_channels),
            },
            DeviceNodeDescriptor {
                path: "dc.aica.dsp".to_string(),
                label: "DSP".to_string(),
                description: Some("DSP VLIW Core".to_string()),
                registers: Some(vec![
                    register("dc.aica.dsp", "STEP", 16),
                    register("dc.aica.dsp", "DSP_ACC", 16),
                ]),
                events: Some(vec![
                    "dc.aica.dsp.step".to_string(),
                    "dc.aica.dsp.sample_start".to_string(),
                ]),
                actions: Some(vec![panel_ids::DSP_DISASSEMBLY.to_string()]),
                children: None,
            },
        ];

        vec![DeviceNodeDescriptor {
            path: "dc".to_string(),
            label: "Dreamcast".to_string(),
            description: Some("beloved console".to_string()),
            registers: Some(vec![
                register("dc", "SYSCLK", 0),
                register("dc", "ASIC_REV", 16),
            ]),
            events: None,
            actions: None,
            children: Some(vec![
                DeviceNodeDescriptor {
                    path: "dc.sh4".to_string(),
                    label: "SH4".to_string(),
                    description: Some("SH7750-alike SoC".to_string()),
                    registers: Some(vec![
                        register("dc.sh4", "VBR", 32),
                        register("dc.sh4", "SR", 32),
                        register("dc.sh4", "FPSCR", 32),
                    ]),
                    events: Some(vec![
                        "dc.sh4.interrupt".to_string(),
                        "dc.sh4.exception".to_string(),
                        "dc.sh4.tlb_miss".to_string(),
                    ]),
                    actions: None,
                    children: Some(sh4_children),
                },
                DeviceNodeDescriptor {
                    path: "dc.holly".to_string(),
                    label: "Holly".to_string(),
                    description: Some("System ASIC".to_string()),
                    registers: Some(vec![
                        register("dc.holly", "HOLLY_ID", 32),
                        register("dc.holly", "DMAC_CTRL", 32),
                    ]),
                    events: None,
                    actions: None,
                    children: Some(holly_children),
                },
                DeviceNodeDescriptor {
                    path: "dc.aica".to_string(),
                    label: "AICA".to_string(),
                    description: Some("Sound SoC".to_string()),
                    registers: Some(vec![
                        register("dc.aica", "AICA_CTRL", 32),
                        register("dc.aica", "AICA_STATUS", 32),
                    ]),
                    events: Some(vec![
                        "dc.aica.interrupt".to_string(),
                        "dc.aica.timer".to_string(),
                    ]),
                    actions: None,
                    children: Some(aica_children),
                },
            ]),
        }]
    }

    fn set_running(&self, running: bool) {
        let mut guard = self.is_running.lock().unwrap();
        *guard = running;
    }

    fn increment_program_counter(&self, target: &str) {
        let target_lower = target.to_ascii_lowercase();
        if target_lower.contains("sh4") {
            if let Some(stripped) = self
                .get_register_value("dc.sh4.cpu", "PC")
                .strip_prefix("0x")
            {
                if let Ok(pc) = u32::from_str_radix(stripped, 16) {
                    let base = 0x8C0000A0;
                    let offset = pc.wrapping_sub(base);
                    let new_pc = base + ((offset + 2) % (8 * 2));
                    self.set_register_value("dc.sh4.cpu", "PC", format!("0x{:08X}", new_pc));
                }
            }
        } else if target_lower.contains("arm7") {
            if let Some(stripped) = self
                .get_register_value("dc.aica.arm7", "PC")
                .strip_prefix("0x")
            {
                if let Ok(pc) = u32::from_str_radix(stripped, 16) {
                    let base = 0x0020_0010;
                    let offset = pc.wrapping_sub(base);
                    let new_pc = base + ((offset + 4) % (8 * 4));
                    self.set_register_value("dc.aica.arm7", "PC", format!("0x{:08X}", new_pc));
                }
            }
        } else if target_lower.contains("dsp") {
            if let Some(stripped) = self
                .get_register_value("dc.aica.dsp", "STEP")
                .strip_prefix("0x")
            {
                if let Ok(step) = u32::from_str_radix(stripped, 16) {
                    let new_step = (step + 1) % 8;
                    self.set_register_value("dc.aica.dsp", "STEP", format!("0x{:03X}", new_step));
                }
            }
        }
    }

    #[allow(dead_code)]
    fn next_event_id(&self) -> u64 {
        let mut guard = self.next_event_id.lock().unwrap();
        let id = *guard;
        *guard += 1;
        id
    }

    #[allow(dead_code)]
    fn push_event(&self, mut entry: EventLogEntry) {
        if entry.event_id.is_empty() {
            entry.event_id = self.next_event_id().to_string();
        }
        entry.timestamp = current_timestamp_ms();
        let mut event_log = self.event_log.lock().unwrap();
        event_log.push(entry);
        if event_log.len() > EVENT_LOG_LIMIT {
            let remove = event_log.len() - EVENT_LOG_LIMIT;
            event_log.drain(0..remove);
        }
    }

    fn build_tick(&self, dreamcast_ptr: usize, hit_breakpoint_id: Option<u32>) -> DebuggerTick {
        let device_tree = self.build_device_tree();
        let all_registers = collect_registers_from_tree(&device_tree);

        let mut registers_by_id: HashMap<String, Vec<RegisterValue>> = HashMap::new();
        for (path, registers) in all_registers {
            registers_by_id.insert(path, registers);
        }

        if dreamcast_ptr != 0 {
            let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;

            let sh4_registers = [
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
            let mut sh4_cpu_regs = Vec::new();
            for (name, width) in sh4_registers {
                if let Some(value) = nulldc::dreamcast::get_sh4_register(dreamcast, name) {
                    sh4_cpu_regs.push(RegisterValue {
                        name: name.to_string(),
                        value: format!("0x{:08X}", value),
                        width,
                        flags: None,
                        metadata: None,
                    });
                }
            }
            for idx in 0..16 {
                let reg_name = format!("R{}", idx);
                if let Some(value) = nulldc::dreamcast::get_sh4_register(dreamcast, &reg_name) {
                    sh4_cpu_regs.push(RegisterValue {
                        name: reg_name,
                        value: format!("0x{:08X}", value),
                        width: 32,
                        flags: None,
                        metadata: None,
                    });
                }
            }
            if !sh4_cpu_regs.is_empty() {
                registers_by_id.insert("dc.sh4.cpu".to_string(), sh4_cpu_regs);
            }

            let mut arm_regs = Vec::new();
            if let Some(value) = nulldc::dreamcast::get_arm_register(dreamcast, "PC") {
                arm_regs.push(RegisterValue {
                    name: "PC".to_string(),
                    value: format!("0x{:08X}", value),
                    width: 32,
                    flags: None,
                    metadata: None,
                });
            }
            for idx in 0..16 {
                let reg_name = format!("R{}", idx);
                if let Some(value) = nulldc::dreamcast::get_arm_register(dreamcast, &reg_name) {
                    arm_regs.push(RegisterValue {
                        name: reg_name,
                        value: format!("0x{:08X}", value),
                        width: 32,
                        flags: None,
                        metadata: None,
                    });
                }
            }
            if !arm_regs.is_empty() {
                registers_by_id.insert("dc.aica.arm7".to_string(), arm_regs);
            }
        }

        let breakpoints_by_id = self
            .breakpoints
            .lock()
            .unwrap()
            .iter()
            .map(|(id, bp)| (id.to_string(), bp.clone()))
            .collect::<HashMap<_, _>>();

        let watches = {
            let watch_map = self.watches.lock().unwrap();
            if watch_map.is_empty() {
                None
            } else {
                Some(
                    watch_map
                        .values()
                        .map(|watch| WatchDescriptor {
                            id: watch.id,
                            expression: watch.expression.clone(),
                            value: self.evaluate_watch_expression(dreamcast_ptr, &watch.expression),
                        })
                        .collect::<Vec<_>>(),
                )
            }
        };

        let mut callstacks = HashMap::new();

        let sh4_pc = if dreamcast_ptr != 0 {
            let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
            nulldc::dreamcast::get_sh4_register(dreamcast, "PC")
                .map(|value| value as u64)
                .unwrap_or(0x8C00_00A0)
        } else {
            self.get_register_value("dc.sh4.cpu", "PC")
                .strip_prefix("0x")
                .and_then(|value| u64::from_str_radix(value, 16).ok())
                .unwrap_or(0x8C00_00A0)
        };
        let sh4_frames = (0..16)
            .map(|index| CallstackFrame {
                index,
                pc: if index == 0 {
                    sh4_pc
                } else {
                    0x8C00_0000 + (index - 1) as u64 * 4
                },
                sp: Some(0x0CFE_0000 - index as u64 * 16),
                symbol: Some(format!("SH4_func_{}", index)),
                location: Some(format!("sh4.c:{}", 100 + index)),
            })
            .collect::<Vec<_>>();
        callstacks.insert("sh4".to_string(), sh4_frames);

        let arm7_pc = if dreamcast_ptr != 0 {
            let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
            nulldc::dreamcast::get_arm_register(dreamcast, "PC")
                .map(|value| value as u64)
                .unwrap_or(0x0020_0010)
        } else {
            self.get_register_value("dc.aica.arm7", "PC")
                .strip_prefix("0x")
                .and_then(|value| u64::from_str_radix(value, 16).ok())
                .unwrap_or(0x0020_0010)
        };
        let arm7_frames = (0..16)
            .map(|index| CallstackFrame {
                index,
                pc: if index == 0 {
                    arm7_pc
                } else {
                    0x0020_0000 + (index - 1) as u64 * 4
                },
                sp: Some(0x0028_0000 - index as u64 * 16),
                symbol: Some(format!("ARM7_func_{}", index)),
                location: Some(format!("arm7.c:{}", 100 + index)),
            })
            .collect::<Vec<_>>();
        callstacks.insert("arm7".to_string(), arm7_frames);

        let tick_id = {
            let mut guard = self.tick_id.lock().unwrap();
            let id = *guard;
            *guard += 1;
            id
        };

        let timestamp = current_timestamp_ms();

        let is_running = if dreamcast_ptr != 0 {
            let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
            nulldc::dreamcast::is_dreamcast_running(dreamcast)
        } else {
            *self.is_running.lock().unwrap()
        };

        DebuggerTick {
            tick_id,
            timestamp,
            execution_state: ExecutionState {
                state: if is_running {
                    "running".to_string()
                } else {
                    "paused".to_string()
                },
                breakpoint_id: hit_breakpoint_id,
            },
            registers: registers_by_id,
            breakpoints: breakpoints_by_id,
            event_log: self.event_log.lock().unwrap().clone(),
            watches,
            callstacks: Some(callstacks),
        }
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
        match target {
            "arm7" => {
                nulldc::dreamcast::read_arm_memory_slice(dreamcast, base_address, effective_length)
            }
            _ => nulldc::dreamcast::read_memory_slice(dreamcast, base_address, effective_length),
        }
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
                    "capabilities": CAPABILITIES,
                }),
                true,
            ))
        }

        "state.getMemorySlice" => {
            let target = params
                .get("target")
                .and_then(|value| value.as_str())
                .unwrap_or("sh4");
            let address = params.get("address").and_then(|value| value.as_u64());
            let length = params
                .get("length")
                .and_then(|value| value.as_u64())
                .map(|value| value as usize);
            let slice = build_memory_slice(dreamcast_ptr, target, address, length);
            Ok((serde_json::to_value(slice).unwrap(), false))
        }

        "state.getDisassembly" => {
            let target = params
                .get("target")
                .and_then(|value| value.as_str())
                .unwrap_or("sh4");
            let address = params
                .get("address")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let count = params
                .get("count")
                .and_then(|value| value.as_u64())
                .unwrap_or(128) as usize;

            let lines = if dreamcast_ptr != 0 {
                let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
                match target {
                    "sh4" => nulldc::dreamcast::disassemble_sh4(dreamcast, address, count)
                        .into_iter()
                        .map(|line| DisassemblyLine {
                            address: line.address,
                            bytes: line.bytes,
                            disassembly: line.disassembly,
                        })
                        .collect::<Vec<_>>(),
                    "arm7" => nulldc::dreamcast::disassemble_arm7(dreamcast, address, count)
                        .into_iter()
                        .map(|line| DisassemblyLine {
                            address: line.address,
                            bytes: line.bytes,
                            disassembly: line.disassembly,
                        })
                        .collect::<Vec<_>>(),
                    _ => generate_disassembly(target, address, count),
                }
            } else {
                generate_disassembly(target, address, count)
            };

            Ok((json!({ "lines": lines }), false))
        }

        "state.getCallstack" => {
            let target = params
                .get("target")
                .and_then(|value| value.as_str())
                .unwrap_or("sh4");
            let max_frames = params
                .get("maxFrames")
                .and_then(|value| value.as_u64())
                .unwrap_or(16)
                .min(64) as usize;

            let frames: Vec<CallstackFrame> = (0..max_frames)
                .map(|index| CallstackFrame {
                    index: index as u32,
                    pc: 0x8c000000 + index as u64 * 4,
                    sp: Some(0x0cfe0000 - index as u64 * 16),
                    symbol: Some(format!("{}_func_{}", target.to_uppercase(), index)),
                    location: Some(format!("{}.c:{}", target, 100 + index)),
                })
                .collect();

            Ok((json!({ "target": target, "frames": frames }), false))
        }

        "state.watch" => {
            let expressions = params
                .get("expressions")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|value| value.as_str().map(|s| s.to_owned()))
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
            let watch_ids = params
                .get("watchIds")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|value| value.as_u64())
                .map(|value| value as u32)
                .collect::<Vec<_>>();

            let mut watches = state.watches.lock().unwrap();
            for id in watch_ids {
                watches.remove(&id);
            }

            Ok((json!({}), true))
        }

        "state.editWatch" => {
            let watch_id = params
                .get("watchId")
                .and_then(|value| value.as_u64())
                .map(|value| value as u32);
            let value = params
                .get("value")
                .and_then(|value| value.as_str())
                .unwrap_or("");

            if let Some(id) = watch_id {
                let expression = {
                    let watches = state.watches.lock().unwrap();
                    watches.get(&id).map(|watch| watch.expression.clone())
                };

                if let Some(expr) = expression {
                    let parts: Vec<&str> = expr.split('.').collect();
                    let (path, name) = if parts.len() > 1 {
                        let name = parts.last().unwrap();
                        let path = parts[..parts.len() - 1].join(".");
                        (path, name.to_string())
                    } else {
                        ("dc.sh4.cpu".to_string(), parts[0].to_string())
                    };
                    let key = format!("{}.{}", path, name.to_lowercase());

                    {
                        let registers = state.register_values.lock().unwrap();
                        if !registers.contains_key(&key) {
                            return Ok((
                                json!({
                                    "error": {
                                        "code": -32602,
                                        "message": format!(
                                            "Cannot edit non-register expression \"{}\"",
                                            expr
                                        ),
                                    }
                                }),
                                false,
                            ));
                        }
                    }

                    state.set_register_value(&path, &name, value.to_string());
                    return Ok((json!({}), true));
                }

                return Ok((
                    json!({
                        "error": {
                            "code": -32602,
                            "message": format!("Watch \"{}\" not found", id),
                        }
                    }),
                    false,
                ));
            }

            Ok((
                json!({
                    "error": {
                        "code": -32602,
                        "message": "Watch not found or cannot edit",
                    }
                }),
                false,
            ))
        }

        "state.modifyWatchExpression" => {
            let watch_id = params
                .get("watchId")
                .and_then(|value| value.as_u64())
                .map(|value| value as u32);
            let new_expression = params
                .get("newExpression")
                .and_then(|value| value.as_str())
                .unwrap_or("");

            if let Some(id) = watch_id {
                let mut watches = state.watches.lock().unwrap();
                if let Some(watch) = watches.get_mut(&id) {
                    watch.expression = new_expression.to_string();
                    return Ok((json!({}), true));
                }

                return Ok((
                    json!({
                        "error": {
                            "code": -32602,
                            "message": format!("Watch {} not found", id),
                        }
                    }),
                    false,
                ));
            }

            Ok((
                json!({
                    "error": {
                        "code": -32602,
                        "message": "Watch not found",
                    }
                }),
                false,
            ))
        }

        "control.pause" => {
            if dreamcast_ptr != 0 {
                let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
                nulldc::dreamcast::set_dreamcast_running(dreamcast, false);
            }
            state.set_running(false);
            Ok((json!({}), true))
        }

        "control.step" | "control.stepOver" | "control.stepOut" => {
            let target = params
                .get("target")
                .and_then(|value| value.as_str())
                .unwrap_or("sh4");

            if dreamcast_ptr != 0 {
                let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
                nulldc::dreamcast::step_dreamcast(dreamcast);
                nulldc::dreamcast::set_dreamcast_running(dreamcast, false);
            } else {
                state.increment_program_counter(target);
            }

            state.set_running(false);
            Ok((json!({}), true))
        }

        "control.runUntil" => {
            if dreamcast_ptr != 0 {
                let dreamcast = dreamcast_ptr as *mut nulldc::dreamcast::Dreamcast;
                nulldc::dreamcast::set_dreamcast_running(dreamcast, true);
            }
            state.set_running(true);
            Ok((json!({}), true))
        }

        "breakpoints.add" => {
            let event = params
                .get("event")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let address = params.get("address").and_then(|value| value.as_u64());
            let kind = params
                .get("kind")
                .and_then(|value| value.as_str())
                .unwrap_or("code");
            let enabled = params
                .get("enabled")
                .and_then(|value| value.as_bool())
                .unwrap_or(true);

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
            if let Some(id) = params
                .get("id")
                .and_then(|value| value.as_u64())
                .map(|value| value as u32)
            {
                let removed = state.breakpoints.lock().unwrap().remove(&id).is_some();
                if removed {
                    return Ok((json!({}), true));
                }

                return Ok((
                    json!({
                        "error": {
                            "code": -32000,
                            "message": format!("Breakpoint {} not found", id),
                        }
                    }),
                    false,
                ));
            }

            Ok((json!({}), false))
        }

        "breakpoints.toggle" => {
            let id = params
                .get("id")
                .and_then(|value| value.as_u64())
                .map(|value| value as u32);
            let enabled = params
                .get("enabled")
                .and_then(|value| value.as_bool())
                .unwrap_or(true);

            if let Some(id) = id {
                let mut breakpoints = state.breakpoints.lock().unwrap();
                if let Some(bp) = breakpoints.get_mut(&id) {
                    bp.enabled = enabled;
                    return Ok((json!({}), true));
                }

                return Ok((
                    json!({
                        "error": {
                            "code": -32000,
                            "message": format!("Breakpoint {} not found", id),
                        }
                    }),
                    false,
                ));
            }

            Ok((json!({}), false))
        }

        "breakpoints.setCategoryStates" => {
            if let Some(categories) = params.get("categories").and_then(|value| value.as_object()) {
                let mut category_states = state.category_states.lock().unwrap();
                for (category, state_value) in categories {
                    if let (Some(muted), Some(soloed)) = (
                        state_value.get("muted").and_then(|value| value.as_bool()),
                        state_value.get("soloed").and_then(|value| value.as_bool()),
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
