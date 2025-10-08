import { create } from "zustand";
import type {
  BreakpointDescriptor,
  CallstackFrame,
  DebuggerShape,
  DebuggerTick,
  DeviceNodeDescriptor,
  FrameLogEntry,
  RegisterValue,
  RpcError,
  ThreadInfo,
  WatchDescriptor,
  WaveformChunk,
} from "../lib/debuggerSchema";
import type { DebuggerClient } from "../services/debuggerClient";
import { useSessionStore } from "./sessionStore";

interface DebuggerDataState {
  initialized: boolean;
  client?: DebuggerClient;
  shape?: DebuggerShape;
  latestTick?: DebuggerTick;
  waveform?: WaveformChunk | null;
  errorMessage?: string;
  breakpointHit?: { breakpoint: BreakpointDescriptor; timestamp: number };
  notificationUnsub?: () => void;
  // Computed accessors from tick
  deviceTree: DeviceNodeDescriptor[];
  registersByPath: Record<string, RegisterValue[]>;
  availableEvents: string[];
  breakpoints: BreakpointDescriptor[];
  threads: ThreadInfo[];
  frameLog: FrameLogEntry[];
  executionState: { state: "running" | "paused"; breakpointId?: string };
  watches: WatchDescriptor[];
  callstacks: Record<string, CallstackFrame[]>;
  // Methods
  initialize: (client: DebuggerClient) => Promise<void>;
  reset: () => void;
  addWatch: (expression: string) => Promise<void>;
  removeWatch: (watchId: string) => Promise<void>;
  editWatch: (watchId: string, value: string) => Promise<void>;
  modifyWatchExpression: (watchId: string, newExpression: string) => Promise<void>;
  addBreakpoint: (location: string, kind?: BreakpointDescriptor["kind"]) => Promise<void>;
  removeBreakpoint: (id: string) => Promise<void>;
  toggleBreakpoint: (id: string, enabled: boolean) => Promise<void>;
  showError: (message: string) => void;
  clearError: () => void;
}
// Helper function to collect all events from device tree
const collectEventsFromTree = (nodes: DeviceNodeDescriptor[]): string[] => {
  const events: string[] = [];
  for (const node of nodes) {
    if (node.events) {
      events.push(...node.events);
    }
    if (node.children) {
      events.push(...collectEventsFromTree(node.children));
    }
  }
  return events;
};

