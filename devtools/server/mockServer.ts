import express from "express";
import { createServer as createHttpServer } from "node:http";
import { WebSocketServer, type WebSocket } from "ws";
import { randomUUID, createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createServer as createViteServer } from "vite";
import { z } from "zod";
import {
  JSON_RPC_VERSION,
  type JsonRpcMessage,
  type JsonRpcRequest,
  type JsonRpcSuccess,
  type JsonRpcError,
} from "../src/lib/jsonRpc";
import type {
  BreakpointDescriptor,
  DebuggerNotification,
  DebuggerRpcSchema,
  DebuggerShape,
  DebuggerTick,
  DeviceNodeDescriptor,
  DisassemblyLine,
  EventLogEntry,
  MemorySlice,
  RegisterValue,
  RpcError,
} from "../src/lib/debuggerSchema";
import {
  DebuggerRpcMethodSchemas,
  DebuggerTickSchema,
  PANEL_IDS,
} from "../src/lib/debuggerSchema";

const PORT = Number(process.env.PORT ?? 5173);
const WS_PATH = process.env.DEBUGGER_WS_PATH ?? "/ws";
const IS_PRODUCTION = process.env.NODE_ENV === "production";

// SGC frame data constants
const CHANNELS_PER_FRAME = 64;
const BYTES_PER_CHANNEL = 128;
const BYTES_PER_FRAME = CHANNELS_PER_FRAME * BYTES_PER_CHANNEL; // 8192
const NUM_FRAMES = 1024;
const TOTAL_BYTES = NUM_FRAMES * BYTES_PER_FRAME;

// Mock SGC frame data buffer (allocated on startup)
let sgcFrameData: Buffer;

interface ClientContext {
  socket: WebSocket;
  sessionId: string;
  watches: Set<string>;
}

interface ServerWatch {
  id: number;
  expression: string;
}

const serverWatches = new Map<number, ServerWatch>(); // id -> {id, expression}
const serverBreakpoints = new Map<number, BreakpointDescriptor>();
let isRunning = true; // Execution state
let nextEventId = 1n; // Event ID counter (BigInt for 64-bit)
let tickId = 0; // Tick counter
let nextWatchId = 1; // Watch ID counter
let nextBreakpointId = 1; // Breakpoint ID counter

// Initialize default watches
const DEFAULT_WATCH_EXPRESSIONS = ["dc.sh4.cpu.pc", "dc.sh4.dmac.dmaor"];
for (const expr of DEFAULT_WATCH_EXPRESSIONS) {
  const id = nextWatchId++;
  serverWatches.set(id, { id, expression: expr });
}

type BreakpointCategory = "events" | "sh4" | "arm7" | "dsp";

interface CategoryState {
  muted: boolean;
  soloed: boolean;
}

const categoryStates = new Map<BreakpointCategory, CategoryState>([
  ["events", { muted: false, soloed: false }],
  ["sh4", { muted: false, soloed: false }],
  ["arm7", { muted: false, soloed: false }],
  ["dsp", { muted: false, soloed: false }],
]);

const categorizeBreakpoint = (bp: BreakpointDescriptor): BreakpointCategory => {
  // All event breakpoints go to "events" category
  if (bp.kind === "event") {
    return "events";
  }

  // Code breakpoints are categorized by processor
  const lower = bp.event.toLowerCase();
  if (lower.includes("sh4")) return "sh4";
  if (lower.includes("arm7")) return "arm7";
  if (lower.includes("aica") || lower.includes("dsp")) return "dsp";

  // Default to events if we can't determine
  return "events";
};

const isBreakpointActive = (bp: BreakpointDescriptor): boolean => {
  const category = categorizeBreakpoint(bp);
  const state = categoryStates.get(category);
  if (!state) return true;

  // If muted, it's inactive
  if (state.muted) return false;

  // Check if any category is soloed
  const anyCategorySoloed = Array.from(categoryStates.values()).some((s) => s.soloed);

  // If any category is soloed, only soloed categories are active
  if (anyCategorySoloed) {
    return state.soloed;
  }

  return true;
};

// Shared register values storage
const registerValues = new Map<string, string>([
  // SH4 Core
  ["dc.sh4.cpu.pc", "0x8C0000A0"],
  ["dc.sh4.cpu.pr", "0x8C0000A2"],
  ["dc.sh4.vbr", "0x8C000000"],
  ["dc.sh4.sr", "0x40000000"],
  ["dc.sh4.fpscr", "0x00040001"],
  // SH4 Caches
  ["dc.sh4.icache.icache_ctrl", "0x00000003"],
  ["dc.sh4.dcache.dcache_ctrl", "0x00000003"],
  // Holly/DMAC
  ["dc.holly.holly_id", "0x00050000"],
  ["dc.holly.dmac_ctrl", "0x00000001"],
  ["dc.holly.dmac.dmaor", "0x8201"],
  ["dc.holly.dmac.chcr0", "0x00000001"],
  // Holly/TA
  ["dc.holly.ta.ta_list_base", "0x0C000000"],
  ["dc.holly.ta.ta_status", "0x00000000"],
  // Holly/CORE
  ["dc.holly.core.pvr_ctrl", "0x00000001"],
  ["dc.holly.core.pvr_status", "0x00010000"],
  // AICA
  ["dc.aica.aica_ctrl", "0x00000002"],
  ["dc.aica.aica_status", "0x00000001"],
  ["dc.aica.arm7.pc", "0x00200010"],
  ["dc.aica.channels.ch0_vol", "0x7F"],
  ["dc.aica.channels.ch1_vol", "0x6A"],
  ["dc.aica.dsp.step", "0x000"],
  ["dc.aica.dsp.dsp_acc", "0x1F"],
  // System
  ["dc.sysclk", "200MHz"],
  ["dc.asic_rev", "0x0001"],
]);

const getRegisterValue = (path: string, name: string): string => {
  const key = `${path}.${name.toLowerCase()}`;
  return registerValues.get(key) ?? "0x00000000";
};

const setRegisterValue = (path: string, name: string, value: string): void => {
  const key = `${path}.${name.toLowerCase()}`;
  registerValues.set(key, value);
};

