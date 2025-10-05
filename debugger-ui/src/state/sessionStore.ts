import { create } from "zustand";
import { DebuggerClient } from "../services/debuggerClient";
import type { DebuggerClientConfig } from "../services/debuggerClient";

export type ConnectionState = "idle" | "connecting" | "connected" | "error";

export interface SessionInfo {
  sessionId: string;
  capabilities: string[];
}

interface ConnectPayload extends Omit<DebuggerClientConfig, "rpcOptions"> {
  clientName: string;
  clientVersion: string;
}

interface SessionStore {
  client?: DebuggerClient;
  mode: "native" | "wasm";
  endpoint?: string;
  connectionState: ConnectionState;
  connectionError?: string;
  session?: SessionInfo;
  connect: (payload: ConnectPayload) => Promise<void>;
  disconnect: () => void;
  setMode: (mode: "native" | "wasm") => void;
}

export const useSessionStore = create<SessionStore>()((set, get) => ({
  mode: "native",
  connectionState: "idle",
  async connect({ mode, endpoint, clientName, clientVersion, transportOptions }) {
    const current = get();
    current.client?.disconnect();

    const client = new DebuggerClient({
      mode,
      endpoint,
      transportOptions,
    });

    set({
      connectionState: "connecting",
      connectionError: undefined,
      mode,
      endpoint,
      client,
      session: undefined,
    });

    try {
      await client.connect();
      const handshake = await client.handshake(clientName, clientVersion, mode);
      set({
        session: handshake,
        connectionState: "connected",
      });
    } catch (error) {
      console.error("Failed to connect", error);
      client.disconnect();
      set({
        connectionState: "error",
        connectionError: error instanceof Error ? error.message : String(error),
        client: undefined,
      });
    }
  },
  disconnect() {
    const client = get().client;
    if (client) {
      client.disconnect();
    }
    set({
      connectionState: "idle",
      connectionError: undefined,
      client: undefined,
      session: undefined,
    });
  },
  setMode(mode) {
    set({ mode });
  },
}));
