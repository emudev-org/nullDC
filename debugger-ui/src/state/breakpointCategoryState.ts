import type { DebuggerClient } from "../services/debuggerClient";

export type BreakpointCategory = "events" | "sh4" | "arm7" | "dsp";

export interface CategoryState {
  muted: boolean;
  soloed: boolean;
}

// Shared category state for breakpoint mute/solo across all panels
export const categoryStates = new Map<BreakpointCategory, CategoryState>([
  ["events", { muted: false, soloed: false }],
  ["sh4", { muted: false, soloed: false }],
  ["arm7", { muted: false, soloed: false }],
  ["dsp", { muted: false, soloed: false }],
]);

let currentClient: DebuggerClient | null = null;

export const setClient = (client: DebuggerClient | null) => {
  currentClient = client;
};

export const syncCategoryStatesToServer = () => {
  if (!currentClient) return;

  const categories: Record<string, { muted: boolean; soloed: boolean }> = {};
  for (const [key, value] of categoryStates.entries()) {
    categories[key] = { muted: value.muted, soloed: value.soloed };
  }

  void currentClient.setCategoryStates(categories);
};