const buildDeviceTree = (): DeviceNodeDescriptor[] => [
  {
    path: "dc",
    label: "Dreamcast",
    description: "beloved console",
    registers: [
      { name: "SYSCLK", value: getRegisterValue("dc", "SYSCLK"), width: 0 },
      { name: "ASIC_REV", value: getRegisterValue("dc", "ASIC_REV"), width: 16 },
    ],
    children: [
      {
        path: "dc.sh4",
        label: "SH4",
        description: "SH7750-alike SoC",
        events: [
          "dc.sh4.interrupt",
          "dc.sh4.exception",
          "dc.sh4.tlb_miss",
        ],
        children: [
          {
            path: "dc.sh4.cpu",
            label: "Core",
            description: "SuperH4 CPU Core",
            registers: [
              { name: "PC", value: getRegisterValue("dc.sh4.cpu", "PC"), width: 32 },
              { name: "PR", value: getRegisterValue("dc.sh4.cpu", "PR"), width: 32 },
              { name: "VBR", value: getRegisterValue("dc.sh4", "VBR"), width: 32 },
              { name: "SR", value: getRegisterValue("dc.sh4", "SR"), width: 32 },
              { name: "FPSCR", value: getRegisterValue("dc.sh4", "FPSCR"), width: 32 },
            ],
            actions: [PANEL_IDS.SH4_DISASSEMBLY, PANEL_IDS.SH4_MEMORY],
          },
          {
            path: "dc.sh4.pbus",
            label: "PBUS",
            description: "Peripherals Bus",
            children: [
            {
              path: "dc.sh4.bsc",
              label: "BSC",
              description: "Bus State Controller",
              actions: [PANEL_IDS.SH4_BSC_REGISTERS],
            },
            {
              path: "dc.sh4.ccn",
              label: "CCN",
              description: "Cache Controller",
              actions: [PANEL_IDS.SH4_CCN_REGISTERS],
            },
            {
              path: "dc.sh4.cpg",
              label: "CPG",
              description: "Clock Pulse Generator",
              actions: [PANEL_IDS.SH4_CPG_REGISTERS],
            },
            {
              path: "dc.sh4.dmac",
              label: "DMAC",
              description: "Direct Memory Access Controller",
              actions: [PANEL_IDS.SH4_DMAC_REGISTERS],
            },
            {
              path: "dc.sh4.intc",
              label: "INTC",
              description: "Interrupt Controller",
              actions: [PANEL_IDS.SH4_INTC_REGISTERS],
            },
            {
              path: "dc.sh4.rtc",
              label: "RTC",
              description: "Real Time Clock",
              actions: [PANEL_IDS.SH4_RTC_REGISTERS],
            },
            {
              path: "dc.sh4.sci",
              label: "SCI",
              description: "Serial Communications Interface",
              actions: [PANEL_IDS.SH4_SCI_REGISTERS],
            },
            {
              path: "dc.sh4.scif",
              label: "SCIF",
              description: "Serial Communications Interface w/ FIFO",
              actions: [PANEL_IDS.SH4_SCIF_REGISTERS],
            },
            {
              path: "dc.sh4.tmu",
              label: "TMU",
              description: "Timer Unit",
              actions: [PANEL_IDS.SH4_TMU_REGISTERS],
            },
            {
              path: "dc.sh4.ubc",
              label: "UBC",
              description: "User Break Controller",
              actions: [PANEL_IDS.SH4_UBC_REGISTERS],
            },
            {
              path: "dc.sh4.sq",
              label: "SQ",
              description: "Store Queues",
              actions: [PANEL_IDS.SH4_SQ_CONTENTS],
            },
            {
              path: "dc.sh4.icache",
              label: "ICACHE",
              description: "Instruction Cache",
              actions: [PANEL_IDS.SH4_ICACHE_CONTENTS],
            },
            {
              path: "dc.sh4.ocache",
              label: "OCACHE",
              description: "Operand Cache",
              actions: [PANEL_IDS.SH4_OCACHE_CONTENTS],
            },
            {
              path: "dc.sh4.ocram",
              label: "OCRAM",
              description: "Operand RAM",
              actions: [PANEL_IDS.SH4_OCRAM_CONTENTS],
            },
            {
              path: "dc.sh4.tlb",
              label: "TLB",
              description: "Translation Lookaside Buffer",
              actions: [PANEL_IDS.SH4_TLB_CONTENTS],
            },
            ],
          }
        ]
      },
      {
        path: "dc.holly",
        label: "Holly",
        description: "System ASIC",
        registers: [
          { name: "HOLLY_ID", value: getRegisterValue("dc.holly", "HOLLY_ID"), width: 32 },
          { name: "DMAC_CTRL", value: getRegisterValue("dc.holly", "DMAC_CTRL"), width: 32 },
        ],
        children: [
          {
            path: "dc.holly.dmac",
            label: "DMA Controller",
            description: "peripheral",
            registers: [
              { name: "DMAOR", value: getRegisterValue("dc.holly.dmac", "DMAOR"), width: 16 },
              { name: "CHCR0", value: getRegisterValue("dc.holly.dmac", "CHCR0"), width: 32 },
            ],
            events: [
              "dc.holly.dmac.transfer_start",
              "dc.holly.dmac.transfer_end",
            ],
          },
          {
            path: "dc.holly.ta",
            label: "TA",
            description: "Tile Accelerator",
            actions: [PANEL_IDS.CLX2_TA],
            registers: [
              { name: "TA_LIST_BASE", value: getRegisterValue("dc.holly.ta", "TA_LIST_BASE"), width: 32 },
              { name: "TA_STATUS", value: getRegisterValue("dc.holly.ta", "TA_STATUS"), width: 32 },
            ],
            events: [
              "dc.holly.ta.list_init",
              "dc.holly.ta.list_end",
              "dc.holly.ta.opaque_complete",
              "dc.holly.ta.translucent_complete",
            ],
          },
          {
            path: "dc.holly.core",
            label: "CORE",
            description: "Depth and Shading Engine",
            actions: [PANEL_IDS.CLX2_CORE],
            registers: [
              { name: "PVR_CTRL", value: getRegisterValue("dc.holly.core", "PVR_CTRL"), width: 32 },
              { name: "PVR_STATUS", value: getRegisterValue("dc.holly.core", "PVR_STATUS"), width: 32 },
            ],
            events: [
              "dc.holly.core.render_start",
              "dc.holly.core.render_end",
              "dc.holly.core.vblank",
            ],
          },
        ],
      },
      {
        path: "dc.aica",
        label: "AICA",
        description: "Sound SoC",
        registers: [
          { name: "AICA_CTRL", value: getRegisterValue("dc.aica", "AICA_CTRL"), width: 32 },
          { name: "AICA_STATUS", value: getRegisterValue("dc.aica", "AICA_STATUS"), width: 32 },
        ],
        events: [
          "dc.aica.interrupt",
          "dc.aica.timer",
        ],
        children: [
          {
            path: "dc.aica.arm7",
            label: "ARM7",
            description: "ARM7DI CPU Core",
            registers: [
              { name: "PC", value: getRegisterValue("dc.aica.arm7", "PC"), width: 32 },
            ],
            actions: [PANEL_IDS.ARM7_DISASSEMBLY, PANEL_IDS.ARM7_MEMORY],
          },
          {
            path: "dc.aica.sgc",
            label: "SGC",
            description: "Sound Generation Core",
            actions: [PANEL_IDS.SGC],
            events: [
              "dc.aica.channels.key_on",
              "dc.aica.channels.key_off",
              "dc.aica.channels.loop",
            ],
            children: [
              {
                path: "dc.aica.sgc.0",
                label: "Channel 0",
                description: "SGC Channel 0",
                registers: [
                  { name: "VOL", value: getRegisterValue("dc.aica.channels", "CH0_VOL"), width: 8 },
                ],
                events: [
                  "dc.aica.channel.0.key_on",
                  "dc.aica.channel.0.key_off",
                  "dc.aica.channel.0.loop",
                ],
              },
              {
                path: "dc.aica.sgc.1",
                label: "Channel 1",
                description: "SGC Channel 1",
                registers: [
                  { name: "VOL", value: getRegisterValue("dc.aica.channels", "CH1_VOL"), width: 8 },
                ],
                events: [
                  "dc.aica.channel.0.key_on",
                  "dc.aica.channel.0.key_off",
                  "dc.aica.channel.0.loop",
                ],
              }
            ]
          },
          {
            path: "dc.aica.dsp",
            label: "DSP",
            description: "DSP VLIW Core",
            registers: [
              { name: "STEP", value: getRegisterValue("dc.aica.dsp", "STEP"), width: 16 },
              { name: "DSP_ACC", value: getRegisterValue("dc.aica.dsp", "DSP_ACC"), width: 16 },
            ],
            events: [
              "dc.aica.dsp.step",
              "dc.aica.dsp.sample_start",
            ],
            actions: [PANEL_IDS.DSP_DISASSEMBLY],
          },
        ],
      },
    ],
  },
];

