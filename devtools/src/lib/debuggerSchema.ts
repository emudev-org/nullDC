import { z } from "zod";
import type { RpcSchema } from "./jsonRpc";

// Common enums and ID types
export const TargetProcessorSchema = z.enum(["sh4", "arm7", "dsp"]);

// Panel ID constants
export const PANEL_IDS = {
  DOCUMENTATION: "documentation",
  SH4_SIM: "sh4-sim",
  EVENTS: "events",
  EVENTS_BREAKPOINTS: "events-breakpoints",
  SH4_DISASSEMBLY: "sh4-disassembly",
  SH4_MEMORY: "sh4-memory",
  SH4_BREAKPOINTS: "sh4-breakpoints",
  SH4_BSC_REGISTERS: "bsc-registers",
  SH4_CCN_REGISTERS: "ccn-registers",
  SH4_CPG_REGISTERS: "cpg-registers",
  SH4_DMAC_REGISTERS: "dmac-registers",
  SH4_INTC_REGISTERS: "intc-registers",
  SH4_RTC_REGISTERS: "rtc-registers",
  SH4_SCI_REGISTERS: "sci-registers",
  SH4_SCIF_REGISTERS: "scif-registers",
  SH4_TMU_REGISTERS: "tmu-registers",
  SH4_UBC_REGISTERS: "ubc-registers",
  SH4_SQ_CONTENTS: "sq-contents",
  SH4_ICACHE_CONTENTS: "icache-contents",
  SH4_OCACHE_CONTENTS: "ocache-contents",
  SH4_OCRAM_CONTENTS: "ocram-contents",
  SH4_TLB_CONTENTS: "tlb-contents",
  ARM7_DISASSEMBLY: "arm7-disassembly",
  ARM7_MEMORY: "arm7-memory",
  ARM7_BREAKPOINTS: "arm7-breakpoints",
  CLX2_TA: "holly-ta",
  CLX2_CORE: "holly-core",
  SGC: "sgc",
  DSP_DISASSEMBLY: "dsp-disassembly",
  DSP_BREAKPOINTS: "dsp-breakpoints",
  DSP_PLAYGROUND: "dsp-playground",
  DEVICE_TREE: "device-tree",
  WATCHES: "watches",
  SH4_CALLSTACK: "sh4-callstack",
  ARM7_CALLSTACK: "arm7-callstack",
} as const;

export const PanelIdSchema = z.enum([
  PANEL_IDS.DOCUMENTATION,
  PANEL_IDS.SH4_SIM,
  PANEL_IDS.EVENTS,
  PANEL_IDS.EVENTS_BREAKPOINTS,
  PANEL_IDS.SH4_DISASSEMBLY,
  PANEL_IDS.SH4_MEMORY,
  PANEL_IDS.SH4_BREAKPOINTS,
  PANEL_IDS.SH4_BSC_REGISTERS,
  PANEL_IDS.SH4_CCN_REGISTERS,
  PANEL_IDS.SH4_CPG_REGISTERS,
  PANEL_IDS.SH4_DMAC_REGISTERS,
  PANEL_IDS.SH4_INTC_REGISTERS,
  PANEL_IDS.SH4_RTC_REGISTERS,
  PANEL_IDS.SH4_SCI_REGISTERS,
  PANEL_IDS.SH4_SCIF_REGISTERS,
  PANEL_IDS.SH4_TMU_REGISTERS,
  PANEL_IDS.SH4_UBC_REGISTERS,
  PANEL_IDS.SH4_SQ_CONTENTS,
  PANEL_IDS.SH4_ICACHE_CONTENTS,
  PANEL_IDS.SH4_OCACHE_CONTENTS,
  PANEL_IDS.SH4_OCRAM_CONTENTS,
  PANEL_IDS.SH4_TLB_CONTENTS,
  PANEL_IDS.ARM7_DISASSEMBLY,
  PANEL_IDS.ARM7_MEMORY,
  PANEL_IDS.ARM7_BREAKPOINTS,
  PANEL_IDS.CLX2_TA,
  PANEL_IDS.CLX2_CORE,
  PANEL_IDS.SGC,
  PANEL_IDS.DSP_DISASSEMBLY,
  PANEL_IDS.DSP_BREAKPOINTS,
  PANEL_IDS.DSP_PLAYGROUND,
  PANEL_IDS.DEVICE_TREE,
  PANEL_IDS.WATCHES,
  PANEL_IDS.SH4_CALLSTACK,
  PANEL_IDS.ARM7_CALLSTACK,
]);

// ID schemas - using integers for efficiency
export const BreakpointIdSchema = z.number().int().nonnegative();
export const WatchIdSchema = z.number().int().nonnegative();

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
} as const;

