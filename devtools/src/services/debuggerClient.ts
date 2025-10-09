import type { JsonRpcNotification } from "../lib/jsonRpc";
import type {
  BreakpointDescriptor,
  BreakpointId,
  CallstackFrame,
  DebuggerNotification,
  DebuggerRpcSchema,
  TargetProcessor,
  WatchId,
} from "../lib/debuggerSchema";
import { DebuggerRpcMethodSchemas } from "../lib/debuggerSchema";
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
    this.rpc = new JsonRpcClient<DebuggerRpcSchema>(this.transport, {
      ...config.rpcOptions,
      validationSchemas: DebuggerRpcMethodSchemas,
      validateResponses: true,
    });
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

  async describe() {
    return this.rpc.call("debugger.describe", {});
  }

  async fetchCallstack(target: "sh4" | "arm7", maxFrames = 32): Promise<{ target: string; frames: CallstackFrame[] }> {
    return this.rpc.call("state.getCallstack", { target, maxFrames });
  }

  async watch(expressions: string[]) {
    if (!expressions.length) {
      return {};
    }
    return this.rpc.call("state.watch", { expressions });
  }

  async unwatch(watchIds: WatchId[]) {
    if (!watchIds.length) {
      return {};
    }
    return this.rpc.call("state.unwatch", { watchIds });
  }

  async editWatch(watchId: WatchId, value: string) {
    return this.rpc.call("state.editWatch", { watchId, value });
  }

  async modifyWatchExpression(watchId: WatchId, newExpression: string) {
    return this.rpc.call("state.modifyWatchExpression", { watchId, newExpression });
  }

  async addBreakpoint(event: string, address?: number, kind: BreakpointDescriptor["kind"] = "code", enabled = true) {
    return this.rpc.call("breakpoints.add", { event, address, kind, enabled });
  }

  async removeBreakpoint(id: BreakpointId) {
    return this.rpc.call("breakpoints.remove", { id });
  }

  async toggleBreakpoint(id: BreakpointId, enabled: boolean) {
    return this.rpc.call("breakpoints.toggle", { id, enabled });
  }

  async setCategoryStates(categories: Record<string, { muted: boolean; soloed: boolean }>) {
    return this.rpc.call("breakpoints.setCategoryStates", { categories });
  }

  async pause(target?: TargetProcessor) {
    return this.rpc.call("control.pause", { target });
  }

  async step(target: TargetProcessor) {
    return this.rpc.call("control.step", { target });
  }

  async stepOver(target: TargetProcessor) {
    return this.rpc.call("control.stepOver", { target });
  }

  async stepOut(target: TargetProcessor) {
    return this.rpc.call("control.stepOut", { target });
  }

  async runUntil() {
    return this.rpc.call("control.runUntil", {});
  }

  async fetchMemorySlice(params: {
    target: TargetProcessor;
    address: number;
    length: number;
  }) {
    return this.rpc.call("state.getMemorySlice", params);
  }

  async fetchDisassembly(params: { target: TargetProcessor; address: number; count: number; context?: number }) {
    return this.rpc.call("state.getDisassembly", params);
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
    case "event.tick":
      return {
        topic: "tick",
        payload: params as import("../lib/debuggerSchema").DebuggerTick,
      };
    default:
      return undefined;
  }
};
