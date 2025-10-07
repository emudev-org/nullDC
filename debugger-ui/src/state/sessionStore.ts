import { create } from "zustand";
import { appConfig, resolveEndpoint, resolveTransportOptions } from "../config";
import { DebuggerClient } from "../services/debuggerClient";

export type ConnectionState = "idle" | "connecting" | "connected" | "error";
export type ExecutionState = "running" | "paused";

export interface SessionInfo {
  sessionId: string;
  capabilities: string[];
}

interface SessionStore {
  client?: DebuggerClient;
  mode: "native" | "wasm";
  endpoint?: string;
  connectionState: ConnectionState;
  connectionError?: string;
  session?: SessionInfo;
  executionState: ExecutionState;
  setExecutionState: (state: ExecutionState) => void;
  connect: (options?: { force?: boolean }) => Promise<void>;
  disconnect: () => void;
}

export const useSessionStore = create<SessionStore>()((set, get) => ({
  mode: appConfig.mode,
  connectionState: "idle",
  executionState: "running",
  setExecutionState(state) {
    set({ executionState: state });
  },
  async connect({ force } = {}) {
    const current = get();
    if (!force && (current.connectionState === "connecting" || current.connectionState === "connected")) {
      return;
    }

    current.client?.disconnect();

    const endpoint = resolveEndpoint();
    const transportOptions = resolveTransportOptions();

    const client = new DebuggerClient({
      mode: appConfig.mode,
      endpoint,
      transportOptions,
    });

    set({
      connectionState: "connecting",
      connectionError: undefined,
      endpoint,
      client,
      session: undefined,
    });

    try {
      await client.connect();
      const handshake = await client.handshake(appConfig.clientName, appConfig.clientVersion, appConfig.mode);
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
}));
