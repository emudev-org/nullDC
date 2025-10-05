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

interface ClientContext {
  socket: WebSocket;
  sessionId: string;
  topics: Set<string>;
  watches: Set<string>;
}

const deviceTree: DeviceNodeDescriptor[] = [
  {
    path: "dc",
    label: "Dreamcast",
    kind: "bus",
    description: "Sega Dreamcast system bus",
    registers: [
      { name: "SYSCLK", value: "200MHz", width: 0 },
      { name: "ASIC_REV", value: "0x0001", width: 16 },
    ],
    children: [
      {
        path: "dc.sh4",
        label: "SH4",
        kind: "processor",
        description: "Hitachi SH-4 main CPU",
        registers: [
          { name: "VBR", value: "0x8C000000", width: 32 },
          { name: "SR", value: "0x40000000", width: 32 },
          { name: "FPSCR", value: "0x00040001", width: 32 },
        ],
        children: [
          {
            path: "dc.sh4.cpu",
            label: "Core",
            kind: "processor",
            description: "Integer pipeline",
            registers: [
              { name: "PC", value: "0x8C0000A0", width: 32 },
              { name: "PR", value: "0x8C0000A2", width: 32 },
            ],
          },
          {
            path: "dc.sh4.icache",
            label: "I-Cache",
            kind: "peripheral",
            description: "Instruction cache",
            registers: [
              { name: "ICRAM", value: "16KB", width: 0 },
              { name: "ICACHE_CTRL", value: "0x00000003", width: 32 },
            ],
          },
          {
            path: "dc.sh4.dcache",
            label: "D-Cache",
            kind: "peripheral",
            description: "Data cache",
            registers: [
              { name: "DCRAM", value: "8KB", width: 0 },
              { name: "DCACHE_CTRL", value: "0x00000003", width: 32 },
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
          { name: "HOLLY_ID", value: "0x00050000", width: 32 },
          { name: "DMAC_CTRL", value: "0x00000001", width: 32 },
        ],
        children: [
          {
            path: "dc.holly.dmac",
            label: "DMA Controller",
            kind: "peripheral",
            registers: [
              { name: "DMAOR", value: "0x8201", width: 16 },
              { name: "CHCR0", value: "0x00000001", width: 32 },
            ],
          },
          {
            path: "dc.holly.ta",
            label: "TA",
            kind: "pipeline",
            registers: [
              { name: "TA_LIST_BASE", value: "0x0C000000", width: 32 },
              { name: "TA_STATUS", value: "0x00000000", width: 32 },
            ],
          },
          {
            path: "dc.holly.core",
            label: "CORE",
            kind: "pipeline",
            registers: [
              { name: "PVR_CTRL", value: "0x00000001", width: 32 },
              { name: "PVR_STATUS", value: "0x00010000", width: 32 },
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
          { name: "AICA_CTRL", value: "0x00000002", width: 32 },
          { name: "AICA_STATUS", value: "0x00000001", width: 32 },
        ],
        children: [
          {
            path: "dc.aica.channels",
            label: "Channels",
            kind: "channel",
            registers: [
              { name: "CH0_VOL", value: "0x7F", width: 8 },
              { name: "CH1_VOL", value: "0x6A", width: 8 },
            ],
          },
          {
            path: "dc.aica.dsp",
            label: "DSP",
            kind: "coprocessor",
            registers: [
              { name: "DSP_PC", value: "0x020", width: 16 },
              { name: "DSP_ACC", value: "0x1F", width: 16 },
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

const sampleSh4Disassembly: DisassemblyLine[] = [
  { address: 0x8c0000a0, bytes: "02 45", mnemonic: "mov.l", operands: "@r15+, r1", isCurrent: true },
  { address: 0x8c0000a2, bytes: "6E F6", mnemonic: "mov", operands: "r15, r14" },
  { address: 0x8c0000a4, bytes: "4F 22", mnemonic: "sts.l", operands: "pr, @-r15" },
  { address: 0x8c0000a6, bytes: "2F 46", mnemonic: "mov", operands: "r4, r15" },
];

const sampleArm7Disassembly: DisassemblyLine[] = [
  { address: 0x00200000, bytes: "E3 A0 00 01", mnemonic: "mov", operands: "r0, #1", isCurrent: true },
  { address: 0x00200004, bytes: "E5 9F 10 04", mnemonic: "ldr", operands: "r1, [pc, #4]" },
  { address: 0x00200008, bytes: "E1 2F FF 1E", mnemonic: "bx", operands: "lr" },
  { address: 0x0020000C, bytes: "E5 8D 20 00", mnemonic: "str", operands: "r2, [sp]" },
];

const sampleDspDisassembly: DisassemblyLine[] = [
  { address: 0x00000000, bytes: "20 0C", mnemonic: "ld", operands: "r0, @0x0C", isCurrent: true },
  { address: 0x00000002, bytes: "21 10", mnemonic: "ld", operands: "r1, @0x10" },
  { address: 0x00000004, bytes: "31 01", mnemonic: "add", operands: "acc, r0, r1" },
  { address: 0x00000006, bytes: "E0 00", mnemonic: "store", operands: "acc, @0x00" },
];

const disassemblyByTarget: Record<string, DisassemblyLine[]> = {
  sh4: sampleSh4Disassembly,
  arm7: sampleArm7Disassembly,
  dsp: sampleDspDisassembly,
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
        devices: deviceTree,
        breakpoints: sampleBreakpoints,
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
      const lines = disassemblyByTarget[target] ?? sampleSh4Disassembly;
      return { lines };
    }
    case "state.watch": {
      const expressions = (params.expressions as string[]) ?? [];
      expressions.forEach((expr) => client.watches.add(expr));
      return { accepted: expressions };
    }
    case "state.unwatch": {
      const expressions = (params.expressions as string[]) ?? [];
      expressions.forEach((expr) => client.watches.delete(expr));
      return { accepted: expressions };
    }
    case "control.step":
      return { target: params.target, state: "halted" as const };
    case "control.runUntil":
      return { target: params.target, state: "running" as const, reason: "breakpoint" };
    case "breakpoints.set": {
      const breakpoint = params.breakpoint as BreakpointDescriptor;
      return { ...breakpoint, id: breakpoint.id ?? `bp-${randomUUID().slice(0, 8)}` };
    }
    case "breakpoints.remove":
      return { removed: true };
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

const broadcastTick = () => {
  const event = createFrameEvent();
  frameLogEntries.push(event);
  if (frameLogEntries.length > FRAME_LOG_LIMIT) {
    frameLogEntries.splice(0, frameLogEntries.length - FRAME_LOG_LIMIT);
  }
  for (const client of clients) {
    if (client.topics.has("state.registers")) {
      sendNotification(client, {
        topic: "state.registers",
        payload: { path: "dc.sh4.cpu", registers: mutateRegisters(baseRegisters) },
      });
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

    if (client.watches.size > 0) {
      for (const expression of client.watches) {
        sendNotification(client, {
          topic: "state.watch",
          payload: { expression, value: Math.floor(Math.random() * 0xffff) },
        });
      }
    }
  }
};

const start = async () => {
  const app = express();
  const vite = await createViteServer({
    server: {
      middlewareMode: true,
    },
    appType: "custom",
  });

  app.use(vite.middlewares);

  app.get("/health", (_req, res) => {
    res.json({ status: "ok" });
  });

  app.use(async (req, res, next) => {
    try {
      const template = await readFile(resolve(process.cwd(), "index.html"), "utf8");
      const transformed = await vite.transformIndexHtml(req.originalUrl, template);
      res.status(200).set({ "Content-Type": "text/html" }).end(transformed);
    } catch (error) {
      vite.ssrFixStacktrace(error as Error);
      next(error);
    }
  });

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
    await vite.close();
    server.close(() => process.exit(0));
  };

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);
};

void start();















