import type { JsonRpcNotification } from "../lib/jsonRpc";
import type {
  BreakpointDescriptor,
  BreakpointId,
  CallstackFrame,
  DebuggerNotification,
  DebuggerRpcSchema,
  RpcError,
  TargetProcessor,
  WatchId,
} from "../lib/debuggerSchema";
import { DebuggerRpcMethodSchemas, RpcMethod } from "../lib/debuggerSchema";
import { JsonRpcClient } from "./jsonRpcClient";
import type { JsonRpcClientOptions } from "./jsonRpcClient";
import type { DebuggerTransport, TransportOptions, TransportState, TransportStateHandler } from "./transport";
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
    return this.rpc.call(RpcMethod.DEBUGGER_DESCRIBE, {});
  }

  async fetchCallstack(target: "sh4" | "arm7", maxFrames = 32): Promise<{ target: string; frames: CallstackFrame[] }> {
    return this.rpc.call(RpcMethod.STATE_GET_CALLSTACK, { target, maxFrames });
  }

  async watch(expressions: string[]) {
    if (!expressions.length) {
      return {};
    }
    return this.rpc.call(RpcMethod.STATE_WATCH, { expressions });
  }

  async unwatch(watchIds: WatchId[]) {
    if (!watchIds.length) {
      return {};
    }
    return this.rpc.call(RpcMethod.STATE_UNWATCH, { watchIds });
  }

  async editWatch(watchId: WatchId, value: string) {
    return this.rpc.call(RpcMethod.STATE_EDIT_WATCH, { watchId, value });
  }

  async modifyWatchExpression(watchId: WatchId, newExpression: string) {
    return this.rpc.call(RpcMethod.STATE_MODIFY_WATCH_EXPRESSION, { watchId, newExpression });
  }

  async addBreakpoint(event: string, address?: number, kind: BreakpointDescriptor["kind"] = "code", enabled = true) {
    return this.rpc.call(RpcMethod.BREAKPOINTS_ADD, { event, address, kind, enabled });
  }

  async removeBreakpoint(id: BreakpointId) {
    return this.rpc.call(RpcMethod.BREAKPOINTS_REMOVE, { id });
  }

  async toggleBreakpoint(id: BreakpointId, enabled: boolean) {
    return this.rpc.call(RpcMethod.BREAKPOINTS_TOGGLE, { id, enabled });
  }

  async setCategoryStates(categories: Record<string, { muted: boolean; soloed: boolean }>) {
    return this.rpc.call(RpcMethod.BREAKPOINTS_SET_CATEGORY_STATES, { categories });
  }

  async pause(target?: TargetProcessor) {
    return this.rpc.call(RpcMethod.CONTROL_PAUSE, { target });
  }

  async step(target: TargetProcessor) {
    return this.rpc.call(RpcMethod.CONTROL_STEP, { target });
  }

  async stepOver(target: TargetProcessor) {
    return this.rpc.call(RpcMethod.CONTROL_STEP_OVER, { target });
  }

  async stepOut(target: TargetProcessor) {
    return this.rpc.call(RpcMethod.CONTROL_STEP_OUT, { target });
  }

  async runUntil() {
    return this.rpc.call(RpcMethod.CONTROL_RUN_UNTIL, {});
  }

  async fetchMemorySlice(params: {
    target: TargetProcessor;
    address: number;
    length: number;
  }) {
    return this.rpc.call(RpcMethod.STATE_GET_MEMORY_SLICE, params);
  }

  async fetchDisassembly(params: { target: TargetProcessor; address: number; count: number; context?: number }) {
    return this.rpc.call(RpcMethod.STATE_GET_DISASSEMBLY, params);
  }

  async fetchSgcFrameData(): Promise<ArrayBuffer> {
    // Set up a promise to wait for the binary data
    const binaryDataPromise = new Promise<ArrayBuffer>((resolve) => {
      const unsubscribe = this.transport.subscribeBinary((data) => {
        unsubscribe();
        resolve(data);
      });
    });

    // Make the RPC call (which triggers the server to send binary data)
    const result = await this.rpc.call("state.getSgcFrameData" as keyof DebuggerRpcSchema, {});

    // Check if the result contains an error
    if (result && typeof result === 'object' && 'error' in result) {
      const rpcError = result as RpcError;
      if (rpcError.error) {
        throw new Error(rpcError.error.message || 'Failed to fetch SGC frame data');
      }
    }

    // Wait for and return the binary data
    return binaryDataPromise;
  }

  async recordSgcFrames(): Promise<void> {
    // Request the server to record 1024 frames
    await this.rpc.call("state.recordSgcFrames" as keyof DebuggerRpcSchema, {});
  }

  async sendNotification(method: keyof DebuggerRpcSchema, params: unknown) {
    this.rpc.notify(method, params as never);
  }

  onNotification(handler: (notification: DebuggerNotification) => void): () => void {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }

  onTransportStateChange(handler: TransportStateHandler): () => void {
    return this.transport.onStateChange(handler);
  }

  get transportState(): TransportState {
    return this.transport.state;
  }
}

const mapNotification = (notification: JsonRpcNotification): DebuggerNotification | undefined => {
  const { method, params } = notification;
  switch (method) {
    case RpcMethod.EVENT_TICK:
      return {
        topic: "tick",
        payload: params as import("../lib/debuggerSchema").DebuggerTick,
      };
    default:
      return undefined;
  }
};
