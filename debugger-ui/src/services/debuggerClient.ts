import type { JsonRpcNotification } from "../lib/jsonRpc";
import type {
  BreakpointDescriptor,
  DebuggerNotification,
  DebuggerRpcSchema,
  FrameLogEntry,
  MemorySlice,
  RegisterValue,
  ThreadInfo,
  WaveformChunk,
} from "../lib/debuggerSchema";
import { JsonRpcClient } from "./jsonRpcClient";
import type { JsonRpcClientOptions } from "./jsonRpcClient";
import type { DebuggerTransport, TransportOptions } from "./transport";
import { createTransport } from "./transport";

export interface DebuggerClientConfig {
  mode: "native" | "wasm";
  endpoint: string;
  transportOptions?: TransportOptions;
  rpcOptions?: JsonRpcClientOptions;
}

export class DebuggerClient {
  private readonly transport: DebuggerTransport;
  private readonly rpc: JsonRpcClient<DebuggerRpcSchema>;
  private readonly config: DebuggerClientConfig;
  private notificationHandlers = new Set<(notification: DebuggerNotification) => void>();

  constructor(config: DebuggerClientConfig) {
    this.config = config;
    this.transport = createTransport(config.mode);
    this.rpc = new JsonRpcClient<DebuggerRpcSchema>(this.transport, config.rpcOptions);
    this.rpc.onNotification((notification) => {
      const mapped = mapNotification(notification);
      if (mapped) {
        this.notificationHandlers.forEach((handler) => handler(mapped));
      }
    });
  }

  async connect() {
    await this.rpc.connect(this.config.endpoint, this.config.transportOptions);
  }

  disconnect() {
    this.rpc.disconnect();
  }

  async handshake(clientName: string, clientVersion: string, transportBuild: "native" | "wasm") {
    return this.rpc.call("debugger.handshake", {
      clientName,
      clientVersion,
      transport: { build: transportBuild },
    });
  }

  async describe(include?: ("devices" | "breakpoints" | "threads")[]) {
    return this.rpc.call("debugger.describe", { include });
  }

  async fetchRegisters(path: string) {
    return this.rpc.call("state.getRegisters", { path });
  }

  async fetchDeviceTree() {
    const { devices } = await this.rpc.call("debugger.describe", { include: ["devices"] });
    return devices;
  }

  async subscribe(topics: string[]) {
    if (!topics.length) {
      return { acknowledged: [] as string[] };
    }
    return this.rpc.call("debugger.subscribe", { topics });
  }

  async unsubscribe(topics: string[]) {
    if (!topics.length) {
      return { acknowledged: [] as string[] };
    }
    return this.rpc.call("debugger.unsubscribe", { topics });
  }

  async watch(expressions: string[]) {
    if (!expressions.length) {
      return { accepted: [] as string[] };
    }
    return this.rpc.call("state.watch", { expressions });
  }

  async unwatch(expressions: string[]) {
    if (!expressions.length) {
      return { accepted: [] as string[] };
    }
    return this.rpc.call("state.unwatch", { expressions });
  }

  async fetchMemorySlice(params: {
    target: string;
    address: number;
    length: number;
    encoding?: MemorySlice["encoding"];
    wordSize?: MemorySlice["wordSize"];
  }) {
    return this.rpc.call("state.getMemorySlice", params);
  }

  async fetchDisassembly(params: { target: string; address: number; count: number; context?: number }) {
    return this.rpc.call("state.getDisassembly", params);
  }

  async fetchFrameLog(frame: number, limit?: number) {
    return this.rpc.call("logs.fetchFrameLog", { frame, limit });
  }

  async sendNotification(method: keyof DebuggerRpcSchema, params: unknown) {
    this.rpc.notify(method, params as never);
  }

  onNotification(handler: (notification: DebuggerNotification) => void): () => void {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }
}

const mapNotification = (notification: JsonRpcNotification): DebuggerNotification | undefined => {
  const { method, params } = notification;
  switch (method) {
    case "event.state.registers":
      return {
        topic: "state.registers",
        payload: params as { path: string; registers: RegisterValue[] },
      };
    case "event.state.watch":
      return {
        topic: "state.watch",
        payload: params as { expression: string; value: unknown },
      };
    case "event.state.breakpoint":
      return {
        topic: "state.breakpoint",
        payload: params as BreakpointDescriptor,
      };
    case "event.state.thread":
      return {
        topic: "state.thread",
        payload: params as ThreadInfo,
      };
    case "event.stream.waveform":
      return {
        topic: "stream.waveform",
        payload: params as WaveformChunk,
      };
    case "event.stream.frameLog":
      return {
        topic: "stream.frameLog",
        payload: params as FrameLogEntry,
      };
    default:
      return undefined;
  }
};
