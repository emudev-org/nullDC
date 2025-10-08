import { z } from "zod";
import type { RpcSchema } from "./jsonRpc";

// Zod Schemas
export const RegisterValueSchema = z.object({
  name: z.string(),
  value: z.string(),
  width: z.number(),
  flags: z.record(z.string(), z.boolean()).optional(),
  metadata: z.record(z.string(), z.unknown()).optional(),
});

export const DeviceNodeDescriptorSchema: z.ZodType<DeviceNodeDescriptor> = z.lazy(() =>
  z.object({
    path: z.string(),
    label: z.string(),
    kind: z.string(),
    description: z.string().optional(),
    registers: z.array(RegisterValueSchema).optional(),
    events: z.array(z.string()).optional(),
    children: z.array(DeviceNodeDescriptorSchema).optional(),
  })
);

export const MemorySliceSchema = z.object({
  baseAddress: z.number(),
  wordSize: z.union([z.literal(1), z.literal(2), z.literal(4), z.literal(8)]),
  encoding: z.enum(["hex", "uint", "float", "ascii"]),
  data: z.string(),
  validity: z.enum(["ok", "tlb-miss", "fault"]),
});

export const DisassemblyLineSchema = z.object({
  address: z.number(),
  bytes: z.string(),
  mnemonic: z.string(),
  operands: z.string(),
  comment: z.string().optional(),
  isCurrent: z.boolean().optional(),
  isBreakpoint: z.boolean().optional(),
});

export const BreakpointDescriptorSchema = z.object({
  id: z.string(),
  location: z.string(),
  kind: z.enum(["code", "event"]),
  enabled: z.boolean(),
  condition: z.string().optional(),
  hitCount: z.number(),
  pending: z.boolean().optional(),
});

export const BacktraceFrameSchema = z.object({
  index: z.number(),
  pc: z.number(),
  symbol: z.string().optional(),
  location: z.string().optional(),
});

export const EventLogEntrySchema = z.object({
  eventId: z.string(),
  timestamp: z.number(),
  subsystem: z.enum(["sh4", "holly", "ta", "core", "aica", "dsp"]),
  severity: z.enum(["trace", "info", "warn", "error"]),
  message: z.string(),
  metadata: z.record(z.string(), z.unknown()).optional(),
});

export const TransportSettingsSchema = z.object({
  sessionToken: z.string().optional(),
  build: z.enum(["native", "wasm"]),
});

export const CallstackFrameSchema = z.object({
  index: z.number(),
  pc: z.number(),
  sp: z.number().optional(),
  symbol: z.string().optional(),
  location: z.string().optional(),
});

export const DebuggerShapeSchema = z.object({
  emulator: z.object({
    name: z.string(),
    version: z.string(),
    build: z.enum(["native", "wasm"]),
  }),
  deviceTree: z.array(DeviceNodeDescriptorSchema),
});

export const WatchDescriptorSchema = z.object({
  id: z.string(),
  expression: z.string(),
  value: z.unknown(),
});

export const DebuggerTickSchema = z.object({
  tickId: z.number(),
  timestamp: z.number(),
  executionState: z.object({
    state: z.enum(["running", "paused"]),
    breakpointId: z.string().optional(),
  }),
  registers: z.record(z.string(), z.array(RegisterValueSchema)),
  breakpoints: z.record(z.string(), BreakpointDescriptorSchema),
  eventLog: z.array(EventLogEntrySchema),
  watches: z.array(WatchDescriptorSchema).optional(),
  callstacks: z.record(z.string(), z.array(CallstackFrameSchema)).optional(),
});

export const RpcErrorSchema = z.object({
  error: z.object({
    code: z.number(),
    message: z.string(),
  }).optional(),
});

// Type exports derived from Zod schemas
export type RegisterValue = z.infer<typeof RegisterValueSchema>;
export type DeviceNodeDescriptor = {
  path: string;
  label: string;
  kind: string;
  description?: string;
  registers?: RegisterValue[];
  events?: string[];
  children?: DeviceNodeDescriptor[];
};
export type MemorySlice = z.infer<typeof MemorySliceSchema>;
export type DisassemblyLine = z.infer<typeof DisassemblyLineSchema>;
export type BreakpointDescriptor = z.infer<typeof BreakpointDescriptorSchema>;
export type BacktraceFrame = z.infer<typeof BacktraceFrameSchema>;
export type EventLogEntry = z.infer<typeof EventLogEntrySchema>;
export type TransportSettings = z.infer<typeof TransportSettingsSchema>;
export type CallstackFrame = z.infer<typeof CallstackFrameSchema>;
export type DebuggerShape = z.infer<typeof DebuggerShapeSchema>;
export type WatchDescriptor = z.infer<typeof WatchDescriptorSchema>;
export type DebuggerTick = z.infer<typeof DebuggerTickSchema>;
export type RpcError = z.infer<typeof RpcErrorSchema>;

