import type { RpcSchema } from "./jsonRpc";

export interface DeviceNodeDescriptor {
  path: string;
  label: string;
  kind: string;
  description?: string;
  registers?: RegisterValue[];
  events?: string[];
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
  eventId: string;
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

export interface DebuggerShape {
  emulator: { name: string; version: string; build: "native" | "wasm" };
  deviceTree: DeviceNodeDescriptor[];
  capabilities: string[];
}

export interface DebuggerTick {
  tickId: number;
  timestamp: number;
  executionState: {
    state: "running" | "paused";
    breakpointId?: string;
  };
  registers: Record<string, RegisterValue[]>;
  breakpoints: Record<string, BreakpointDescriptor>;
  eventLog: FrameLogEntry[];
  watches?: Record<string, unknown>;
  threads?: ThreadInfo[];
}

export interface RpcError {
  error?: { code: number; message: string };
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
    params: Record<string, never>;
    result: DebuggerShape;
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
    result: RpcError;
  };
  "state.unwatch": {
    params: { expressions: string[] };
    result: RpcError;
  };
  "control.step": {
    params: { target: string; granularity: "instruction" | "block" | "event"; modifiers?: string[] };
    result: RpcError;
  };
  "control.runUntil": {
    params: { target: string; type: "interrupt" | "exception" | "primitive" | "tile" | "vertex" | "list" | "sample"; value?: string };
    result: RpcError;
  };
  "control.pause": {
    params: { target?: string };
    result: RpcError;
  };
  "breakpoints.add": {
    params: { location: string; kind?: BreakpointDescriptor["kind"]; enabled?: boolean };
    result: RpcError;
  };
  "breakpoints.setCategoryStates": {
    params: { categories: Record<string, { muted: boolean; soloed: boolean }> };
    result: RpcError;
  };
  "breakpoints.remove": {
    params: { id: string };
    result: RpcError;
  };
  "breakpoints.toggle": {
    params: { id: string; enabled: boolean };
    result: RpcError;
  };
  "audio.requestWaveform": {
    params: { channelId: string; window: number };
    result: WaveformChunk;
  };
};

export type DebuggerNotification =
  | {
      topic: "tick";
      payload: DebuggerTick;
    }
  | {
      topic: "stream.waveform";
      payload: WaveformChunk;
    };


