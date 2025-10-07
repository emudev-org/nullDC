import { create } from "zustand";
import type {
  BreakpointDescriptor,
  DeviceNodeDescriptor,
  FrameLogEntry,
  RegisterValue,
  ThreadInfo,
  WaveformChunk,
} from "../lib/debuggerSchema";
import type { DebuggerClient } from "../services/debuggerClient";
import { useSessionStore } from "./sessionStore";
const DEFAULT_WATCH_EXPRESSIONS = ["dc.sh4.cpu.pc", "dc.sh4.dmac.dmaor"] as const;
const FRAME_LOG_LIMIT = 200;
interface RegistersByPath {
  [path: string]: RegisterValue[];
}

interface DebuggerDataState {
  initialized: boolean;
  client?: DebuggerClient;
  deviceTree: DeviceNodeDescriptor[];
  registersByPath: RegistersByPath;
  availableEvents: string[];
  watchExpressions: string[];
  watchValues: Record<string, unknown>;
  breakpoints: BreakpointDescriptor[];
  threads: ThreadInfo[];
  frameLog: FrameLogEntry[];
  waveform?: WaveformChunk | null;
  breakpointHit?: { breakpoint: BreakpointDescriptor; timestamp: number };
  notificationUnsub?: () => void;
  initialize: (client: DebuggerClient) => Promise<void>;
  reset: () => void;
  addWatch: (expression: string) => Promise<void>;
  removeWatch: (expression: string) => Promise<void>;
  addBreakpoint: (location: string, kind?: BreakpointDescriptor["kind"]) => Promise<void>;
  removeBreakpoint: (id: string) => Promise<void>;
  toggleBreakpoint: (id: string, enabled: boolean) => Promise<void>;
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
  watchExpressions: [],
  watchValues: {},
  breakpoints: [],
  threads: [],
  frameLog: [],
  waveform: null,
  async initialize(client) {
    const { notificationUnsub } = get();
    notificationUnsub?.();
    set({
      client,
      initialized: false,
      deviceTree: [],
      breakpoints: [],
      threads: [],
      frameLog: [],
    });
    try {
      const describe = await client.describe(["devices", "breakpoints", "threads"]);
      const devices = describe.devices ?? [];
      const events = collectEventsFromTree(devices);
      set({
        deviceTree: devices,
        availableEvents: events,
        breakpoints: describe.breakpoints ?? [],
        threads: describe.threads ?? [],
      });
      const registers = await client.fetchRegisters("dc.sh4.cpu");
      set((state) => ({
        registersByPath: {
          ...state.registersByPath,
          [registers.path]: registers.registers,
        },
      }));
      const frameLog = await client.fetchFrameLog(0, 64);
      set({ frameLog: frameLog.entries });
      const subscriptionTopics = [
        "state.registers",
        "state.watch",
        "state.breakpoint",
        "state.thread",
        "state.execution",
        "stream.waveform",
        "stream.frameLog",
      ];
      await client.subscribe(subscriptionTopics);
      if (DEFAULT_WATCH_EXPRESSIONS.length > 0) {
        const defaults = Array.from(DEFAULT_WATCH_EXPRESSIONS);
        const result = await client.watch(defaults);
        set((state) => ({
          watchExpressions: result.all,
          watchValues: {
            ...state.watchValues,
            ...Object.fromEntries(result.all.map((expr) => [expr, state.watchValues[expr] ?? null])),
          },
        }));
      }
      const unsub = client.onNotification((notification) => {
        switch (notification.topic) {
          case "state.registers": {
            const { path, registers } = notification.payload as { path: string; registers: RegisterValue[] };
            set((state) => ({
              registersByPath: {
                ...state.registersByPath,
                [path]: registers,
              },
            }));
            break;
          }
          case "state.watch": {
            const { expression, value } = notification.payload as { expression: string; value: unknown };
            set((state) => ({
              watchValues: {
                ...state.watchValues,
                [expression]: value,
              },
            }));
            break;
          }
          case "state.breakpoint": {
            const breakpoint = notification.payload as BreakpointDescriptor;
            set((state) => {
              const existingIndex = state.breakpoints.findIndex((bp) => bp.id === breakpoint.id);
              if (existingIndex >= 0) {
                const updated = state.breakpoints.slice();
                updated[existingIndex] = breakpoint;
                return { breakpoints: updated };
              }
              return { breakpoints: [...state.breakpoints, breakpoint] };
            });
            break;
          }
          case "state.thread": {
            const thread = notification.payload as ThreadInfo;
            set((state) => {
              const existingIndex = state.threads.findIndex((t) => t.id === thread.id);
              if (existingIndex >= 0) {
                const updated = state.threads.slice();
                updated[existingIndex] = thread;
                return { threads: updated };
              }
              return { threads: [...state.threads, thread] };
            });
            break;
          }
          case "stream.waveform": {
            set({ waveform: notification.payload as WaveformChunk });
            break;
          }
          case "stream.frameLog": {
            const entry = notification.payload as FrameLogEntry;
            set((state) => {
              const next = [...state.frameLog, entry];
              if (next.length > FRAME_LOG_LIMIT) {
                next.splice(0, next.length - FRAME_LOG_LIMIT);
              }
              return { frameLog: next };
            });
            break;
          }
          case "state.execution": {
            const { state: execState, breakpoint } = notification.payload as { state: "running" | "paused"; breakpoint?: BreakpointDescriptor };
            useSessionStore.getState().setExecutionState(execState);

            // Show notification if breakpoint was hit
            if (breakpoint && execState === "paused") {
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
            break;
          }
          default:
            break;
        }
      });
      set({ initialized: true, notificationUnsub: unsub });
    } catch (error) {
      console.error("Failed to initialize debugger data", error);
      get().reset();
    }
  },
  reset() {
    const { notificationUnsub, client } = get();
    notificationUnsub?.();
    if (client) {
      void client
        .unsubscribe([
          "state.registers",
          "state.watch",
          "state.breakpoint",
          "state.thread",
          "state.execution",
          "stream.waveform",
          "stream.frameLog",
        ])
        .catch(() => {});
    }
    set({
      initialized: false,
      client: undefined,
      deviceTree: [],
      registersByPath: {},
      availableEvents: [],
      watchExpressions: [],
      watchValues: {},
      breakpoints: [],
      threads: [],
      frameLog: [],
      waveform: null,
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
      const result = await client.watch([trimmed]);
      if (result.accepted.includes(trimmed)) {
        set((state) => ({
          watchExpressions: result.all,
          watchValues: {
            ...state.watchValues,
            ...Object.fromEntries(result.all.map((expr) => [expr, state.watchValues[expr] ?? null])),
          },
        }));
      }
    } catch (error) {
      console.error("Failed to add watch", error);
    }
  },
  async removeWatch(expression) {
    const { client, watchExpressions } = get();
    if (!client || !watchExpressions.includes(expression)) {
      return;
    }
    try {
      const result = await client.unwatch([expression]);
      if (result.accepted.includes(expression)) {
        set((state) => {
          const newValues: Record<string, unknown> = {};
          for (const expr of result.all) {
            newValues[expr] = state.watchValues[expr] ?? null;
          }
          return {
            watchExpressions: result.all,
            watchValues: newValues,
          };
        });
      }
    } catch (error) {
      console.error("Failed to remove watch", error);
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
      const result = await client.addBreakpoint(trimmed, kind);
      if (result.all) {
        set({ breakpoints: result.all });
      }
    } catch (error) {
      console.error("Failed to add breakpoint", error);
    }
  },
  async removeBreakpoint(id) {
    const { client } = get();
    if (!client) {
      return;
    }
    try {
      const result = await client.removeBreakpoint(id);
      if (result.all) {
        set({ breakpoints: result.all });
      }
    } catch (error) {
      console.error("Failed to remove breakpoint", error);
    }
  },
  async toggleBreakpoint(id, enabled) {
    const { client } = get();
    if (!client) {
      return;
    }
    try {
      const result = await client.toggleBreakpoint(id, enabled);
      if (result.all) {
        set({ breakpoints: result.all });
      }
    } catch (error) {
      console.error("Failed to toggle breakpoint", error);
    }
  },
}));