// RPC Method Schemas for validation
export const DebuggerRpcMethodSchemas = {
  "debugger.handshake": {
    params: z.object({
      clientName: z.string(),
      clientVersion: z.string(),
      transport: TransportSettingsSchema,
    }),
    result: z.object({
      sessionId: z.string(),
    }),
  },
  "state.getCallstack": {
    params: z.object({
      target: z.enum(["sh4", "arm7"]),
      maxFrames: z.number().optional(),
    }),
    result: z.object({
      target: z.string(),
      frames: z.array(CallstackFrameSchema),
    }),
  },
  "debugger.describe": {
    params: z.object({}),
    result: DebuggerShapeSchema,
  },
  "state.getMemorySlice": {
    params: z.object({
      target: z.string().optional(),
      address: z.number(),
      length: z.number(),
      encoding: z.enum(["hex", "uint", "float", "ascii"]).optional(),
      wordSize: z.union([z.literal(1), z.literal(2), z.literal(4), z.literal(8)]).optional(),
    }),
    result: MemorySliceSchema,
  },
  "state.getDisassembly": {
    params: z.object({
      target: z.string().optional(),
      address: z.number(),
      count: z.number(),
      context: z.number().optional(),
    }),
    result: z.object({
      lines: z.array(DisassemblyLineSchema),
    }),
  },
  "state.watch": {
    params: z.object({
      expressions: z.array(z.string()),
    }),
    result: RpcErrorSchema,
  },
  "state.unwatch": {
    params: z.object({
      expressions: z.array(z.string()),
    }),
    result: RpcErrorSchema,
  },
  "state.editWatch": {
    params: z.object({
      watchId: z.string(),
      value: z.string(),
    }),
    result: RpcErrorSchema,
  },
  "state.modifyWatchExpression": {
    params: z.object({
      watchId: z.string(),
      newExpression: z.string(),
    }),
    result: RpcErrorSchema,
  },
  "control.step": {
    params: z.object({
      target: z.string(),
    }),
    result: RpcErrorSchema,
  },
  "control.stepOver": {
    params: z.object({
      target: z.string(),
    }),
    result: RpcErrorSchema,
  },
  "control.stepOut": {
    params: z.object({
      target: z.string(),
    }),
    result: RpcErrorSchema,
  },
  "control.runUntil": {
    params: z.object({
      target: z.string(),
      type: z.enum(["interrupt", "exception", "primitive", "tile", "vertex", "list", "sample"]),
      value: z.string().optional(),
    }),
    result: RpcErrorSchema,
  },
  "control.pause": {
    params: z.object({
      target: z.string().optional(),
    }),
    result: RpcErrorSchema,
  },
  "breakpoints.add": {
    params: z.object({
      location: z.string(),
      kind: z.enum(["code", "event"]).optional(),
      enabled: z.boolean().optional(),
    }),
    result: RpcErrorSchema,
  },
  "breakpoints.setCategoryStates": {
    params: z.object({
      categories: z.record(z.string(), z.object({
        muted: z.boolean(),
        soloed: z.boolean(),
      })),
    }),
    result: RpcErrorSchema,
  },
  "breakpoints.remove": {
    params: z.object({
      id: z.string(),
    }),
    result: RpcErrorSchema,
  },
  "breakpoints.toggle": {
    params: z.object({
      id: z.string(),
      enabled: z.boolean(),
    }),
    result: RpcErrorSchema,
  },
  "event.tick": {
    params: DebuggerTickSchema,
    result: z.never(),
  },
} as const;

export type DebuggerRpcSchema = RpcSchema & {
  "debugger.handshake": {
    params: { clientName: string; clientVersion: string; transport: TransportSettings };
    result: { sessionId: string };
  };
  "state.getCallstack": {
    params: { target: "sh4" | "arm7"; maxFrames?: number };
    result: { target: string; frames: CallstackFrame[] };
  };
  "debugger.describe": {
    params: Record<string, never>;
    result: DebuggerShape;
  };
  "state.getMemorySlice": {
    params: { target?: string; address: number; length: number; encoding?: MemorySlice["encoding"]; wordSize?: MemorySlice["wordSize"]; };
    result: MemorySlice;
  };
  "state.getDisassembly": {
    params: { target?: string; address: number; count: number; context?: number };
    result: { lines: DisassemblyLine[] };
  };
  "state.watch": {
    params: { expressions: string[] };
    result: RpcError;
  };
  "state.unwatch": {
    params: { expressions: string[] }; // Array of watch IDs
    result: RpcError;
  };
  "state.editWatch": {
    params: { watchId: string; value: string };
    result: RpcError;
  };
  "state.modifyWatchExpression": {
    params: { watchId: string; newExpression: string };
    result: RpcError;
  };
  "control.step": {
    params: { target: string };
    result: RpcError;
  };
  "control.stepOver": {
    params: { target: string };
    result: RpcError;
  };
  "control.stepOut": {
    params: { target: string };
    result: RpcError;
  };
  "control.runUntil": {
    params: { target: string; type: "interrupt" | "exception" | "primitive" | "tile" | "vertex" | "list" | "sample"; value?: string };
    result: RpcError;
  };
  "control.pause": {
    params: { target?: string };
    result: RpcError;
  };
  "breakpoints.add": {
    params: { location: string; kind?: BreakpointDescriptor["kind"]; enabled?: boolean };
    result: RpcError;
  };
  "breakpoints.setCategoryStates": {
    params: { categories: Record<string, { muted: boolean; soloed: boolean }> };
    result: RpcError;
  };
  "breakpoints.remove": {
    params: { id: string };
    result: RpcError;
  };
  "breakpoints.toggle": {
    params: { id: string; enabled: boolean };
    result: RpcError;
  };
};

export const DebuggerNotificationSchema = z.object({
  topic: z.literal("tick"),
  payload: DebuggerTickSchema,
});

export type DebuggerNotification = z.infer<typeof DebuggerNotificationSchema>;


