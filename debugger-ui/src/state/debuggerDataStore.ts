import { create } from "zustand";
import type {
  BreakpointDescriptor,
  DebuggerShape,
  DebuggerTick,
  DeviceNodeDescriptor,
  FrameLogEntry,
  RegisterValue,
  RpcError,
  ThreadInfo,
  WaveformChunk,
} from "../lib/debuggerSchema";
import type { DebuggerClient } from "../services/debuggerClient";
import { useSessionStore } from "./sessionStore";
const DEFAULT_WATCH_EXPRESSIONS = ["dc.sh4.cpu.pc", "dc.sh4.dmac.dmaor"] as const;

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
  watchExpressions: string[];
  watchValues: Record<string, unknown>;
  // Methods
  initialize: (client: DebuggerClient) => Promise<void>;
  reset: () => void;
  addWatch: (expression: string) => Promise<void>;
  removeWatch: (expression: string) => Promise<void>;
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
  watchExpressions: [],
  watchValues: {},
  waveform: null,
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
      watchExpressions: [],
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

      // Add default watches
      if (DEFAULT_WATCH_EXPRESSIONS.length > 0) {
        const defaults = Array.from(DEFAULT_WATCH_EXPRESSIONS);
        await client.watch(defaults);
        set({ watchExpressions: defaults });
      }

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
              watchExpressions: tick.watches ? Object.keys(tick.watches) : [],
              watchValues: tick.watches ?? {},
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
        watchExpressions: [],
        watchValues: {},
        breakpoints: [],
        threads: [],
        frameLog: [],
        executionState: { state: "paused" },
        waveform: null,
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
    const { client, watchExpressions } = get();
    if (!client || watchExpressions.includes(trimmed)) {
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
  async removeWatch(expression) {
    const { client, watchExpressions } = get();
    if (!client || !watchExpressions.includes(expression)) {
      return;
    }
    try {
      const result = await client.unwatch([expression]) as RpcError;
      if (result.error) {
        get().showError(result.error.message);
      }
    } catch (error) {
      console.error("Failed to remove watch", error);
      get().showError(error instanceof Error ? error.message : "Failed to remove watch");
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


