export type TransportState = "idle" | "connecting" | "open" | "closed";

export type TransportMessageHandler = (payload: string) => void;
export type TransportStateHandler = (state: TransportState, event?: Event | Error) => void;

export interface TransportOptions {
  protocols?: string | string[];
  channelName?: string;
}

export interface AvailableConnection {
  id: string;           // GUID for broadcast, URL for websocket
  name: string;         // Display name
  mode: "native" | "wasm";
  lastSeen: number;     // Timestamp
}

export type ConnectionsChangedHandler = (connections: AvailableConnection[]) => void;

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
      throw new Error("Not Connected");
    }

    this.socket.send(payload);
  }
}

export class BroadcastChannelTransport extends BaseTransport {
  public readonly kind = "broadcast" as const;
  private channel?: BroadcastChannel;
  private pingInterval?: number;
  private pongTimeout?: number;
  private readonly PING_INTERVAL_MS = 1000;
  private readonly PONG_TIMEOUT_MS = 3000;

  async connect(endpoint: string, _options?: TransportOptions): Promise<void> {
    if (typeof BroadcastChannel === "undefined") {
      throw new Error("BroadcastChannel is not supported in this environment");
    }

    if (this.channel) {
      return;
    }

    this.state = "connecting";
    // endpoint is the GUID for broadcast channel mode
    const channelName = `nulldc-debugger-${endpoint}`;
    const channel = new BroadcastChannel(channelName);
    this.channel = channel;

    channel.addEventListener("message", (event) => {
      if (typeof event.data === "string") {
        // Handle ping/pong messages
        if (event.data === "pong") {
          if (this.pongTimeout) {
            clearTimeout(this.pongTimeout);
            this.pongTimeout = undefined;
          }
          return;
        }
        this.broadcastMessage(event.data);
      }
    });

    channel.addEventListener("messageerror", (event) => {
      this.broadcastState("closed", event);
    });

    this.broadcastState("open");

    // Start ping/pong heartbeat
    this.startHeartbeat();
  }

  disconnect(): void {
    this.stopHeartbeat();

    if (!this.channel) {
      return;
    }
    this.channel.close();
    this.channel = undefined;
    this.broadcastState("closed");
  }

  send(payload: string): void {
    if (!this.channel) {
      throw new Error("Not Connected");
    }
    this.channel.postMessage(payload);
  }

  private startHeartbeat(): void {
    // Send ping every second
    this.pingInterval = window.setInterval(() => {
      if (this.channel) {
        this.channel.postMessage("ping");

        // Expect pong within 3 seconds
        this.pongTimeout = window.setTimeout(() => {
          console.warn("Pong timeout - connection lost");
          this.broadcastState("closed");
          this.disconnect();
        }, this.PONG_TIMEOUT_MS);
      }
    }, this.PING_INTERVAL_MS);
  }

  private stopHeartbeat(): void {
    if (this.pingInterval) {
      clearInterval(this.pingInterval);
      this.pingInterval = undefined;
    }
    if (this.pongTimeout) {
      clearTimeout(this.pongTimeout);
      this.pongTimeout = undefined;
    }
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

// Connection discovery for both websocket and broadcast channel modes
export class ConnectionDiscovery {
  private mode: "native" | "wasm";
  private connections = new Map<string, AvailableConnection>();
  private announcementChannel?: BroadcastChannel;
  private cleanupInterval?: number;
  private handlers = new Set<ConnectionsChangedHandler>();
  private readonly EXPIRY_MS = 4000;
  private readonly CLEANUP_INTERVAL_MS = 1000;

  constructor(mode: "native" | "wasm") {
    this.mode = mode;
  }

  start(): void {
    if (this.mode === "wasm") {
      this.startBroadcastDiscovery();
    } else {
      this.startWebSocketDiscovery();
    }
  }

  stop(): void {
    if (this.announcementChannel) {
      this.announcementChannel.close();
      this.announcementChannel = undefined;
    }
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = undefined;
    }
    this.connections.clear();
  }

  getAvailableConnections(): AvailableConnection[] {
    return Array.from(this.connections.values());
  }

  onConnectionsChanged(handler: ConnectionsChangedHandler): () => void {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }

  private startBroadcastDiscovery(): void {
    if (typeof BroadcastChannel === "undefined") {
      console.warn("BroadcastChannel not supported");
      return;
    }

    // Listen for announcements
    this.announcementChannel = new BroadcastChannel("nulldc-debugger-announce");
    this.announcementChannel.addEventListener("message", (event) => {
      try {
        // Handle both JSON string and object formats
        let announcement: { id: string; name: string; timestamp: number };
        if (typeof event.data === "string") {
          announcement = JSON.parse(event.data);
        } else {
          announcement = event.data as { id: string; name: string; timestamp: number };
        }

        if (announcement.id && announcement.name) {
          const connection: AvailableConnection = {
            id: announcement.id,
            name: announcement.name,
            mode: "wasm",
            lastSeen: Date.now(),
          };
          const isNew = !this.connections.has(announcement.id);
          this.connections.set(announcement.id, connection);
          if (isNew) {
            this.notifyHandlers();
          }
        }
      } catch (error) {
        console.error("Failed to parse announcement", error);
      }
    });

    // Cleanup expired connections
    this.cleanupInterval = window.setInterval(() => {
      const now = Date.now();
      let changed = false;
      for (const [id, conn] of this.connections.entries()) {
        if (now - conn.lastSeen > this.EXPIRY_MS) {
          this.connections.delete(id);
          changed = true;
        }
      }
      if (changed) {
        this.notifyHandlers();
      }
    }, this.CLEANUP_INTERVAL_MS);
  }

  private startWebSocketDiscovery(): void {
    // For WebSocket mode, return current host as the only connection
    const { protocol, host } = window.location;
    const wsProtocol = protocol === "https:" ? "wss:" : "ws:";
    const url = `${wsProtocol}//${host}/ws`;

    const connection: AvailableConnection = {
      id: url,
      name: `nullDC @ ${host}`,
      mode: "native",
      lastSeen: Date.now(),
    };

    this.connections.set(url, connection);
    this.notifyHandlers();
  }

  private notifyHandlers(): void {
    const connections = this.getAvailableConnections();
    this.handlers.forEach((handler) => handler(connections));
  }
}