const sh4Instructions = [
  { mnemonic: "mov.l", operands: (r1: number, r2: number, _r3: number, _val: number, _offset: number) => `@r${r1}+, r${r2}`, bytes: 2 },
  { mnemonic: "mov", operands: (r1: number, r2: number, _r3: number, _val: number, _offset: number) => `r${r1}, r${r2}`, bytes: 2 },
  { mnemonic: "sts.l", operands: (r1: number, _r2: number, _r3: number, _val: number, _offset: number) => `pr, @-r${r1}`, bytes: 2 },
  { mnemonic: "add", operands: (r1: number, r2: number, _r3: number, _val: number, _offset: number) => `r${r1}, r${r2}`, bytes: 2 },
  { mnemonic: "cmp/eq", operands: (r1: number, r2: number, _r3: number, _val: number, _offset: number) => `r${r1}, r${r2}`, bytes: 2 },
  { mnemonic: "bf", operands: (_r1: number, _r2: number, _r3: number, _val: number, offset: number) => `0x${offset.toString(16)}`, bytes: 2 },
  { mnemonic: "jmp", operands: (r: number, _r2: number, _r3: number, _val: number, _offset: number) => `@r${r}`, bytes: 2 },
  { mnemonic: "nop", operands: (_r1: number, _r2: number, _r3: number, _val: number, _offset: number) => "", bytes: 2 },
];

const arm7Instructions = [
  { mnemonic: "mov", operands: (r1: number, _r2: number, _r3: number, val: number, _offset: number) => `r${r1}, #${val}`, bytes: 4 },
  { mnemonic: "ldr", operands: (r1: number, r2: number, _r3: number, _val: number, offset: number) => `r${r1}, [r${r2}, #${offset}]`, bytes: 4 },
  { mnemonic: "str", operands: (r1: number, r2: number, _r3: number, _val: number, _offset: number) => `r${r1}, [r${r2}]`, bytes: 4 },
  { mnemonic: "add", operands: (r1: number, r2: number, r3: number, _val: number, _offset: number) => `r${r1}, r${r2}, r${r3}`, bytes: 4 },
  { mnemonic: "sub", operands: (r1: number, r2: number, r3: number, _val: number, _offset: number) => `r${r1}, r${r2}, r${r3}`, bytes: 4 },
  { mnemonic: "bx", operands: (r: number, _r2: number, _r3: number, _val: number, _offset: number) => `r${r}`, bytes: 4 },
  { mnemonic: "bl", operands: (_r1: number, _r2: number, _r3: number, _val: number, offset: number) => `0x${offset.toString(16)}`, bytes: 4 },
  { mnemonic: "nop", operands: (_r1: number, _r2: number, _r3: number, _val: number, _offset: number) => "", bytes: 4 },
];

const dspSampleProgram = [
  "TWT TWA YSEL BSELXSEL YSEL IRA:2 ZERO NOFL MASA:6 NXADRTRA YSEL BSEL",
  "YSEL IWT IWA:6 MWT BSEL NOFL",
  "YSEL BSEL",
  "YSEL MRD BSEL NOFL MASA:7",
  "YSEL BSEL",
  "YSEL IWT IWA:7 MRD BSEL NOFL MASA:8",
  "YSEL BSEL",
  "YSEL IWT IWA:8 MRD BSEL NOFL MASA:9",
  "YSEL BSEL",
  "YSEL IWT IWA:9 MRD BSEL NOFL MASA:10XSEL YSEL IRA:2 ZERO",
  "TWT YSEL IWT IWA:10 MRD BSEL NOFL MASA:11XSEL YSEL IRA:3 NEGB BSELXSEL YSEL IRA:33 IWT IWA:11 MRD BSEL NOFL MASA:12",
  "TWT TWA YSEL BSELXSEL YSEL IRA:2 IWT IWA:12 MRD ZERO NOFL MASA:13TRA YSEL BSEL",
  "YSEL IWT IWA:13 MWT BSEL NOFL MASA:2",
  "YSEL BSEL",
  "YSEL MRD BSEL NOFL MASA:14",
  "YSEL BSEL",
  "YSEL IWT IWA:14 MRD BSEL NOFL MASA:15",
  "YSEL BSEL",
  "YSEL IWT IWA:15 MRD BSEL NOFL MASA:17",
  "YSEL BSEL",
  "YSEL IWT IWA:17 MRD BSEL NOFL MASA:19XSEL YSEL IRA:4 ZERO",
  "TWT YSEL IWT IWA:19 MRD BSEL NOFL MASA:21XSEL YSEL IRA:5 NEGB BSELXSEL YSEL IRA:32 IWT IWA:21 MRD BSEL NOFL MASA:23",
  "TWT TWA YSEL BSELXSEL YSEL IRA:4 IWT IWA:23 ZEROTRA YSEL BSEL",
  "YSEL BSELXSEL YSEL IRA:6 ZERO",
  "TWT YSEL BSELXSEL YSEL IRA:7 NEGB BSELXSEL YSEL IRA:33 BSEL",
  "TWT TWA YSEL BSELXSEL YSEL IRA:6 ZEROTRA YSEL BSEL",
  "YSEL BSELXSEL YSEL IRA:8 ZEROXSEL YSEL IRA:9 BSEL",
  "XSEL YSEL IRA:10 BSEL",
  "XSEL YSEL IRA:11 BSEL",
  "TWT TWA:2 YSEL BSEL",
  "YSEL BSELXSEL YSEL IRA:12 ZEROXSEL YSEL IRA:13 BSEL",
  "XSEL YSEL IRA:14 BSEL",
  "XSEL YSEL IRA:15 BSEL",
  "TWT TWA:3 YSEL BSEL",
  "YSEL BSELTRA:2 XSEL YSEL IRA:17",
  "TWT TWA:2 YSEL BSEL",
  "YSEL BSEL",
  "YSEL MWT BSEL NOFL MASA:16XSEL YSEL IRA:17 ZEROTRA:2 YSEL BSEL",
  "TWT TWA:2 YSEL BSEL",
  "YSEL BSELTRA:3 XSEL YSEL IRA:19",
  "TWT TWA:3 YSEL BSEL",
  "YSEL BSEL",
  "YSEL MWT BSEL NOFL MASA:18XSEL YSEL IRA:19 ZEROTRA:3 YSEL BSEL",
  "TWT TWA:3 YSEL BSEL",
  "YSEL BSELTRA:2 XSEL YSEL IRA:21",
  "TWT TWA:2 YSEL BSEL",
  "YSEL BSEL",
  "YSEL MWT BSEL NOFL MASA:20XSEL YSEL IRA:21 ZEROTRA:2 YSEL BSEL",
  "YSEL EWT BSEL",
  "YSEL BSELTRA:3 XSEL YSEL IRA:23",
  "TWT TWA:3 YSEL BSEL",
  "YSEL BSEL",
  "YSEL MWT BSEL NOFL MASA:22XSEL YSEL IRA:23 ZEROTRA:3 YSEL BSEL",
  "YSEL EWT EWA BSEL",
];

const generateDisassembly = (target: string, address: number, count: number): DisassemblyLine[] => {
  if (target === "dsp") {
    const lines: DisassemblyLine[] = [];
    const sanitizedAddress = Number.isFinite(address) && address >= 0 ? address : 0;
    const startStep = Math.max(0, Math.min(0x7f, sanitizedAddress));

    for (let i = 0; i < count; i++) {
      const step = (startStep + i) & 0x7f;
      const programLine = dspSampleProgram[(startStep + i) % dspSampleProgram.length] ?? "";
      lines.push({
        address: step,
        bytes: ((step * 2) & 0xff).toString(16).toUpperCase().padStart(2, "0"),
        disassembly: programLine,
      });
    }

    return lines;
  }

  const instructionSets = {
    sh4: sh4Instructions,
    arm7: arm7Instructions,
  };

  const selected = instructionSets[target as keyof typeof instructionSets] ?? sh4Instructions;
  const lines: DisassemblyLine[] = [];
  const sanitizedAddress = Number.isFinite(address) && address >= 0 ? address : 0;
  let currentAddr = sanitizedAddress;

  for (let i = 0; i < count; i++) {
    const hash = sha256Byte(`${target}:${currentAddr.toString(16)}`);
    const instrIndex = hash % selected.length;
    const instr = selected[instrIndex];

    const r1 = (hash >> 4) % 16;
    const r2 = (hash >> 2) % 16;
    const r3 = hash % 16;
    const val = (hash * 3) & 0xff;
    const offset = (hash * 7) & 0xfff;

    const operands = instr.operands(r1, r2, r3, val, offset);
    const disassembly = operands ? `${instr.mnemonic} ${operands}` : instr.mnemonic;

    const byteValues: number[] = [];
    for (let b = 0; b < instr.bytes; b++) {
      byteValues.push(sha256Byte(`${target}:${currentAddr.toString(16)}:${b}`));
    }
    const bytes = byteValues.map((b) => b.toString(16).toUpperCase().padStart(2, "0")).join(" " );

    lines.push({
      address: currentAddr,
      bytes,
      disassembly,
    });

    currentAddr += instr.bytes;
  }

  return lines;
};

