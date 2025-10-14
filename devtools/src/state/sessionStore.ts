import { create } from "zustand";
import { appConfig } from "../config";
import { DebuggerClient } from "../services/debuggerClient";
import { ConnectionDiscovery, type AvailableConnection } from "../services/transport";

export type ConnectionState = "idle" | "connecting" | "connected" | "error";
export type ExecutionState = "running" | "paused";

interface SessionStore {
  client?: DebuggerClient;
  mode: "native" | "wasm";
  endpoint?: string;
  connectionState: ConnectionState;
  connectionError?: string;
  executionState: ExecutionState;
  connectionDiscovery?: ConnectionDiscovery;
  availableConnections: AvailableConnection[];
  selectedConnectionId?: string;
  showConnectionModal: boolean;
  setExecutionState: (state: ExecutionState) => void;
  connect: (options?: { force?: boolean; connectionId?: string }) => Promise<void>;
  disconnect: () => void;
  startDiscovery: () => void;
  stopDiscovery: () => void;
  setShowConnectionModal: (show: boolean) => void;
  setSelectedConnectionId: (id?: string) => void;
}

export const useSessionStore = create<SessionStore>()((set, get) => ({
  mode: appConfig.mode,
  connectionState: "idle",
  executionState: "running",
  availableConnections: [],
  showConnectionModal: false,
  setExecutionState(state) {
    set({ executionState: state });
  },
  startDiscovery() {
    const current = get();
    if (current.connectionDiscovery) {
      return; // Already started
    }

    const discovery = new ConnectionDiscovery(appConfig.mode);
    discovery.onConnectionsChanged((connections) => {
      set({ availableConnections: connections });
    });
    discovery.start();
    set({ connectionDiscovery: discovery });
  },
  stopDiscovery() {
    const current = get();
    if (current.connectionDiscovery) {
      current.connectionDiscovery.stop();
      set({ connectionDiscovery: undefined, availableConnections: [] });
    }
  },
  setShowConnectionModal(show) {
    set({ showConnectionModal: show });
  },
  setSelectedConnectionId(id) {
    set({ selectedConnectionId: id });
  },
  async connect({ force, connectionId } = {}) {
    const current = get();
    if (!force && (current.connectionState === "connecting" || current.connectionState === "connected")) {
      return;
    }

    current.client?.disconnect();

    // Determine which connection to use
    let targetConnectionId = connectionId || current.selectedConnectionId;

    // If no explicit connection specified, check available connections
    if (!targetConnectionId) {
      const connections = current.availableConnections;

      if (connections.length === 0) {
        // No connections available - show modal
        set({ showConnectionModal: true });
        return;
      } else if (connections.length === 1) {
        // Auto-connect to the only available connection
        targetConnectionId = connections[0].id;
      } else {
        // Multiple connections - show modal for selection
        set({ showConnectionModal: true });
        return;
      }
    }

    // Find the connection details
    const connection = current.availableConnections.find((c) => c.id === targetConnectionId);
    if (!connection) {
      set({
        connectionState: "error",
        connectionError: "Selected connection not found",
      });
      return;
    }

    const endpoint = connection.id;
    const transportOptions = connection.mode === "wasm" ? { channelName: endpoint } : undefined;

    const client = new DebuggerClient({
      mode: connection.mode,
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
      selectedConnectionId: targetConnectionId,
      showConnectionModal: false,
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
