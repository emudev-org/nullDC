export type TransportState = "idle" | "connecting" | "open" | "closed";

export type TransportMessageHandler = (payload: string) => void;
export type TransportStateHandler = (state: TransportState, event?: Event | Error) => void;

export interface TransportOptions {
  protocols?: string | string[];
  channelName?: string;
}

export interface DebuggerTransport {
  readonly kind: "websocket" | "broadcast";
  readonly state: TransportState;
  connect(endpoint: string, options?: TransportOptions): Promise<void>;
  disconnect(code?: number, reason?: string): void;
  send(payload: string): void;
  subscribe(handler: TransportMessageHandler): () => void;
  onStateChange(handler: TransportStateHandler): () => void;
}

abstract class BaseTransport implements DebuggerTransport {
  public abstract readonly kind: DebuggerTransport["kind"];
  public state: TransportState = "idle";
  protected messageHandlers = new Set<TransportMessageHandler>();
  protected stateHandlers = new Set<TransportStateHandler>();

  abstract connect(endpoint: string, options?: TransportOptions): Promise<void>;
  abstract disconnect(code?: number, reason?: string): void;
  abstract send(payload: string): void;

  public subscribe(handler: TransportMessageHandler): () => void {
    this.messageHandlers.add(handler);
    return () => this.messageHandlers.delete(handler);
  }

  public onStateChange(handler: TransportStateHandler): () => void {
    this.stateHandlers.add(handler);
    return () => this.stateHandlers.delete(handler);
  }

  protected broadcastState(state: TransportState, event?: Event | Error) {
    this.state = state;
    this.stateHandlers.forEach((handler) => handler(state, event));
  }

  protected broadcastMessage(payload: string) {
    this.messageHandlers.forEach((handler) => handler(payload));
  }
}

export class WebSocketTransport extends BaseTransport {
  public readonly kind = "websocket" as const;
  private socket?: WebSocket;

  async connect(endpoint: string, options?: TransportOptions): Promise<void> {
    if (this.socket && this.state === "open") {
      return;
    }

    this.state = "connecting";

    await new Promise<void>((resolve, reject) => {
      const ws = new WebSocket(endpoint, options?.protocols);
      this.socket = ws;

      const handleOpen = () => {
        ws.removeEventListener("open", handleOpen);
        ws.removeEventListener("error", handleError);
        ws.addEventListener("message", (event) => {
          this.broadcastMessage(String(event.data));
        });
        ws.addEventListener("close", () => {
          this.broadcastState("closed");
        });
        this.broadcastState("open");
        resolve();
      };

      const handleError = (event: Event) => {
        ws.removeEventListener("open", handleOpen);
        ws.removeEventListener("error", handleError);
        this.broadcastState("closed", event);
        reject(event);
      };

      ws.addEventListener("open", handleOpen, { once: true });
      ws.addEventListener("error", handleError, { once: true });
    });
  }

  disconnect(code?: number, reason?: string) {
    if (!this.socket) {
      return;
    }

    this.socket.close(code, reason);
    this.socket = undefined;
    this.broadcastState("closed");
  }

  send(payload: string) {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      throw new Error("WebSocket is not open");
    }

    this.socket.send(payload);
  }
}

export class BroadcastChannelTransport extends BaseTransport {
  public readonly kind = "broadcast" as const;
  private channel?: BroadcastChannel;

  async connect(endpoint: string, options?: TransportOptions): Promise<void> {
    if (typeof BroadcastChannel === "undefined") {
      throw new Error("BroadcastChannel is not supported in this environment");
    }

    if (this.channel) {
      return;
    }

    this.state = "connecting";
    const channelName = options?.channelName ?? endpoint;
    const channel = new BroadcastChannel(channelName);
    this.channel = channel;
    channel.addEventListener("message", (event) => {
      if (typeof event.data === "string") {
        this.broadcastMessage(event.data);
      }
    });
    channel.addEventListener("messageerror", (event) => {
      this.broadcastState("closed", event);
    });
    this.broadcastState("open");
  }

  disconnect(): void {
    if (!this.channel) {
      return;
    }
    this.channel.close();
    this.channel = undefined;
    this.broadcastState("closed");
  }

  send(payload: string): void {
    if (!this.channel) {
      throw new Error("Broadcast channel is not open");
    }
    this.channel.postMessage(payload);
  }
}

export const createTransport = (
  mode: "native" | "wasm",
): DebuggerTransport => {
  if (mode === "native") {
    return new WebSocketTransport();
  }
  return new BroadcastChannelTransport();
};
