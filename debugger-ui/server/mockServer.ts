import express from "express";
import { createServer as createHttpServer } from "node:http";
import { WebSocketServer, type WebSocket } from "ws";
import { randomUUID, createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createServer as createViteServer } from "vite";
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
  DeviceNodeDescriptor,
  DisassemblyLine,
  FrameLogEntry,
  MemorySlice,
  RegisterValue,
  ThreadInfo,
  WaveformChunk,
} from "../src/lib/debuggerSchema";

const PORT = Number(process.env.PORT ?? 5173);
const WS_PATH = process.env.DEBUGGER_WS_PATH ?? "/ws";
const IS_PRODUCTION = process.env.NODE_ENV === "production";

interface ClientContext {
  socket: WebSocket;
  sessionId: string;
  topics: Set<string>;
  watches: Set<string>;
}

const serverWatches = new Set<string>();
const serverBreakpoints = new Map<string, BreakpointDescriptor>();

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
  ["dc.aica.channels.ch0_vol", "0x7F"],
  ["dc.aica.channels.ch1_vol", "0x6A"],
  ["dc.aica.dsp.dsp_pc", "0x020"],
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
    kind: "bus",
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
              { name: "DSP_PC", value: getRegisterValue("dc.aica.dsp", "DSP_PC"), width: 16 },
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

