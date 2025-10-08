import { z } from "zod";
import type { RpcSchema } from "./jsonRpc";

// Common enums and ID types
export const TargetProcessorSchema = z.enum(["sh4", "arm7", "dsp"]);

// ID schemas - using integers for efficiency
export const BreakpointIdSchema = z.number().int().nonnegative();
export const WatchIdSchema = z.number().int().nonnegative();

export const RpcMethodNameSchema = z.enum([
  "debugger.describe",
  "state.getCallstack",
  "state.getMemorySlice",
  "state.getDisassembly",
  "state.watch",
  "state.unwatch",
  "state.editWatch",
  "state.modifyWatchExpression",
  "control.step",
  "control.stepOver",
  "control.stepOut",
  "control.runUntil",
  "control.pause",
  "breakpoints.add",
  "breakpoints.remove",
  "breakpoints.toggle",
  "breakpoints.setCategoryStates",
  "event.tick",
]);

// Enum-style constants for method names (for better discoverability and refactoring)
export const RpcMethod = {
  DEBUGGER_DESCRIBE: "debugger.describe",
  STATE_GET_CALLSTACK: "state.getCallstack",
  STATE_GET_MEMORY_SLICE: "state.getMemorySlice",
  STATE_GET_DISASSEMBLY: "state.getDisassembly",
  STATE_WATCH: "state.watch",
  STATE_UNWATCH: "state.unwatch",
  STATE_EDIT_WATCH: "state.editWatch",
  STATE_MODIFY_WATCH_EXPRESSION: "state.modifyWatchExpression",
  CONTROL_STEP: "control.step",
  CONTROL_STEP_OVER: "control.stepOver",
  CONTROL_STEP_OUT: "control.stepOut",
  CONTROL_RUN_UNTIL: "control.runUntil",
  CONTROL_PAUSE: "control.pause",
  BREAKPOINTS_ADD: "breakpoints.add",
  BREAKPOINTS_REMOVE: "breakpoints.remove",
  BREAKPOINTS_TOGGLE: "breakpoints.toggle",
  BREAKPOINTS_SET_CATEGORY_STATES: "breakpoints.setCategoryStates",
  EVENT_TICK: "event.tick",
} as const satisfies Record<string, RpcMethodName>;

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
  data: z.array(z.number()), // Byte array
  validity: z.enum(["ok", "cache-miss", "tlb-miss", "fault"]),
});

export const DisassemblyLineSchema = z.object({
  address: z.number(),
  bytes: z.string(),
  disassembly: z.string(),
});

export const BreakpointCategorySchema = z.enum(["events", "sh4", "arm7", "dsp"]);

export const BreakpointCategoryStateSchema = z.object({
  muted: z.boolean(),
  soloed: z.boolean(),
});

export const BreakpointDescriptorSchema = z.object({
  id: BreakpointIdSchema,
  event: z.string(), // For code: "dc.sh4.cpu.pc", "dc.aica.dsp.step", etc. For events: "dc.holly.ta.list_end", etc.
  address: z.number().optional(), // For code breakpoints: the PC/step value. Undefined for event breakpoints.
  kind: z.enum(["code", "event"]),
  enabled: z.boolean(),
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
  id: WatchIdSchema,
  expression: z.string(),
  value: z.unknown(),
});

export const DebuggerTickSchema = z.object({
  tickId: z.number(),
  timestamp: z.number(),
  executionState: z.object({
    state: z.enum(["running", "paused"]),
    breakpointId: BreakpointIdSchema.optional(),
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
export type TargetProcessor = z.infer<typeof TargetProcessorSchema>;
export type RpcMethodName = z.infer<typeof RpcMethodNameSchema>;
export type BreakpointId = z.infer<typeof BreakpointIdSchema>;
export type WatchId = z.infer<typeof WatchIdSchema>;
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
export type BreakpointCategory = z.infer<typeof BreakpointCategorySchema>;
export type BreakpointCategoryState = z.infer<typeof BreakpointCategoryStateSchema>;
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
  "state.getCallstack": {
    params: z.object({
      target: z.enum(["sh4", "arm7"]),
      maxFrames: z.number().optional(),
    }),
    result: z.object({
      target: TargetProcessorSchema,
      frames: z.array(CallstackFrameSchema),
    }),
  },
  "debugger.describe": {
    params: z.object({}),
    result: DebuggerShapeSchema,
  },
  "state.getMemorySlice": {
    params: z.object({
      target: TargetProcessorSchema.optional(),
      address: z.number(),
      length: z.number(),
    }),
    result: MemorySliceSchema,
  },
  "state.getDisassembly": {
    params: z.object({
      target: TargetProcessorSchema.optional(),
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
      watchIds: z.array(WatchIdSchema),
    }),
    result: RpcErrorSchema,
  },
  "state.editWatch": {
    params: z.object({
      watchId: WatchIdSchema,
      value: z.string(),
    }),
    result: RpcErrorSchema,
  },
  "state.modifyWatchExpression": {
    params: z.object({
      watchId: WatchIdSchema,
      newExpression: z.string(),
    }),
    result: RpcErrorSchema,
  },
  "control.step": {
    params: z.object({
      target: TargetProcessorSchema,
    }),
    result: RpcErrorSchema,
  },
  "control.stepOver": {
    params: z.object({
      target: TargetProcessorSchema,
    }),
    result: RpcErrorSchema,
  },
  "control.stepOut": {
    params: z.object({
      target: TargetProcessorSchema,
    }),
    result: RpcErrorSchema,
  },
  "control.runUntil": {
    params: z.object({}),
    result: RpcErrorSchema,
  },
  "control.pause": {
    params: z.object({
      target: TargetProcessorSchema.optional(),
    }),
    result: RpcErrorSchema,
  },
  "breakpoints.add": {
    params: z.object({
      event: z.string(),
      address: z.number().optional(),
      kind: z.enum(["code", "event"]).optional(),
      enabled: z.boolean().optional(),
    }),
    result: RpcErrorSchema,
  },
  "breakpoints.setCategoryStates": {
    params: z.object({
      categories: z.record(z.string(), BreakpointCategoryStateSchema),
    }),
    result: RpcErrorSchema,
  },
  "breakpoints.remove": {
    params: z.object({
      id: BreakpointIdSchema,
    }),
    result: RpcErrorSchema,
  },
  "breakpoints.toggle": {
    params: z.object({
      id: BreakpointIdSchema,
      enabled: z.boolean(),
    }),
    result: RpcErrorSchema,
  },
  "event.tick": {
    params: DebuggerTickSchema,
    result: z.never(),
  },
} as const;

// Helper type to infer TypeScript types from Zod RPC schemas
type InferRpcMethod<T extends { params: z.ZodType; result: z.ZodType }> = {
  params: z.infer<T["params"]>;
  result: z.infer<T["result"]>;
};

type InferRpcSchema<T extends Record<string, { params: z.ZodType; result: z.ZodType }>> = {
  [K in keyof T]: InferRpcMethod<T[K]>;
};

export type DebuggerRpcSchema = RpcSchema & InferRpcSchema<typeof DebuggerRpcMethodSchemas>;

export const DebuggerNotificationSchema = z.object({
  topic: z.literal("tick"),
  payload: DebuggerTickSchema,
});

export type DebuggerNotification = z.infer<typeof DebuggerNotificationSchema>;