const EVENT_LOG_LIMIT = 60;

const frameEventGenerators: Array<() => Omit<EventLogEntry, "timestamp" | "eventId">> = [
  () => ({ subsystem: "ta", severity: "info", message: `TA/END_LIST tile ${Math.floor(Math.random() * 32)}` }),
  () => ({ subsystem: "core", severity: "info", message: "CORE/START_RENDER" }),
  () => ({ subsystem: "core", severity: "trace", message: `CORE/QUEUE_SUBMISSION ${Math.floor(Math.random() * 4)}` }),
  () => ({ subsystem: "dsp", severity: "trace", message: "DSP/STEP pipeline advanced" }),
  () => ({ subsystem: "aica", severity: "info", message: "AICA/SGC/STEP channel 0" }),
  () => ({ subsystem: "sh4", severity: "warn", message: "SH4/INTERRUPT IRQ5 asserted" }),
  () => ({ subsystem: "holly", severity: "info", message: "HOLLY/START_RENDER pass" }),
];

const createFrameEvent = (): EventLogEntry => {
  const generator = frameEventGenerators[Math.floor(Math.random() * frameEventGenerators.length)];
  const event = generator();
  return {
    eventId: (nextEventId++).toString(),
    timestamp: Date.now(),
    ...event,
  };
};

const eventLogEntries: EventLogEntry[] = Array.from({ length: 6 }, () => createFrameEvent());

const clients = new Set<ClientContext>();

const sendNotification = (client: ClientContext, notification: DebuggerNotification) => {
  const method = notification.topic === "tick" ? "event.tick" : `event.${notification.topic}`;
  const payload = JSON.stringify({
    jsonrpc: JSON_RPC_VERSION,
    method,
    params: notification.payload,
  });
  client.socket.send(payload);
};

const handleRequest = async (client: ClientContext, message: JsonRpcRequest) => {
  try {
    const method = message.method as keyof DebuggerRpcSchema;
    const params = message.params ?? {};

    // Validate params using Zod schema
    const methodSchema = DebuggerRpcMethodSchemas[method as keyof typeof DebuggerRpcMethodSchemas];
    if (methodSchema?.params) {
      try {
        methodSchema.params.parse(params);
      } catch (error) {
        if (error instanceof z.ZodError) {
          throw new Error(`Invalid parameters for ${method}: ${error.message}`);
        }
        throw error;
      }
    }

    const { result, shouldBroadcastTick } = await dispatchMethod(method, params as Record<string, unknown>, client);

    // Validate result using Zod schema
    if (methodSchema?.result) {
      try {
        methodSchema.result.parse(result);
      } catch (error) {
        if (error instanceof z.ZodError) {
          console.error(`Invalid result for ${method}:`, error.message);
          // Still send the result, but log the validation error
        }
      }
    }

    await respondSuccess(client.socket, message.id, result);

    // Send binary data for specific methods
    if (method === "state.getSgcFrameData") {
      // Send the SGC frame data as a binary message
      client.socket.send(sgcFrameData);
    }

    if (shouldBroadcastTick) {
      broadcastTick();
    }
  } catch (error) {
    respondError(client.socket, message.id, error);
  }
};

const respondSuccess = async (socket: WebSocket, id: JsonRpcSuccess["id"], result: unknown) => {
  const payload: JsonRpcSuccess = { jsonrpc: JSON_RPC_VERSION, id, result };
  socket.send(JSON.stringify(payload));
  // Ensure message is sent before resolving
  await new Promise(resolve => setImmediate(resolve));
};

const respondError = (socket: WebSocket, id: JsonRpcError["id"], error: unknown) => {
  const payload: JsonRpcError = {
    jsonrpc: JSON_RPC_VERSION,
    id,
    error: {
      code: -32000,
      message: error instanceof Error ? error.message : "Unknown error",
      data: error instanceof Error ? { stack: error.stack } : undefined,
    },
  };
  socket.send(JSON.stringify(payload));
};

const incrementProgramCounter = (target: string) => {
  const targetLower = target.toLowerCase();

  if (targetLower.includes("sh4")) {
    // Increment SH4 PC with wraparound (8 instructions * 2 bytes each = 16 bytes)
    const pcValue = registerValues.get("dc.sh4.cpu.pc");
    if (pcValue && pcValue.startsWith("0x")) {
      const pc = Number.parseInt(pcValue, 16);
      const baseAddress = 0x8C0000A0;
      const offset = pc - baseAddress;
      const newOffset = (offset + 2) % (8 * 2);
      const newPc = baseAddress + newOffset;
      setRegisterValue("dc.sh4.cpu", "PC", `0x${newPc.toString(16).toUpperCase().padStart(8, "0")}`);
    }
  } else if (targetLower.includes("arm7")) {
    // Increment ARM7 PC with wraparound (8 instructions * 4 bytes each = 32 bytes)
    const arm7PcValue = registerValues.get("dc.aica.arm7.pc");
    if (arm7PcValue && arm7PcValue.startsWith("0x")) {
      const arm7Pc = Number.parseInt(arm7PcValue, 16);
      const baseAddress = 0x00200010;
      const offset = arm7Pc - baseAddress;
      const newOffset = (offset + 4) % (8 * 4);
      const newPc = baseAddress + newOffset;
      setRegisterValue("dc.aica.arm7", "PC", `0x${newPc.toString(16).toUpperCase().padStart(8, "0")}`);
    }
  } else if (targetLower.includes("dsp")) {
    // Increment DSP step counter with wraparound (0..7)
    const dspStepValue = registerValues.get("dc.aica.dsp.step");
    if (dspStepValue && dspStepValue.startsWith("0x")) {
      const step = Number.parseInt(dspStepValue, 16);
      const newStep = (step + 1) % 8;
      setRegisterValue("dc.aica.dsp", "STEP", `0x${newStep.toString(16).toUpperCase().padStart(3, "0")}`);
    }
  }
};

