import { create } from "zustand";
import { appConfig, resolveEndpoint, resolveTransportOptions } from "../config";
import { DebuggerClient } from "../services/debuggerClient";

export type ConnectionState = "idle" | "connecting" | "connected" | "error";
export type ExecutionState = "running" | "paused";

interface SessionStore {
  client?: DebuggerClient;
  mode: "native" | "wasm";
  endpoint?: string;
  connectionState: ConnectionState;
  connectionError?: string;
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

    // Subscribe to transport state changes to keep connectionState in sync
    client.onTransportStateChange((state, event) => {
      console.log("Transport state changed:", state, event);

      if (state === "closed") {
        const currentClient = get().client;
        // Only update if this is still the active client
        if (currentClient === client) {
          set({
            connectionState: "error",
            connectionError: "Connection closed",
          });
        }
      } else if (state === "open") {
        const currentClient = get().client;
        if (currentClient === client) {
          set({
            connectionState: "connected",
          });
        }
      }
    });

    set({
      connectionState: "connecting",
      connectionError: undefined,
      endpoint,
      client,
    });

    try {
      await client.connect();
      set({
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
    });
  },
}));