const baseRegisters: RegisterValue[] = [
  { name: "PC", value: "0x8C0000A0", width: 32 },
  { name: "R0", value: "0x00000000", width: 32 },
  { name: "R1", value: "0x00000001", width: 32 },
  { name: "R2", value: "0x8C001000", width: 32 },
  { name: "PR", value: "0x8C0000A2", width: 32 },
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

const sampleBreakpoints: BreakpointDescriptor[] = [
  { id: "bp-1", location: "dc.sh4.cpu.pc == 0x8C0000A0", kind: "code", enabled: true, hitCount: 3 },
  { id: "bp-2", location: "dc.aica.channel[0].step", kind: "event", enabled: false, hitCount: 0 },
];

const sampleThreads: ThreadInfo[] = [
  {
    id: "thread-main",
    name: "Main Thread",
    state: "running",
    core: "SH4",
    priority: 0,
    backtrace: [
      { index: 0, pc: 0x8c0000a0, symbol: "_start", location: "crt0.S:42" },
      { index: 1, pc: 0x8c001234, symbol: "kernel_main", location: "kernel.c:120" },
      { index: 2, pc: 0x8c0100ff, symbol: "game_loop", location: "game.c:240" },
    ],
  },
  {
    id: "thread-audio",
    name: "AICA Worker",
    state: "blocked",
    core: "AICA",
    priority: 3,
    backtrace: [
      { index: 0, pc: 0x7f000020, symbol: "aica_wait", location: "aica.c:88" },
      { index: 1, pc: 0x7f000120, symbol: "aica_mix", location: "aica.c:132" },
    ],
  },
];

const FRAME_LOG_LIMIT = 256;

const frameEventGenerators: Array<() => Omit<FrameLogEntry, "timestamp">> = [
  () => ({ subsystem: "ta", severity: "info", message: `TA/END_LIST tile ${Math.floor(Math.random() * 32)}` }),
  () => ({ subsystem: "core", severity: "info", message: "CORE/START_RENDER" }),
  () => ({ subsystem: "core", severity: "trace", message: `CORE/QUEUE_SUBMISSION ${Math.floor(Math.random() * 4)}` }),
  () => ({ subsystem: "dsp", severity: "trace", message: "DSP/STEP pipeline advanced" }),
  () => ({ subsystem: "aica", severity: "info", message: "AICA/SGC/STEP channel 0" }),
  () => ({ subsystem: "sh4", severity: "warn", message: "SH4/INTERRUPT IRQ5 asserted" }),
  () => ({ subsystem: "holly", severity: "info", message: "HOLLY/START_RENDER pass" }),
];

const createFrameEvent = (): FrameLogEntry => {
  const generator = frameEventGenerators[Math.floor(Math.random() * frameEventGenerators.length)];
  const event = generator();
  return {
    timestamp: Date.now(),
    ...event,
  };
};

const frameLogEntries: FrameLogEntry[] = Array.from({ length: 6 }, () => createFrameEvent());

const clients = new Set<ClientContext>();

const sendNotification = (client: ClientContext, notification: DebuggerNotification) => {
  const method = mapTopicToMethod(notification.topic);
  if (!method) {
    return;
  }

  const payload = JSON.stringify({
    jsonrpc: JSON_RPC_VERSION,
    method,
    params: notification.payload,
  });
  client.socket.send(payload);
};

const mapTopicToMethod = (topic: DebuggerNotification["topic"]): string | undefined => {
  switch (topic) {
    case "state.registers":
      return "event.state.registers";
    case "state.watch":
      return "event.state.watch";
    case "state.breakpoint":
      return "event.state.breakpoint";
    case "state.thread":
      return "event.state.thread";
    case "stream.waveform":
      return "event.stream.waveform";
    case "stream.frameLog":
      return "event.stream.frameLog";
    default:
      return undefined;
  }
};

const handleRequest = async (client: ClientContext, message: JsonRpcRequest) => {
  try {
    const params = (message.params ?? {}) as Record<string, unknown>;
    const result = await dispatchMethod(message.method as keyof DebuggerRpcSchema, params, client);
    respondSuccess(client.socket, message.id, result);
  } catch (error) {
    respondError(client.socket, message.id, error);
  }
};

const respondSuccess = (socket: WebSocket, id: JsonRpcSuccess["id"], result: unknown) => {
  const payload: JsonRpcSuccess = { jsonrpc: JSON_RPC_VERSION, id, result };
  socket.send(JSON.stringify(payload));
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

const dispatchMethod = async (
  method: keyof DebuggerRpcSchema,
  params: Record<string, unknown>,
  client: ClientContext,
): Promise<unknown> => {
  switch (method) {
    case "debugger.handshake":
      return {
        sessionId: client.sessionId,
        capabilities: ["watches", "step", "breakpoints", "frame-log", "waveforms"],
      };
    case "debugger.describe":
      return {
        emulator: { name: "nullDC", version: "dev", build: "native" as const },
        devices: buildDeviceTree(),
        breakpoints: Array.from(serverBreakpoints.values()),
        threads: sampleThreads,
      };
    case "debugger.subscribe": {
      const topics = new Set<string>((params.topics as string[]) ?? []);
      topics.forEach((topic) => client.topics.add(topic));
      return { acknowledged: Array.from(topics) };
    }
    case "debugger.unsubscribe": {
      const topics = (params.topics as string[]) ?? [];
      topics.forEach((topic) => client.topics.delete(topic));
      return { acknowledged: topics };
    }
    case "state.getRegisters":
      return { path: params.path, registers: mutateRegisters(baseRegisters) };
    case "state.getCache":
      return {
        path: params.path,
        cache: params.cache,
        entries: Array.from({ length: 16 }).map((_, index) => ({
          index,
          tag: `0x${(0x8000 + index).toString(16)}`,
          valid: Math.random() > 0.2,
        })),
      };
    case "state.getMemorySlice": {
      const target = typeof params.target === "string" ? params.target : "sh4";
      const addressValue = Number(params.address);
      const lengthValue = Number(params.length);
      const encoding = params.encoding as MemorySlice["encoding"] | undefined;
      const wordSize = params.wordSize as MemorySlice["wordSize"] | undefined;
      return buildMemorySlice({
        target,
        address: Number.isFinite(addressValue) ? addressValue : undefined,
        length: Number.isFinite(lengthValue) && lengthValue > 0 ? lengthValue : undefined,
        encoding,
        wordSize,
      });
    }
    case "state.getDisassembly": {
      const target = typeof params.target === "string" ? params.target : "sh4";
      const address = typeof params.address === "number" ? params.address : 0;
      const count = typeof params.count === "number" ? params.count : 128;
      const lines = generateDisassembly(target, address, count);
      return { lines };
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
      return { target, frames };
    }
    case "state.watch": {
      const expressions = (params.expressions as string[]) ?? [];
      expressions.forEach((expr) => {
        client.watches.add(expr);
        serverWatches.add(expr);
      });
      return { accepted: expressions, all: Array.from(serverWatches) };
    }
    case "state.unwatch": {
      const expressions = (params.expressions as string[]) ?? [];
      expressions.forEach((expr) => {
        client.watches.delete(expr);
        serverWatches.delete(expr);
      });
      return { accepted: expressions, all: Array.from(serverWatches) };
    }
    case "control.step":
      return { target: params.target, state: "halted" as const };
    case "control.runUntil":
      return { target: params.target, state: "running" as const, reason: "breakpoint" };
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

      // Broadcast to all clients
      for (const client of clients) {
        if (client.topics.has("state.breakpoint")) {
          sendNotification(client, {
            topic: "state.breakpoint",
            payload: breakpoint,
          });
        }
      }

      return { breakpoint, all: Array.from(serverBreakpoints.values()) };
    }
    case "breakpoints.remove": {
      const id = params.id as string;
      const removed = serverBreakpoints.delete(id);
      return { removed, all: Array.from(serverBreakpoints.values()) };
    }
    case "breakpoints.toggle": {
      const id = params.id as string;
      const enabled = params.enabled as boolean;
      const breakpoint = serverBreakpoints.get(id);
      if (!breakpoint) {
        throw new Error(`Breakpoint ${id} not found`);
      }
      const updated = { ...breakpoint, enabled };
      serverBreakpoints.set(id, updated);

      // Broadcast to all clients
      for (const client of clients) {
        if (client.topics.has("state.breakpoint")) {
          sendNotification(client, {
            topic: "state.breakpoint",
            payload: updated,
          });
        }
      }

      return { breakpoint: updated, all: Array.from(serverBreakpoints.values()) };
    }
    case "breakpoints.list":
      return { breakpoints: Array.from(serverBreakpoints.values()) };
    case "audio.requestWaveform":
      return buildWaveform(String(params.channelId ?? "0"), Number(params.window) || 256);
    case "logs.fetchFrameLog":
      return { frame: params.frame ?? 0, entries: frameLogEntries };
    default:
      throw new Error(`Unhandled JSON-RPC method: ${String(method)}`);
  }
};

const mutateRegisters = (registers: RegisterValue[]): RegisterValue[] =>
  registers.map((reg) => ({
    ...reg,
    value: reg.name === "PC" ? advancePc(reg.value) : reg.value,
  }));

const advancePc = (value: string): string => {
  const current = Number.parseInt(value, 16);
  const next = current + 2;
  return `0x${next.toString(16).toUpperCase().padStart(8, "0")}`;
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

const buildWaveform = (channelId: string, window: number): WaveformChunk => {
  const samples = Array.from({ length: window }).map((_, index) => Math.sin((index / window) * Math.PI * 4));
  return {
    channelId,
    sampleRate: 44_100,
    format: "pcm_f32",
    samples,
    label: `Channel ${channelId}`,
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

const broadcastTick = () => {
  // Mutate some register values to simulate execution
  const pcValue = registerValues.get("dc.sh4.cpu.pc");
  if (pcValue && pcValue.startsWith("0x")) {
    const pc = Number.parseInt(pcValue, 16);
    setRegisterValue("dc.sh4.cpu", "PC", `0x${(pc + 2).toString(16).toUpperCase().padStart(8, "0")}`);
  }

  const prValue = registerValues.get("dc.sh4.cpu.pr");
  if (prValue && prValue.startsWith("0x")) {
    const pr = Number.parseInt(prValue, 16);
    setRegisterValue("dc.sh4.cpu", "PR", `0x${(pr + 2).toString(16).toUpperCase().padStart(8, "0")}`);
  }

  // Mutate some AICA values
  const ch0Vol = registerValues.get("dc.aica.channels.ch0_vol");
  if (ch0Vol && ch0Vol.startsWith("0x")) {
    const vol = Number.parseInt(ch0Vol, 16);
    setRegisterValue("dc.aica.channels", "CH0_VOL", `0x${((vol + 1) & 0xFF).toString(16).toUpperCase().padStart(2, "0")}`);
  }

  const event = createFrameEvent();
  frameLogEntries.push(event);
  if (frameLogEntries.length > FRAME_LOG_LIMIT) {
    frameLogEntries.splice(0, frameLogEntries.length - FRAME_LOG_LIMIT);
  }

  // Get current device tree with live values
  const deviceTree = buildDeviceTree();
  const allRegisters = collectRegistersFromTree(deviceTree);

  for (const client of clients) {
    if (client.topics.has("state.registers")) {
      // Send register updates for all paths in device tree
      for (const { path, registers } of allRegisters) {
        sendNotification(client, {
          topic: "state.registers",
          payload: { path, registers },
        });
      }
    }

    if (client.topics.has("stream.frameLog")) {
      sendNotification(client, {
        topic: "stream.frameLog",
        payload: event,
      });
    }
    if (client.topics.has("stream.waveform")) {
      sendNotification(client, {
        topic: "stream.waveform",
        payload: buildWaveform("0", 128),
      });
    }

    if (serverWatches.size > 0) {
      for (const expression of serverWatches) {
        const value = registerValues.get(expression) ?? "0x00000000";
        sendNotification(client, {
          topic: "state.watch",
          payload: { expression, value },
        });
      }
    }
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
    app.get("*", async (_req, res, next) => {
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
      topics: new Set(),
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

  const timer = setInterval(broadcastTick, 1_000);

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