const dispatchMethod = async (
  method: keyof DebuggerRpcSchema,
  params: Record<string, unknown>,
  client: ClientContext,
): Promise<{ result: unknown; shouldBroadcastTick: boolean }> => {
  switch (method) {
    case "debugger.describe":
      return {
        result: {
          emulator: { name: "mockServer", version: "unspecified", build: "native" as const },
          deviceTree: buildDeviceTree(),
          capabilities: ["watches", "step", "breakpoints", "frame-log"],
        } as DebuggerShape,
        shouldBroadcastTick: true, // Send initial state after describe
      };
    case "state.getMemorySlice": {
      const target = typeof params.target === "string" ? params.target : "sh4";
      const addressValue = Number(params.address);
      const lengthValue = Number(params.length);
      return {
        result: buildMemorySlice({
          target,
          address: Number.isFinite(addressValue) ? addressValue : undefined,
          length: Number.isFinite(lengthValue) && lengthValue > 0 ? lengthValue : undefined,
        }),
        shouldBroadcastTick: false,
      };
    }
    case "state.getDisassembly": {
      const target = typeof params.target === "string" ? params.target : "sh4";
      const address = typeof params.address === "number" ? params.address : 0;
      const count = typeof params.count === "number" ? params.count : 128;
      const lines = generateDisassembly(target, address, count);
      return {
        result: { lines },
        shouldBroadcastTick: false,
      };
    }
    case "state.getCallstack": {
      const target = (params.target as string) || "sh4";
      const max = Math.min(Number(params.maxFrames) || 16, 64);
      const frames = Array.from({ length: max }).map((_, index) => ({
        index,
        pc: 0x8c000000 + index * 4,
        sp: 0x0cfe0000 - index * 16,
        symbol: `${target.toUpperCase()}_func_${index}`,
        location: `${target}.c:${100 + index}`,
      }));
      return {
        result: { target, frames },
        shouldBroadcastTick: false,
      };
    }
    case "state.watch": {
      const expressions = (params.expressions as string[]) ?? [];
      expressions.forEach((expr) => {
        const id = nextWatchId++;
        client.watches.add(id.toString());
        serverWatches.set(id, { id, expression: expr });
      });
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "state.unwatch": {
      const watchIds = (params.watchIds as number[]) ?? [];
      watchIds.forEach((watchId) => {
        client.watches.delete(watchId.toString());
        serverWatches.delete(watchId);
      });
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "state.editWatch": {
      const { watchId, value } = params as { watchId: number; value: string };

      // Validate that the watch exists
      const watch = serverWatches.get(watchId);
      if (!watch) {
        return {
          result: {
            error: {
              code: -32602,
              message: `Watch "${watchId}" not found`,
            },
          } as RpcError,
          shouldBroadcastTick: false,
        };
      }

      // Try to parse and update the value
      try {
        // For now, just update the register value directly
        if (registerValues.has(watch.expression)) {
          registerValues.set(watch.expression, value);
        } else {
          // Return error for non-register watches
          return {
            result: {
              error: {
                code: -32602,
                message: `Cannot edit non-register expression "${watch.expression}"`,
              },
            } as RpcError,
            shouldBroadcastTick: false,
          };
        }

        return {
          result: {} as RpcError,
          shouldBroadcastTick: true,
        };
      } catch (error) {
        return {
          result: {
            error: {
              code: -32603,
              message: `Failed to set value: ${error instanceof Error ? error.message : "Unknown error"}`,
            },
          } as RpcError,
          shouldBroadcastTick: false,
        };
      }
    }
    case "state.modifyWatchExpression": {
      const { watchId, newExpression } = params as { watchId: number; newExpression: string };

      // Find the watch by ID
      const watch = serverWatches.get(watchId);
      if (!watch) {
        return {
          result: {
            error: {
              code: -32602,
              message: `Watch ${watchId} not found`,
            },
          } as RpcError,
          shouldBroadcastTick: false,
        };
      }

      // Update the expression while keeping the same ID
      serverWatches.set(watchId, { id: watchId, expression: newExpression });

      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "control.pause":
      isRunning = false;
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    case "control.step": {
      isRunning = false;
      const stepTarget = (params.target as string) ?? "sh4";
      incrementProgramCounter(stepTarget);
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "control.stepOver": {
      isRunning = false;
      const stepOverTarget = (params.target as string) ?? "sh4";
      incrementProgramCounter(stepOverTarget);
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "control.stepOut": {
      isRunning = false;
      const stepOutTarget = (params.target as string) ?? "sh4";
      incrementProgramCounter(stepOutTarget);
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "control.runUntil":
      isRunning = true;
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    case "breakpoints.add": {
      const event = params.event as string;
      const address = params.address as number | undefined;
      const kind = (params.kind as BreakpointDescriptor["kind"]) ?? "code";
      const enabled = params.enabled !== false;
      const id = nextBreakpointId++;
      const breakpoint: BreakpointDescriptor = {
        id,
        event,
        address,
        kind,
        enabled,
      };
      serverBreakpoints.set(id, breakpoint);
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "breakpoints.remove": {
      const id = params.id as number;
      const removed = serverBreakpoints.delete(id);
      if (!removed) {
        return {
          result: { error: { code: -32000, message: `Breakpoint ${id} not found` } } as RpcError,
          shouldBroadcastTick: false,
        };
      }
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "breakpoints.toggle": {
      const id = params.id as number;
      const enabled = params.enabled as boolean;
      const breakpoint = serverBreakpoints.get(id);
      if (!breakpoint) {
        return {
          result: { error: { code: -32000, message: `Breakpoint ${id} not found` } } as RpcError,
          shouldBroadcastTick: false,
        };
      }
      const updated = { ...breakpoint, enabled };
      serverBreakpoints.set(id, updated);
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "breakpoints.setCategoryStates": {
      const categories = params.categories as Record<string, { muted: boolean; soloed: boolean }>;
      for (const [category, state] of Object.entries(categories)) {
        const categoryKey = category as BreakpointCategory;
        if (categoryStates.has(categoryKey)) {
          categoryStates.set(categoryKey, { muted: state.muted, soloed: state.soloed });
        }
      }
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "state.getSgcFrameData": {
      // This method will trigger a binary response
      // The actual binary data will be sent separately after the JSON response
      return {
        result: {
          frameCount: NUM_FRAMES,
          channelsPerFrame: CHANNELS_PER_FRAME,
          bytesPerChannel: BYTES_PER_CHANNEL,
          totalBytes: TOTAL_BYTES
        },
        shouldBroadcastTick: false,
      };
    }
    default:
      throw new Error(`Unhandled JSON-RPC method: ${String(method)}`);
  }
};

const sha256Byte = (input: string): number => {
  const hash = createHash("sha256").update(input).digest();
  return hash[0];
};

const memoryProfiles: Record<string, { defaultBase: number; generator: (index: number, base: number) => number }> = {
  sh4: {
    defaultBase: 0x8c000000,
    generator: (index, base) => sha256Byte(`SH4:${(base + index).toString(16)}`),
  },
  arm7: {
    defaultBase: 0x00200000,
    generator: (index, base) => sha256Byte(`ARM7:${(base + index).toString(16)}`),
  },
  dsp: {
    defaultBase: 0x00000000,
    generator: (index, base) => sha256Byte(`DSP:${(base + index).toString(16)}`),
  },
};

const buildMemorySlice = ({
  target,
  address,
  length,
}: {
  target: string;
  address?: number;
  length?: number;
}): MemorySlice => {
  const profile = memoryProfiles[target] ?? memoryProfiles.sh4;
  const effectiveLength = length && length > 0 ? length : 64;
  const baseAddress = typeof address === "number" && address >= 0 ? address : profile.defaultBase;
  const bytes = Array.from({ length: effectiveLength }, (_, index) => profile.generator(index, baseAddress) & 0xff);
  return {
    baseAddress,
    data: bytes,
    validity: "ok",
  };
};

const collectRegistersFromTree = (tree: DeviceNodeDescriptor[]): Array<{ path: string; registers: RegisterValue[] }> => {
  const result: Array<{ path: string; registers: RegisterValue[] }> = [];
  for (const node of tree) {
    if (node.registers && node.registers.length > 0) {
      result.push({ path: node.path, registers: node.registers });
    }
    if (node.children) {
      result.push(...collectRegistersFromTree(node.children));
    }
  }
  return result;
};

const checkBreakpoint = (path: string, registerName: string, value: number): BreakpointDescriptor | undefined => {
  const event = `${path}.${registerName.toLowerCase()}`;
  for (const bp of serverBreakpoints.values()) {
    if (bp.event === event && bp.address === value && bp.enabled && bp.kind === "code" && isBreakpointActive(bp)) {
      return bp;
    }
  }
  return undefined;
};

// Emulation tick - advances the emulator state
const emulationTick = () => {
  let hitBreakpointId: number | undefined;

  // Only mutate registers and generate events when running
  if (isRunning) {
    // Mutate some register values to simulate execution
    // Loop around first 8 instructions for debugging
    const pcValue = registerValues.get("dc.sh4.cpu.pc");
    if (pcValue && pcValue.startsWith("0x")) {
      const pc = Number.parseInt(pcValue, 16);
      const baseAddress = 0x8C0000A0;
      const offset = pc - baseAddress;
      const newOffset = (offset + 2) % (8 * 2); // 8 instructions * 2 bytes each
      const newPc = baseAddress + newOffset;
      setRegisterValue("dc.sh4.cpu", "PC", `0x${newPc.toString(16).toUpperCase().padStart(8, "0")}`);

      // Check if we hit a breakpoint
      const hitBp = checkBreakpoint("dc.sh4.cpu", "PC", newPc);
      if (hitBp) {
        isRunning = false;
        hitBreakpointId = hitBp.id;
        const message = `SH4 breakpoint hit at 0x${newPc.toString(16).toUpperCase()}`;

        // Add to event log
        const logEntry: EventLogEntry = {
          eventId: (nextEventId++).toString(),
          timestamp: Date.now(),
          subsystem: "sh4",
          severity: "info",
          message,
          metadata: { breakpointId: hitBp.id, address: newPc },
        };
        eventLogEntries.push(logEntry);
        if (eventLogEntries.length > EVENT_LOG_LIMIT) {
          eventLogEntries.splice(0, eventLogEntries.length - EVENT_LOG_LIMIT);
        }
      }
    }

    const prValue = registerValues.get("dc.sh4.cpu.pr");
    if (prValue && prValue.startsWith("0x")) {
      const pr = Number.parseInt(prValue, 16);
      setRegisterValue("dc.sh4.cpu", "PR", `0x${(pr + 2).toString(16).toUpperCase().padStart(8, "0")}`);
    }

    // Mutate ARM7 PC value - loop around first 8 instructions
    const arm7PcValue = registerValues.get("dc.aica.arm7.pc");
    if (arm7PcValue && arm7PcValue.startsWith("0x")) {
      const arm7Pc = Number.parseInt(arm7PcValue, 16);
      const baseAddress = 0x00200010;
      const offset = arm7Pc - baseAddress;
      const newOffset = (offset + 4) % (8 * 4); // 8 instructions * 4 bytes each
      const newPc = baseAddress + newOffset;
      setRegisterValue("dc.aica.arm7", "PC", `0x${newPc.toString(16).toUpperCase().padStart(8, "0")}`);

      // Check if we hit a breakpoint
      const hitBp = checkBreakpoint("dc.aica.arm7", "pc", newPc);
      if (hitBp && !hitBreakpointId) {
        isRunning = false;
        hitBreakpointId = hitBp.id;
        const message = `ARM7 breakpoint hit at 0x${newPc.toString(16).toUpperCase()}`;

        // Add to event log
        const logEntry: EventLogEntry = {
          eventId: (nextEventId++).toString(),
          timestamp: Date.now(),
          subsystem: "aica",
          severity: "info",
          message,
          metadata: { breakpointId: hitBp.id, address: newPc },
        };
        eventLogEntries.push(logEntry);
        if (eventLogEntries.length > EVENT_LOG_LIMIT) {
          eventLogEntries.splice(0, eventLogEntries.length - EVENT_LOG_LIMIT);
        }
      }
    }

    // Mutate DSP step value - loop around first 8 steps (0..7)
    const dspStepValue = registerValues.get("dc.aica.dsp.step");
    if (dspStepValue && dspStepValue.startsWith("0x")) {
      const step = Number.parseInt(dspStepValue, 16);
      const newStep = (step + 1) % 8; // Loop 0..7
      setRegisterValue("dc.aica.dsp", "STEP", `0x${newStep.toString(16).toUpperCase().padStart(3, "0")}`);

      // Check if we hit a breakpoint (using lowercase "step" for compatibility)
      const hitBp = checkBreakpoint("dc.aica.dsp", "step", newStep);
      if (hitBp && !hitBreakpointId) {
        isRunning = false;
        hitBreakpointId = hitBp.id;
        const message = `DSP breakpoint hit at step ${newStep}`;

        // Add to event log
        const logEntry: EventLogEntry = {
          eventId: (nextEventId++).toString(),
          timestamp: Date.now(),
          subsystem: "dsp",
          severity: "info",
          message,
          metadata: { breakpointId: hitBp.id, step: newStep },
        };
        eventLogEntries.push(logEntry);
        if (eventLogEntries.length > EVENT_LOG_LIMIT) {
          eventLogEntries.splice(0, eventLogEntries.length - EVENT_LOG_LIMIT);
        }
      }
    }

    // Mutate some AICA values
    const ch0Vol = registerValues.get("dc.aica.channels.ch0_vol");
    if (ch0Vol && ch0Vol.startsWith("0x")) {
      const vol = Number.parseInt(ch0Vol, 16);
      setRegisterValue("dc.aica.channels", "CH0_VOL", `0x${((vol + 1) & 0xFF).toString(16).toUpperCase().padStart(2, "0")}`);
    }

    // Generate event log event only when running
    const event = createFrameEvent();
    eventLogEntries.push(event);
    if (eventLogEntries.length > EVENT_LOG_LIMIT) {
      eventLogEntries.splice(0, eventLogEntries.length - EVENT_LOG_LIMIT);
    }
  }

  return hitBreakpointId;
};

// Build and broadcast tick to all connected clients
const broadcastTick = (hitBreakpointId?: number) => {
  // Build complete tick with all state
  const deviceTree = buildDeviceTree();
  const allRegisters = collectRegistersFromTree(deviceTree);

  // Convert registers array to Record<string, RegisterValue[]>
  const registersById: Record<string, RegisterValue[]> = {};
  for (const { path, registers } of allRegisters) {
    registersById[path] = registers;
  }

  // Convert breakpoints map to Record<string, BreakpointDescriptor>
  const breakpointsById: Record<string, BreakpointDescriptor> = {};
  for (const [id, bp] of serverBreakpoints.entries()) {
    breakpointsById[id] = bp;
  }

  // Build watches array with WatchDescriptor objects
  const watches: import("../src/lib/debuggerSchema").WatchDescriptor[] = [];
  for (const [watchId, watchInfo] of serverWatches.entries()) {
    watches.push({
      id: watchId,
      expression: watchInfo.expression,
      value: registerValues.get(watchInfo.expression) ?? "0x00000000",
    });
  }

  // Build callstacks for all targets
  const callstacks: Record<string, import("../src/lib/debuggerSchema").CallstackFrame[]> = {};

  // SH4 callstack - first frame is current PC
  const sh4PcValue = registerValues.get("dc.sh4.cpu.pc");
  const sh4Pc = sh4PcValue && sh4PcValue.startsWith("0x") ? Number.parseInt(sh4PcValue, 16) : 0x8c0000a0;
  const sh4Frames = Array.from({ length: 16 }).map((_, index) => ({
    index,
    pc: index === 0 ? sh4Pc : 0x8c000000 + (index - 1) * 4,
    sp: 0x0cfe0000 - index * 16,
    symbol: `SH4_func_${index}`,
    location: `sh4.c:${100 + index}`,
  }));
  callstacks["sh4"] = sh4Frames;

  // ARM7 callstack - first frame is current PC
  const arm7PcValue = registerValues.get("dc.aica.arm7.pc");
  const arm7Pc = arm7PcValue && arm7PcValue.startsWith("0x") ? Number.parseInt(arm7PcValue, 16) : 0x00200010;
  const arm7Frames = Array.from({ length: 16 }).map((_, index) => ({
    index,
    pc: index === 0 ? arm7Pc : 0x00200000 + (index - 1) * 4,
    sp: 0x00280000 - index * 16,
    symbol: `ARM7_func_${index}`,
    location: `arm7.c:${100 + index}`,
  }));
  callstacks["arm7"] = arm7Frames;

  const tick: DebuggerTick = {
    tickId: tickId++,
    timestamp: Date.now(),
    executionState: {
      state: isRunning ? "running" : "paused",
      breakpointId: hitBreakpointId,
    },
    registers: registersById,
    breakpoints: breakpointsById,
    eventLog: eventLogEntries.slice(),
    watches: serverWatches.size > 0 ? watches : undefined,
    callstacks,
  };

  // Validate tick before broadcasting
  try {
    DebuggerTickSchema.parse(tick);
  } catch (error) {
    if (error instanceof z.ZodError) {
      console.error("Invalid tick data:", error.message);
      return; // don't do the broadcast
    }
  }

  // Broadcast tick to all clients
  for (const client of clients) {
    sendNotification(client, {
      topic: "tick",
      payload: tick,
    });
  }
};

// Generate mock SGC frame data
const generateSgcFrameData = (): Buffer => {
  console.log(`Generating ${NUM_FRAMES} frames of SGC data (${(TOTAL_BYTES / 1024 / 1024).toFixed(2)} MB)...`);

  const buffer = Buffer.alloc(TOTAL_BYTES);

  // Helper to write bits into a 32-bit word (little-endian)
  const writeBits = (offset: number, value: number) => {
    buffer.writeUInt32LE(value, offset);
  };

  // Simple seeded random for deterministic data
  const seededRandom = (seed: number, min: number, max: number) => {
    const x = Math.sin(seed) * 10000;
    const normalized = x - Math.floor(x);
    return Math.floor(normalized * (max - min + 1)) + min;
  };

  for (let frame = 0; frame < NUM_FRAMES; frame++) {
    const frameOffset = frame * BYTES_PER_FRAME;

    for (let channel = 0; channel < CHANNELS_PER_FRAME; channel++) {
      const channelOffset = frameOffset + (channel * BYTES_PER_CHANNEL);
      const seed = frame * 1000 + channel;
      const channelSeed = channel; // For values that should be constant per channel

      // ChannelCommonData struct (72 bytes of actual data, rest is padding)

      // +00 [0] - SA_hi, PCMS, LPCTL, SSCTL, KYONB, KYONEX
      const word0 =
        (seededRandom(seed + 0, 0, 0x7F) << 0) |      // SA_hi:7
        (seededRandom(channelSeed + 1, 0, 3) << 7) |  // PCMS:2 (constant per channel)
        (seededRandom(seed + 2, 0, 1) << 9) |         // LPCTL:1
        (seededRandom(seed + 3, 0, 1) << 10) |        // SSCTL:1
        (seededRandom(seed + 4, 0, 1) << 14) |        // KYONB:1
        (seededRandom(seed + 5, 0, 1) << 15);         // KYONEX:1
      writeBits(channelOffset + 0, word0);

      // +04 [1] - SA_low
      const word1 = seededRandom(seed + 6, 0, 0xFFFF);
      writeBits(channelOffset + 4, word1);

      // +08 [2] - LSA (Loop Start Address) - constant per channel
      const word2 = seededRandom(channelSeed + 7, 0, 0xFFFF);
      writeBits(channelOffset + 8, word2);

      // +0C [3] - LEA (Loop End Address) - constant per channel
      const word3 = seededRandom(channelSeed + 8, 0, 0xFFFF);
      writeBits(channelOffset + 12, word3);

      // +10 [4] - AR, D1R, D2R
      const word4 =
        (seededRandom(seed + 9, 0, 0x1F) << 0) |      // AR:5
        (seededRandom(seed + 10, 0, 0x1F) << 6) |     // D1R:5
        (seededRandom(seed + 11, 0, 0x1F) << 11);     // D2R:5
      writeBits(channelOffset + 16, word4);

      // +14 [5] - RR, DL, KRS, LPSLNK
      const word5 =
        (seededRandom(seed + 12, 0, 0x1F) << 0) |     // RR:5
        (seededRandom(seed + 13, 0, 0x1F) << 5) |     // DL:5
        (seededRandom(seed + 14, 0, 0xF) << 10) |     // KRS:4
        (seededRandom(seed + 15, 0, 1) << 14);        // LPSLNK:1
      writeBits(channelOffset + 20, word5);

      // +18 [6] - FNS, OCT - constant per channel
      const word6 =
        (seededRandom(channelSeed + 16, 0, 0x3FF) << 0) |    // FNS:10 (constant per channel)
        (seededRandom(channelSeed + 17, 0, 0xF) << 11);      // OCT:4 (constant per channel)
      writeBits(channelOffset + 24, word6);

      // +1C [7] - ALFOS, ALFOWS, PLFOS, PLFOWS, LFOF, LFORE
      const word7 =
        (seededRandom(seed + 18, 0, 7) << 0) |        // ALFOS:3
        (seededRandom(seed + 19, 0, 3) << 3) |        // ALFOWS:2
        (seededRandom(seed + 20, 0, 7) << 5) |        // PLFOS:3
        (seededRandom(seed + 21, 0, 3) << 8) |        // PLFOWS:2
        (seededRandom(seed + 22, 0, 0x1F) << 10) |    // LFOF:5
        (seededRandom(seed + 23, 0, 1) << 15);        // LFORE:1
      writeBits(channelOffset + 28, word7);

      // +20 [8] - ISEL, IMXL
      const word8 =
        (seededRandom(seed + 24, 0, 0xF) << 0) |      // ISEL:4
        (seededRandom(seed + 25, 0, 0xF) << 4);       // IMXL:4
      writeBits(channelOffset + 32, word8);

      // +24 [9] - DIPAN, DISDL
      const word9 =
        (seededRandom(seed + 26, 0, 0x1F) << 0) |     // DIPAN:5
        (seededRandom(seed + 27, 0, 0xF) << 8);       // DISDL:4
      writeBits(channelOffset + 36, word9);

      // +28 [10] - Q, TL
      const word10 =
        (seededRandom(seed + 28, 0, 0x1F) << 0) |     // Q:5
        (seededRandom(seed + 29, 0, 0xFF) << 8);      // TL:8
      writeBits(channelOffset + 40, word10);

      // +2C [11] - FLV0
      const word11 = seededRandom(seed + 30, 0, 0x1FFF);
      writeBits(channelOffset + 44, word11);

      // +30 [12] - FLV1
      const word12 = seededRandom(seed + 31, 0, 0x1FFF);
      writeBits(channelOffset + 48, word12);

      // +34 [13] - FLV2
      const word13 = seededRandom(seed + 32, 0, 0x1FFF);
      writeBits(channelOffset + 52, word13);

      // +38 [14] - FLV3
      const word14 = seededRandom(seed + 33, 0, 0x1FFF);
      writeBits(channelOffset + 56, word14);

      // +3C [15] - FLV4
      const word15 = seededRandom(seed + 34, 0, 0x1FFF);
      writeBits(channelOffset + 60, word15);

      // +40 [16] - FD1R, FAR
      const word16 =
        (seededRandom(seed + 35, 0, 0x1F) << 0) |     // FD1R:5
        (seededRandom(seed + 36, 0, 0x1F) << 8);      // FAR:5
      writeBits(channelOffset + 64, word16);

      // +44 [17] - FRR, FD2R
      const word17 =
        (seededRandom(seed + 37, 0, 0x1F) << 0) |     // FRR:5
        (seededRandom(seed + 38, 0, 0x1F) << 8);      // FD2R:5
      writeBits(channelOffset + 68, word17);

      // Sample data (72-89: 9 x int16)
      // Generate realistic audio sample values (-32768 to 32767)
      const sampleSeed = seed + frame;

      // Current sample (simulated waveform)
      const sample_current = Math.floor(Math.sin(sampleSeed * 0.1) * 16000);
      buffer.writeInt16LE(sample_current, channelOffset + 72);

      // Previous sample (slightly phase-shifted)
      const sample_previous = Math.floor(Math.sin((sampleSeed - 1) * 0.1) * 16000);
      buffer.writeInt16LE(sample_previous, channelOffset + 74);

      // Filtered sample (softer/smoother)
      const sample_filtered = Math.floor((sample_current * 0.7 + sample_previous * 0.3));
      buffer.writeInt16LE(sample_filtered, channelOffset + 76);

      // Generate ADSR envelope for AEG (0 to 0x3FF)
      const aegPhase = (frame / NUM_FRAMES) * Math.PI * 2; // Full cycle over all frames
      const aegEnvelope = Math.max(0, Math.sin(aegPhase)); // 0.0 to 1.0
      const aeg_value = Math.floor(aegEnvelope * 0x3FF);

      // Sample after AEG (apply amplitude envelope)
      const sample_post_aeg = Math.floor(sample_filtered * aegEnvelope);
      buffer.writeInt16LE(sample_post_aeg, channelOffset + 78);

      // Generate ADSR envelope for FEG (0 to 0x1FF8)
      const fegPhase = (frame / NUM_FRAMES) * Math.PI * 4 + channel * 0.1; // Faster cycle, per-channel offset
      const fegEnvelope = Math.max(0, Math.cos(fegPhase)); // 0.0 to 1.0, different shape
      const feg_value = Math.floor(fegEnvelope * 0x1FF8);

      // Sample after FEG (apply filter envelope - affects brightness)
      const filterAmount = fegEnvelope * 0.5 + 0.5; // 0.5 to 1.0
      const sample_post_feg = Math.floor(sample_post_aeg * filterAmount);
      buffer.writeInt16LE(sample_post_feg, channelOffset + 80);

      // Sample after TL (apply total level attenuation)
      const tlAttenuation = (255 - seededRandom(seed + 29, 0, 255)) / 255; // 0.0 to 1.0
      const sample_post_tl = Math.floor(sample_post_feg * tlAttenuation);
      buffer.writeInt16LE(sample_post_tl, channelOffset + 82);

      // Pan to left/right (simple equal-power pan)
      const panPosition = seededRandom(seed + 26, 0, 31) / 31; // 0.0 (left) to 1.0 (right)
      const panLeft = Math.cos(panPosition * Math.PI / 2);
      const panRight = Math.sin(panPosition * Math.PI / 2);
      const sample_left = Math.floor(sample_post_tl * panLeft);
      const sample_right = Math.floor(sample_post_tl * panRight);
      buffer.writeInt16LE(sample_left, channelOffset + 84);
      buffer.writeInt16LE(sample_right, channelOffset + 86);

      // DSP send (reduced level)
      const dspSendLevel = seededRandom(seed + 27, 0, 15) / 15; // 0.0 to 1.0
      const sample_dsp = Math.floor(sample_post_tl * dspSendLevel * 0.3);
      buffer.writeInt16LE(sample_dsp, channelOffset + 88);

      // Additional state (90-106)
      // CA fraction (10 bits in 16-bit word) - set to 0
      const ca_fraction = 0;
      buffer.writeUInt16LE(ca_fraction, channelOffset + 90);

      // CA step (32-bit) - use unsigned bitwise OR
      const ca_step = (seededRandom(seed + 40, 0, 0xFFFF) | (seededRandom(seed + 41, 0, 0xFFFF) << 16)) >>> 0;
      buffer.writeUInt32LE(ca_step, channelOffset + 92);

      // AEG value (32-bit) - computed above
      buffer.writeUInt32LE(aeg_value, channelOffset + 96);

      // FEG value (32-bit) - computed above
      buffer.writeUInt32LE(feg_value, channelOffset + 100);

      // LFO value (8-bit)
      const lfo_value = Math.floor((Math.sin(frame * 0.05) * 0.5 + 0.5) * 255);
      buffer.writeUInt8(lfo_value, channelOffset + 104);

      // Amplitude LFO value (8-bit)
      const alfo_value = Math.floor((Math.sin(frame * 0.07 + channel) * 0.5 + 0.5) * 255);
      buffer.writeUInt8(alfo_value, channelOffset + 105);

      // Pitch LFO value (8-bit)
      const plfo_value = Math.floor((Math.sin(frame * 0.03 + channel * 0.5) * 0.5 + 0.5) * 255);
      buffer.writeUInt8(plfo_value, channelOffset + 106);

      // CA current (16-bit) - set to frame index to represent sample position
      const ca_current = frame & 0xFFFF;
      buffer.writeUInt16LE(ca_current, channelOffset + 107);

      // Rest of the 128 bytes is zero-initialized (padding for future state)
    }
  }

  console.log('SGC frame data generation complete');
  return buffer;
};

const start = async () => {
  // Generate mock SGC frame data on startup
  sgcFrameData = generateSgcFrameData();

  const app = express();
  let vite: Awaited<ReturnType<typeof createViteServer>> | null = null;

  app.get("/health", (_req, res) => {
    res.json({ status: "ok" });
  });

  if (IS_PRODUCTION) {
    const distDir = resolve(process.cwd(), "dist");
    app.use(express.static(distDir));
    app.use(async (req, res, next) => {
      if (req.method !== "GET") {
        return next();
      }
      try {
        const template = await readFile(resolve(distDir, "index.html"), "utf8");
        res.status(200).set({ "Content-Type": "text/html" }).end(template);
      } catch (error) {
        next(error);
      }
    });
  } else {
    vite = await createViteServer({
      server: {
        middlewareMode: true,
      },
      appType: "custom",
    });

    app.use(vite.middlewares);

    app.use(async (req, res, next) => {
      try {
        const template = await readFile(resolve(process.cwd(), "index.html"), "utf8");
        const transformed = await vite!.transformIndexHtml(req.originalUrl, template);
        res.status(200).set({ "Content-Type": "text/html" }).end(transformed);
      } catch (error) {
        vite!.ssrFixStacktrace(error as Error);
        next(error);
      }
    });
  }

  const server = createHttpServer(app);
  const wss = new WebSocketServer({ server, path: WS_PATH });

  wss.on("connection", (socket) => {
    const context: ClientContext = {
      socket,
      sessionId: randomUUID(),
      watches: new Set(),
    };

    clients.add(context);

    socket.on("message", (data) => {
      let message: JsonRpcMessage;
      try {
        message = JSON.parse(data.toString());
      } catch (error) {
        console.warn("Invalid JSON payload", error);
        return;
      }

      if ("method" in message && "id" in message) {
        void handleRequest(context, message as JsonRpcRequest);
      }
    });

    socket.on("close", () => {
      clients.delete(context);
    });
  });

  server.listen(PORT, () => {
    console.log(`Mock debugger server running at http://localhost:${PORT}`);
    console.log(`WebSocket endpoint available at ws://localhost:${PORT}${WS_PATH}`);
  });

  // Run emulator loop at ~60fps, but only mutate state when isRunning
  const timer = setInterval(() => {
    const hitBreakpointId = emulationTick();
    // Only broadcast when a breakpoint was hit (which pauses execution)
    if (hitBreakpointId) {
      broadcastTick(hitBreakpointId);
    }
  }, 16);

  const shutdown = async () => {
    clearInterval(timer);
      for (const client of clients) {
      client.socket.close(1001, "Server shutting down");
    }
    if (vite) {
      await vite.close();
    }
    server.close(() => process.exit(0));
  };

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);
};

void start();










