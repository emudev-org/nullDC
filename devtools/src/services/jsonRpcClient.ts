import { z } from "zod";
import {
  JSON_RPC_VERSION,
  JsonRpcException,
  isJsonRpcError,
  isJsonRpcNotification,
  isJsonRpcRequest,
  isJsonRpcSuccess,
} from "../lib/jsonRpc";
import type {
  JsonRpcError,
  JsonRpcMessage,
  JsonRpcNotification,
  JsonRpcRequest,
  JsonRpcSuccess,
  RpcParams,
  RpcResult,
  RpcSchema,
} from "../lib/jsonRpc";
import type { DebuggerTransport, TransportOptions } from "./transport";

export type RpcMethodSchemas = Record<string, { params: z.ZodType; result: z.ZodType }>;

export interface JsonRpcClientOptions<T extends RpcMethodSchemas = RpcMethodSchemas> {
  requestTimeoutMs?: number;
  validationSchemas?: T;
  validateResponses?: boolean;
}

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (reason?: unknown) => void;
  timeout?: number;
  method?: string;
}

export type NotificationCallback = (notification: JsonRpcNotification) => void;

export class JsonRpcClient<S extends RpcSchema> {
  private readonly transport: DebuggerTransport;
  private idCounter = 0;
  private readonly pending = new Map<number, PendingRequest>();
  private notificationHandlers = new Set<NotificationCallback>();
  private readonly requestTimeoutMs: number;
  private readonly validationSchemas?: RpcMethodSchemas;
  private readonly validateResponses: boolean;

  constructor(transport: DebuggerTransport, options?: JsonRpcClientOptions) {
    this.transport = transport;
    this.requestTimeoutMs = options?.requestTimeoutMs ?? 10_000;
    this.validationSchemas = options?.validationSchemas;
    this.validateResponses = options?.validateResponses ?? true;
    this.transport.subscribe((payload) => this.handlePayload(payload));
  }

  async connect(endpoint: string, options?: TransportOptions) {
    await this.transport.connect(endpoint, options);
  }

  disconnect() {
    this.transport.disconnect();
    this.clearPending(new Error("Transport disconnected"));
  }

  async call<M extends keyof S>(method: M, params: RpcParams<S, M>): Promise<RpcResult<S, M>> {
    // Validate params if schemas are provided
    if (this.validateResponses && this.validationSchemas) {
      const schema = this.validationSchemas[method as string];
      if (schema?.params) {
        try {
          schema.params.parse(params);
        } catch (error) {
          if (error instanceof z.ZodError) {
            throw new Error(`Parameter validation failed for ${String(method)}: ${error.message}`);
          }
          throw error;
        }
      }
    }

    const id = ++this.idCounter;
    const request: JsonRpcRequest<RpcParams<S, M>> = {
      jsonrpc: JSON_RPC_VERSION,
      id,
      method: method as string,
      params,
    };
    const payload = JSON.stringify(request);

    const result = await new Promise<unknown>((resolve, reject) => {
      const timeout = window.setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Request timed out: ${String(method)}`));
      }, this.requestTimeoutMs);

      this.pending.set(id, {
        resolve: (value) => {
          window.clearTimeout(timeout);
          resolve(value);
        },
        reject: (reason) => {
          window.clearTimeout(timeout);
          reject(reason);
        },
        method: method as string,
      });

      try {
        this.transport.send(payload);
      } catch (error) {
        window.clearTimeout(timeout);
        this.pending.delete(id);
        // Wrap the error with the method name for better error messages
        const errorMessage = error instanceof Error ? error.message : String(error);
        reject(new Error(`${String(method)}: ${errorMessage}`));
      }
    });

    return result as RpcResult<S, M>;
  }

  notify<M extends keyof S>(method: M, params: RpcParams<S, M>): void {
    // Validate params if schemas are provided
    if (this.validateResponses && this.validationSchemas) {
      const schema = this.validationSchemas[method as string];
      if (schema?.params) {
        try {
          schema.params.parse(params);
        } catch (error) {
          if (error instanceof z.ZodError) {
            throw new Error(`Parameter validation failed for ${String(method)}: ${error.message}`);
          }
          throw error;
        }
      }
    }

    const notification: JsonRpcNotification<RpcParams<S, M>> = {
      jsonrpc: JSON_RPC_VERSION,
      method: method as string,
      params,
    };
    this.transport.send(JSON.stringify(notification));
  }

  onNotification(handler: NotificationCallback): () => void {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }

  private handlePayload(payload: string) {
    let message: JsonRpcMessage;
    try {
      message = JSON.parse(payload);
    } catch (error) {
      console.error("Failed to parse JSON-RPC payload", error);
      return;
    }

    if (!message || message.jsonrpc !== JSON_RPC_VERSION) {
      console.warn("Ignoring invalid JSON-RPC message", message);
      return;
    }

    if (isJsonRpcRequest(message) || isJsonRpcNotification(message)) {
      const notification = message as JsonRpcNotification;

      // Validate notification if schemas are provided
      if (this.validateResponses && this.validationSchemas && notification.method) {
        const schema = this.validationSchemas[notification.method];
        if (schema?.params) {
          try {
            schema.params.parse(notification.params);
          } catch (error) {
            if (error instanceof z.ZodError) {
              console.error(
                `Notification validation failed for ${notification.method}:`,
                error.message
              );
              return;
            }
          }
        }
      }

      this.notificationHandlers.forEach((handler) => handler(notification));
      return;
    }

    const response = message as JsonRpcSuccess | JsonRpcError;
    const id = (response as JsonRpcSuccess).id;

    if (typeof id !== "number") {
      console.warn("Received response without numeric id", response);
      return;
    }

    const pending = this.pending.get(id);
    if (!pending) {
      console.warn("No pending request for id", id, response);
      return;
    }

    this.pending.delete(id);

    if (isJsonRpcSuccess(response)) {
      // Validate response if schemas are provided
      if (this.validateResponses && this.validationSchemas && pending.method) {
        const schema = this.validationSchemas[pending.method];
        if (schema?.result) {
          try {
            const validated = schema.result.parse(response.result);
            pending.resolve(validated);
            return;
          } catch (error) {
            if (error instanceof z.ZodError) {
              pending.reject(
                new Error(`Response validation failed for ${pending.method}: ${error.message}`)
              );
              return;
            }
            pending.reject(error);
            return;
          }
        }
      }
      pending.resolve(response.result);
      return;
    }

    if (isJsonRpcError(response)) {
      pending.reject(new JsonRpcException(response.error));
      return;
    }

    pending.reject(new Error("Unknown JSON-RPC response"));
  }

  private clearPending(error: Error) {
    this.pending.forEach((pending) => pending.reject(error));
    this.pending.clear();
  }
}
