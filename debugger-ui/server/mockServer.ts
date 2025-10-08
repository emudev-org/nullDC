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
} from "../src/lib/debuggerSchema";

const PORT = Number(process.env.PORT ?? 5173);
const WS_PATH = process.env.DEBUGGER_WS_PATH ?? "/ws";
const IS_PRODUCTION = process.env.NODE_ENV === "production";

interface ClientContext {
  socket: WebSocket;
  sessionId: string;
  watches: Set<string>;
}

interface ServerWatch {
  id: string;
  expression: string;
}

const serverWatches = new Map<string, ServerWatch>(); // id -> {id, expression}
const serverBreakpoints = new Map<string, BreakpointDescriptor>();
let isRunning = true; // Execution state
let nextEventId = 1n; // Event ID counter (BigInt for 64-bit)
let tickId = 0; // Tick counter

// Initialize default watches
const DEFAULT_WATCH_EXPRESSIONS = ["dc.sh4.cpu.pc", "dc.sh4.dmac.dmaor"];
for (const expr of DEFAULT_WATCH_EXPRESSIONS) {
  const id = randomUUID();
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
  const lower = bp.location.toLowerCase();
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
    kind: "beloved console",
    description: "Sega Dreamcast system bus",
    registers: [
      { name: "SYSCLK", value: getRegisterValue("dc", "SYSCLK"), width: 0 },
      { name: "ASIC_REV", value: getRegisterValue("dc", "ASIC_REV"), width: 16 },
    ],
    children: [
      {
        path: "dc.sh4",
        label: "SH4",
        kind: "processor",
        description: "Hitachi SH-4 main CPU",
        registers: [
          { name: "VBR", value: getRegisterValue("dc.sh4", "VBR"), width: 32 },
          { name: "SR", value: getRegisterValue("dc.sh4", "SR"), width: 32 },
          { name: "FPSCR", value: getRegisterValue("dc.sh4", "FPSCR"), width: 32 },
        ],
        events: [
          "dc.sh4.interrupt",
          "dc.sh4.exception",
          "dc.sh4.tlb_miss",
        ],
        children: [
          {
            path: "dc.sh4.cpu",
            label: "Core",
            kind: "processor",
            description: "Integer pipeline",
            registers: [
              { name: "PC", value: getRegisterValue("dc.sh4.cpu", "PC"), width: 32 },
              { name: "PR", value: getRegisterValue("dc.sh4.cpu", "PR"), width: 32 },
            ],
          },
          {
            path: "dc.sh4.icache",
            label: "I-Cache",
            kind: "peripheral",
            description: "Instruction cache",
            registers: [
              { name: "ICRAM", value: "16KB", width: 0 },
              { name: "ICACHE_CTRL", value: getRegisterValue("dc.sh4.icache", "ICACHE_CTRL"), width: 32 },
            ],
          },
          {
            path: "dc.sh4.dcache",
            label: "D-Cache",
            kind: "peripheral",
            description: "Data cache",
            registers: [
              { name: "DCRAM", value: "8KB", width: 0 },
              { name: "DCACHE_CTRL", value: getRegisterValue("dc.sh4.dcache", "DCACHE_CTRL"), width: 32 },
            ],
          },
          {
            path: "dc.sh4.tlb",
            label: "TLB",
            kind: "peripheral",
            description: "Translation lookaside buffer",
            registers: [
              { name: "UTLB_ENTRIES", value: "64", width: 0 },
              { name: "ITLB_ENTRIES", value: "4", width: 0 },
            ],
          },
        ],
      },
      {
        path: "dc.holly",
        label: "Holly",
        kind: "peripheral",
        description: "System ASIC",
        registers: [
          { name: "HOLLY_ID", value: getRegisterValue("dc.holly", "HOLLY_ID"), width: 32 },
          { name: "DMAC_CTRL", value: getRegisterValue("dc.holly", "DMAC_CTRL"), width: 32 },
        ],
        children: [
          {
            path: "dc.holly.dmac",
            label: "DMA Controller",
            kind: "peripheral",
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
            kind: "pipeline",
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
            kind: "pipeline",
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
        kind: "coprocessor",
        description: "Sound processor",
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
            kind: "processor",
            description: "ARM7TDMI sound CPU",
            registers: [
              { name: "PC", value: getRegisterValue("dc.aica.arm7", "PC"), width: 32 },
            ],
          },
          {
            path: "dc.aica.channels",
            label: "Channels",
            kind: "channel",
            registers: [
              { name: "CH0_VOL", value: getRegisterValue("dc.aica.channels", "CH0_VOL"), width: 8 },
              { name: "CH1_VOL", value: getRegisterValue("dc.aica.channels", "CH1_VOL"), width: 8 },
            ],
            events: [
              "dc.aica.channels.key_on",
              "dc.aica.channels.key_off",
              "dc.aica.channels.loop",
            ],
          },
          {
            path: "dc.aica.dsp",
            label: "DSP",
            kind: "coprocessor",
            registers: [
              { name: "STEP", value: getRegisterValue("dc.aica.dsp", "STEP"), width: 16 },
              { name: "DSP_ACC", value: getRegisterValue("dc.aica.dsp", "DSP_ACC"), width: 16 },
            ],
            events: [
              "dc.aica.dsp.step",
            ],
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
        mnemonic: programLine,
        operands: "",
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

    const byteValues: number[] = [];
    for (let b = 0; b < instr.bytes; b++) {
      byteValues.push(sha256Byte(`${target}:${currentAddr.toString(16)}:${b}`));
    }
    const bytes = byteValues.map((b) => b.toString(16).toUpperCase().padStart(2, "0")).join(" " );

    lines.push({
      address: currentAddr,
      bytes,
      mnemonic: instr.mnemonic,
      operands,
      isCurrent: false,
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
    case "debugger.handshake":
      return {
        result: {
          sessionId: client.sessionId,
          capabilities: ["watches", "step", "breakpoints", "frame-log"],
        },
        shouldBroadcastTick: false,
      };
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
      const encoding = params.encoding as MemorySlice["encoding"] | undefined;
      const wordSize = params.wordSize as MemorySlice["wordSize"] | undefined;
      return {
        result: buildMemorySlice({
          target,
          address: Number.isFinite(addressValue) ? addressValue : undefined,
          length: Number.isFinite(lengthValue) && lengthValue > 0 ? lengthValue : undefined,
          encoding,
          wordSize,
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
        const id = randomUUID();
        client.watches.add(id);
        serverWatches.set(id, { id, expression: expr });
      });
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "state.unwatch": {
      const expressions = (params.expressions as string[]) ?? [];
      // expressions here are actually watch IDs
      expressions.forEach((watchId) => {
        client.watches.delete(watchId);
        serverWatches.delete(watchId);
      });
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "state.editWatch": {
      const { watchId, value } = params as { watchId: string; value: string };

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
      const { watchId, newExpression } = params as { watchId: string; newExpression: string };

      // Find the watch by ID
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
      const location = params.location as string;
      const kind = (params.kind as BreakpointDescriptor["kind"]) ?? "code";
      const enabled = params.enabled !== false;
      const id = `bp-${randomUUID().slice(0, 8)}`;
      const breakpoint: BreakpointDescriptor = {
        id,
        location,
        kind,
        enabled,
        hitCount: 0,
      };
      serverBreakpoints.set(id, breakpoint);
      return {
        result: {} as RpcError,
        shouldBroadcastTick: true,
      };
    }
    case "breakpoints.remove": {
      const id = params.id as string;
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
      const id = params.id as string;
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
    default:
      throw new Error(`Unhandled JSON-RPC method: ${String(method)}`);
  }
};

const sha256Byte = (input: string): number => {
  const hash = createHash("sha256").update(input).digest();
  return hash[0];
};

const memoryProfiles: Record<string, { defaultBase: number; wordSize: MemorySlice["wordSize"]; generator: (index: number, base: number) => number }> = {
  sh4: {
    defaultBase: 0x8c000000,
    wordSize: 4,
    generator: (index, base) => sha256Byte(`SH4:${(base + index).toString(16)}`),
  },
  arm7: {
    defaultBase: 0x00200000,
    wordSize: 4,
    generator: (index, base) => sha256Byte(`ARM7:${(base + index).toString(16)}`),
  },
  dsp: {
    defaultBase: 0x00000000,
    wordSize: 2,
    generator: (index, base) => sha256Byte(`DSP:${(base + index).toString(16)}`),
  },
};

const buildMemorySlice = ({
  target,
  address,
  length,
  encoding,
  wordSize,
}: {
  target: string;
  address?: number;
  length?: number;
  encoding?: MemorySlice["encoding"];
  wordSize?: MemorySlice["wordSize"];
}): MemorySlice => {
  const profile = memoryProfiles[target] ?? memoryProfiles.sh4;
  const effectiveLength = length && length > 0 ? length : 64;
  const baseAddress = typeof address === "number" && address >= 0 ? address : profile.defaultBase;
  const effectiveWordSize = wordSize ?? profile.wordSize;
  const effectiveEncoding = encoding ?? "hex";
  const bytes = Array.from({ length: effectiveLength }, (_, index) => profile.generator(index, baseAddress) & 0xff);
  return {
    baseAddress,
    wordSize: effectiveWordSize,
    encoding: effectiveEncoding,
    data: Buffer.from(bytes).toString("hex"),
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
  const location = `${path}.${registerName.toLowerCase()} == 0x${value.toString(16).toUpperCase().padStart(8, "0")}`;
  for (const bp of serverBreakpoints.values()) {
    if (bp.location === location && bp.enabled && bp.kind === "code" && isBreakpointActive(bp)) {
      return bp;
    }
  }
  return undefined;
};

// Emulation tick - advances the emulator state
const emulationTick = () => {
  let hitBreakpointId: string | undefined;

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
        hitBp.hitCount++;
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
        hitBp.hitCount++;
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
        hitBp.hitCount++;
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
const broadcastTick = (hitBreakpointId?: string) => {
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

const start = async () => {
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