export const useDebuggerDataStore = create<DebuggerDataState>()((set, get) => ({
  initialized: false,
  deviceTree: [],
  registersByPath: {},
  availableEvents: [],
  breakpoints: [],
  threads: [],
  frameLog: [],
  executionState: { state: "paused" },
  watches: [],
  waveform: null,
  callstacks: {},
  async initialize(client) {
    const { notificationUnsub } = get();
    notificationUnsub?.();
    set({
      client,
      initialized: false,
      shape: undefined,
      latestTick: undefined,
      deviceTree: [],
      breakpoints: [],
      threads: [],
      frameLog: [],
      executionState: { state: "paused" },
      watches: [],
    });
    try {
      // Fetch shape (device tree structure, capabilities, emulator info)
      const shape = await client.describe();
      const events = collectEventsFromTree(shape.deviceTree);
      set({
        shape,
        deviceTree: shape.deviceTree,
        availableEvents: events,
      });

      // Set up tick notification handler
      const unsub = client.onNotification((notification) => {
        switch (notification.topic) {
          case "tick": {
            const tick = notification.payload as DebuggerTick;
            const breakpointList = Object.values(tick.breakpoints);

            set({
              latestTick: tick,
              registersByPath: tick.registers,
              breakpoints: breakpointList,
              frameLog: tick.eventLog,
              executionState: tick.executionState,
              threads: tick.threads ?? [],
              watches: tick.watches ?? [],
              callstacks: tick.callstacks ?? get().callstacks,
            });

            // Update session store execution state
            useSessionStore.getState().setExecutionState(tick.executionState.state);

            // Show notification if breakpoint was hit
            if (tick.executionState.breakpointId && tick.executionState.state === "paused") {
              const breakpoint = tick.breakpoints[tick.executionState.breakpointId];
              if (breakpoint) {
                set({
                  breakpointHit: {
                    breakpoint,
                    timestamp: Date.now(),
                  },
                });
                // Clear the notification after 4 seconds
                setTimeout(() => {
                  set({ breakpointHit: undefined });
                }, 4000);
              }
            }
            break;
          }
          case "stream.waveform": {
            set({ waveform: notification.payload as WaveformChunk });
            break;
          }
          default:
            break;
        }
      });

      set({ initialized: true, notificationUnsub: unsub });
    } catch (error) {
      console.error("Failed to initialize debugger data", error);
      // On initialization failure, clear everything since we never successfully connected
      const { notificationUnsub } = get();
      notificationUnsub?.();
      set({
        initialized: false,
        client: undefined,
        shape: undefined,
        latestTick: undefined,
        deviceTree: [],
        registersByPath: {},
        availableEvents: [],
        watches: [],
        breakpoints: [],
        threads: [],
        frameLog: [],
        executionState: { state: "paused" },
        waveform: null,
        callstacks: {},
        notificationUnsub: undefined,
        errorMessage: undefined,
      });
    }
  },
  reset() {
    const { notificationUnsub } = get();
    notificationUnsub?.();
    // Only clear client and subscription, preserve all data and initialized state
    set({
      client: undefined,
      notificationUnsub: undefined,
    });
  },
  async addWatch(expression) {
    const trimmed = expression.trim();
    if (!trimmed) {
      return;
    }
    const { client, watches } = get();
    if (!client || watches.some((w) => w.expression === trimmed)) {
      return;
    }
    try {
      const result = await client.watch([trimmed]) as RpcError;
      if (result.error) {
        get().showError(result.error.message);
      }
    } catch (error) {
      console.error("Failed to add watch", error);
      get().showError(error instanceof Error ? error.message : "Failed to add watch");
    }
  },
  async removeWatch(watchId) {
    const { client, watches } = get();
    const watch = watches.find((w) => w.id === watchId);
    if (!client || !watch) {
      return;
    }
    try {
      const result = await client.unwatch([watchId]) as RpcError;
      if (result.error) {
        get().showError(result.error.message);
      }
    } catch (error) {
      console.error("Failed to remove watch", error);
      get().showError(error instanceof Error ? error.message : "Failed to remove watch");
    }
  },
  async editWatch(watchId, value) {
    const { client, watches } = get();
    if (!client) {
      const error = new Error("No client connected");
      get().showError(error.message);
      throw error;
    }
    const watch = watches.find((w) => w.id === watchId);
    if (!watch) {
      const error = new Error("Watch not found");
      get().showError(error.message);
      throw error;
    }
    try {
      const result = await client.editWatch(watchId, value) as RpcError;
      if (result.error) {
        get().showError(result.error.message);
        throw new Error(result.error.message);
      }
    } catch (error) {
      console.error("Failed to edit watch", error);
      get().showError(error instanceof Error ? error.message : "Failed to edit watch");
      throw error;
    }
  },
  async modifyWatchExpression(watchId, newExpression) {
    const { client, watches } = get();
    if (!client) {
      const error = new Error("No client connected");
      get().showError(error.message);
      throw error;
    }
    const watch = watches.find((w) => w.id === watchId);
    if (!watch) {
      const error = new Error("Watch not found");
      get().showError(error.message);
      throw error;
    }
    try {
      const result = await client.modifyWatchExpression(watchId, newExpression) as RpcError;
      if (result.error) {
        get().showError(result.error.message);
        throw new Error(result.error.message);
      }
    } catch (error) {
      console.error("Failed to modify watch expression", error);
      get().showError(error instanceof Error ? error.message : "Failed to modify watch expression");
      throw error;
    }
  },
  async addBreakpoint(location, kind = "code") {
    const trimmed = location.trim();
    if (!trimmed) {
      return;
    }
    const { client } = get();
    if (!client) {
      return;
    }
    try {
      const result = await client.addBreakpoint(trimmed, kind) as RpcError;
      if (result.error) {
        get().showError(result.error.message);
      }
    } catch (error) {
      console.error("Failed to add breakpoint", error);
      get().showError(error instanceof Error ? error.message : "Failed to add breakpoint");
    }
  },
  async removeBreakpoint(id) {
    const { client } = get();
    if (!client) {
      return;
    }
    try {
      const result = await client.removeBreakpoint(id) as RpcError;
      if (result.error) {
        get().showError(result.error.message);
      }
    } catch (error) {
      console.error("Failed to remove breakpoint", error);
      get().showError(error instanceof Error ? error.message : "Failed to remove breakpoint");
    }
  },
  async toggleBreakpoint(id, enabled) {
    const { client } = get();
    if (!client) {
      return;
    }
    try {
      const result = await client.toggleBreakpoint(id, enabled) as RpcError;
      if (result.error) {
        get().showError(result.error.message);
      }
    } catch (error) {
      console.error("Failed to toggle breakpoint", error);
      get().showError(error instanceof Error ? error.message : "Failed to toggle breakpoint");
    }
  },
  showError(message) {
    set({ errorMessage: message });
    setTimeout(() => {
      set({ errorMessage: undefined });
    }, 5000);
  },
  clearError() {
    set({ errorMessage: undefined });
  },
}));


