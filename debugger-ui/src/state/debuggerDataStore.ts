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
  watchExpressions: string[];
  watchValues: Record<string, unknown>;
  breakpoints: BreakpointDescriptor[];
  threads: ThreadInfo[];
  frameLog: FrameLogEntry[];
  waveform?: WaveformChunk | null;
  notificationUnsub?: () => void;
  initialize: (client: DebuggerClient) => Promise<void>;
  reset: () => void;
  addWatch: (expression: string) => Promise<void>;
  removeWatch: (expression: string) => Promise<void>;
}

export const useDebuggerDataStore = create<DebuggerDataState>()((set, get) => ({
  initialized: false,
  deviceTree: [],
  registersByPath: {},
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
      set({
        deviceTree: describe.devices ?? [],
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
        "stream.waveform",
        "stream.frameLog",
      ];
      await client.subscribe(subscriptionTopics);

      if (DEFAULT_WATCH_EXPRESSIONS.length > 0) {
        const defaults = Array.from(DEFAULT_WATCH_EXPRESSIONS);
        const result = await client.watch(defaults);
        set((state) => ({
          watchExpressions: result.accepted,
          watchValues: {
            ...state.watchValues,
            ...Object.fromEntries(result.accepted.map((expr) => [expr, state.watchValues[expr] ?? null])),
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
          watchExpressions: [...state.watchExpressions, trimmed],
          watchValues: { ...state.watchValues, [trimmed]: state.watchValues[trimmed] ?? null },
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
          const { [expression]: _removed, ...restValues } = state.watchValues;
          return {
            watchExpressions: state.watchExpressions.filter((expr) => expr !== expression),
            watchValues: restValues,
          };
        });
      }
    } catch (error) {
      console.error("Failed to remove watch", error);
    }
  },
}));
