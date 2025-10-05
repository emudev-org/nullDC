import type { RpcSchema } from "./jsonRpc";

export type DeviceKind =
  | "bus"
  | "processor"
  | "coprocessor"
  | "peripheral"
  | "memory"
  | "channel"
  | "pipeline"
  | "debugger";

export interface DeviceNodeDescriptor {
  path: string;
  label: string;
  kind: DeviceKind;
  description?: string;
  registers?: RegisterValue[];
  children?: DeviceNodeDescriptor[];
}

export interface RegisterValue {
  name: string;
  value: string;
  width: number;
  flags?: Record<string, boolean>;
  metadata?: Record<string, unknown>;
}

export interface MemorySlice {
  baseAddress: number;
  wordSize: 1 | 2 | 4 | 8;
  encoding: "hex" | "uint" | "float" | "ascii";
  data: string;
  validity: "ok" | "tlb-miss" | "fault";
}

export interface DisassemblyLine {
  address: number;
  bytes: string;
  mnemonic: string;
  operands: string;
  comment?: string;
  isCurrent?: boolean;
  isBreakpoint?: boolean;
}

export interface BreakpointDescriptor {
  id: string;
  location: string;
  kind: "code" | "data" | "event";
  enabled: boolean;
  condition?: string;
  hitCount: number;
  pending?: boolean;
}

export interface ThreadInfo {
  id: string;
  name?: string;
  state: "running" | "stopped" | "blocked" | "unknown";
  core?: string;
  priority?: number;
  backtrace?: BacktraceFrame[];
}

export interface BacktraceFrame {
  index: number;
  pc: number;
  symbol?: string;
  location?: string;
}

export interface WaveformChunk {
  channelId: string;
  sampleRate: number;
  format: "pcm_u8" | "pcm_s16" | "pcm_f32";
  samples: number[];
  label?: string;
}

export interface FrameLogEntry {
  timestamp: number;
  subsystem: "sh4" | "holly" | "ta" | "core" | "aica" | "dsp";
  severity: "trace" | "info" | "warn" | "error";
  message: string;
  metadata?: Record<string, unknown>;
}

export interface TransportSettings {
  sessionToken?: string;
  build: "native" | "wasm";
}

export interface CallstackFrame {
  index: number;
  pc: number;
  sp?: number;
  symbol?: string;
  location?: string;
}

export type DebuggerRpcSchema = RpcSchema & {
  "debugger.handshake": {
    params: { clientName: string; clientVersion: string; transport: TransportSettings };
    result: { sessionId: string; capabilities: string[] };
  };
  "state.getCallstack": {
    params: { target: "sh4" | "arm7"; maxFrames?: number };
    result: { target: string; frames: CallstackFrame[] };
  };
  "debugger.describe": {
    params: { include?: ("devices" | "breakpoints" | "threads")[] };
    result: {
      emulator: { name: string; version: string; build: "native" | "wasm" };
      devices: DeviceNodeDescriptor[];
      breakpoints: BreakpointDescriptor[];
      threads: ThreadInfo[];
    };
  };
  "debugger.subscribe": {
    params: {
      topics: string[];
    };
    result: { acknowledged: string[] };
  };
  "debugger.unsubscribe": {
    params: { topics: string[] };
    result: { acknowledged: string[] };
  };
  "state.getRegisters": {
    params: { path: string };
    result: { path: string; registers: RegisterValue[] };
  };
  "state.getCache": {
    params: { path: string; cache: "icache" | "dcache" | "utlb" | "itlb" };
    result: { path: string; cache: string; entries: Record<string, unknown>[] };
  };
  "state.getMemorySlice": {
    params: { target?: string; address: number; length: number; encoding?: MemorySlice["encoding"]; wordSize?: MemorySlice["wordSize"]; };
    result: MemorySlice;
  };
  "state.getDisassembly": {
    params: { target?: string; address: number; count: number; context?: number };
    result: { lines: DisassemblyLine[] };
  };
  "state.watch": {
    params: { expressions: string[] };
    result: { accepted: string[]; all: string[] };
  };
  "state.unwatch": {
    params: { expressions: string[] };
    result: { accepted: string[]; all: string[] };
  };
  "control.step": {
    params: { target: string; granularity: "instruction" | "block" | "event"; modifiers?: string[] };
    result: { target: string; state: "running" | "halted" };
  };
  "control.runUntil": {
    params: { target: string; type: "interrupt" | "exception" | "primitive" | "tile" | "vertex" | "list" | "sample"; value?: string };
    result: { target: string; state: "running" | "halted"; reason?: string };
  };
  "breakpoints.set": {
    params: { breakpoint: BreakpointDescriptor };
    result: BreakpointDescriptor;
  };
  "breakpoints.remove": {
    params: { id: string };
    result: { removed: boolean };
  };
  "audio.requestWaveform": {
    params: { channelId: string; window: number };
    result: WaveformChunk;
  };
  "logs.fetchFrameLog": {
    params: { frame: number; limit?: number };
    result: { frame: number; entries: FrameLogEntry[] };
  };
};

export type DebuggerNotification =
  | {
      topic: "state.registers";
      payload: { path: string; registers: RegisterValue[] };
    }
  | {
      topic: "state.watch";
      payload: { expression: string; value: unknown };
    }
  | {
      topic: "state.breakpoint";
      payload: BreakpointDescriptor;
    }
  | {
      topic: "state.thread";
      payload: ThreadInfo;
    }
  | {
      topic: "stream.waveform";
      payload: WaveformChunk;
    }
  | {
      topic: "stream.frameLog";
      payload: FrameLogEntry;
    };