export const RpcMethodNameSchema = z.enum([
  RpcMethod.DEBUGGER_DESCRIBE,
  RpcMethod.STATE_GET_CALLSTACK,
  RpcMethod.STATE_GET_MEMORY_SLICE,
  RpcMethod.STATE_GET_DISASSEMBLY,
  RpcMethod.STATE_WATCH,
  RpcMethod.STATE_UNWATCH,
  RpcMethod.STATE_EDIT_WATCH,
  RpcMethod.STATE_MODIFY_WATCH_EXPRESSION,
  RpcMethod.CONTROL_STEP,
  RpcMethod.CONTROL_STEP_OVER,
  RpcMethod.CONTROL_STEP_OUT,
  RpcMethod.CONTROL_RUN_UNTIL,
  RpcMethod.CONTROL_PAUSE,
  RpcMethod.BREAKPOINTS_ADD,
  RpcMethod.BREAKPOINTS_REMOVE,
  RpcMethod.BREAKPOINTS_TOGGLE,
  RpcMethod.BREAKPOINTS_SET_CATEGORY_STATES,
  RpcMethod.EVENT_TICK,
]);

// Zod Schemas
export const RegisterValueSchema = z.object({
  name: z.string(),
  value: z.string(),
  width: z.number(),
  flags: z.record(z.string(), z.boolean()).optional(),
  metadata: z.record(z.string(), z.unknown()).optional(),
});

// Define the base schema without the recursive children
const BaseDeviceNodeDescriptorSchema = z.object({
  path: z.string(),
  label: z.string(),
  description: z.string(),
  registers: z.array(RegisterValueSchema).optional(),
  events: z.array(z.string()).optional(),
  actions: z.array(PanelIdSchema).optional(),
});

export type DeviceNodeDescriptor = z.infer<typeof BaseDeviceNodeDescriptorSchema> & {
  children?: DeviceNodeDescriptor[];
};

export const DeviceNodeDescriptorSchema: z.ZodType<DeviceNodeDescriptor> = z.lazy(() =>
  BaseDeviceNodeDescriptorSchema.extend({
    children: z.array(z.lazy(() => DeviceNodeDescriptorSchema)).optional(),
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
export type PanelId = z.infer<typeof PanelIdSchema>;
export type RegisterValue = z.infer<typeof RegisterValueSchema>;
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
  [RpcMethod.STATE_GET_CALLSTACK]: {
    params: z.object({
      target: z.enum(["sh4", "arm7"]),
      maxFrames: z.number().optional(),
    }),
    result: z.object({
      target: TargetProcessorSchema,
      frames: z.array(CallstackFrameSchema),
    }),
  },
  [RpcMethod.DEBUGGER_DESCRIBE]: {
    params: z.object({}),
    result: DebuggerShapeSchema,
  },
  [RpcMethod.STATE_GET_MEMORY_SLICE]: {
    params: z.object({
      target: TargetProcessorSchema.optional(),
      address: z.number(),
      length: z.number(),
    }),
    result: MemorySliceSchema,
  },
  [RpcMethod.STATE_GET_DISASSEMBLY]: {
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
  [RpcMethod.STATE_WATCH]: {
    params: z.object({
      expressions: z.array(z.string()),
    }),
    result: RpcErrorSchema,
  },
  [RpcMethod.STATE_UNWATCH]: {
    params: z.object({
      watchIds: z.array(WatchIdSchema),
    }),
    result: RpcErrorSchema,
  },
  [RpcMethod.STATE_EDIT_WATCH]: {
    params: z.object({
      watchId: WatchIdSchema,
      value: z.string(),
    }),
    result: RpcErrorSchema,
  },
  [RpcMethod.STATE_MODIFY_WATCH_EXPRESSION]: {
    params: z.object({
      watchId: WatchIdSchema,
      newExpression: z.string(),
    }),
    result: RpcErrorSchema,
  },
  [RpcMethod.CONTROL_STEP]: {
    params: z.object({
      target: TargetProcessorSchema,
    }),
    result: RpcErrorSchema,
  },
  [RpcMethod.CONTROL_STEP_OVER]: {
    params: z.object({
      target: TargetProcessorSchema,
    }),
    result: RpcErrorSchema,
  },
  [RpcMethod.CONTROL_STEP_OUT]: {
    params: z.object({
      target: TargetProcessorSchema,
    }),
    result: RpcErrorSchema,
  },
  [RpcMethod.CONTROL_RUN_UNTIL]: {
    params: z.object({}),
    result: RpcErrorSchema,
  },
  [RpcMethod.CONTROL_PAUSE]: {
    params: z.object({
      target: TargetProcessorSchema.optional(),
    }),
    result: RpcErrorSchema,
  },
  [RpcMethod.BREAKPOINTS_ADD]: {
    params: z.object({
      event: z.string(),
      address: z.number().optional(),
      kind: z.enum(["code", "event"]).optional(),
      enabled: z.boolean().optional(),
    }),
    result: RpcErrorSchema,
  },
  [RpcMethod.BREAKPOINTS_SET_CATEGORY_STATES]: {
    params: z.object({
      categories: z.record(z.string(), BreakpointCategoryStateSchema),
    }),
    result: RpcErrorSchema,
  },
  [RpcMethod.BREAKPOINTS_REMOVE]: {
    params: z.object({
      id: BreakpointIdSchema,
    }),
    result: RpcErrorSchema,
  },
  [RpcMethod.BREAKPOINTS_TOGGLE]: {
    params: z.object({
      id: BreakpointIdSchema,
      enabled: z.boolean(),
    }),
    result: RpcErrorSchema,
  },
  [RpcMethod.EVENT_TICK]: {
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


