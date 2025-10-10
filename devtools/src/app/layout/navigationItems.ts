export type NavigationItem = {
  id: string;
  label: string;
  category: "Debugger Preset" | "Tool" | "Page";
  onClick: () => void;
};

export const createNavigationItems = (navigate: (path: string) => void): NavigationItem[] => [
  // Debugger Workspaces
  { id: "sh4-debugger", label: "SH4 Debugger", category: "Debugger Preset", onClick: () => navigate("/workspace/sh4-debugger") },
  { id: "arm7-debugger", label: "ARM7 Debugger", category: "Debugger Preset", onClick: () => navigate("/workspace/arm7-debugger") },
  { id: "dsp-debugger", label: "DSP Debugger", category: "Debugger Preset", onClick: () => navigate("/workspace/dsp-debugger") },
  { id: "custom-debugger", label: "Custom Debugger", category: "Debugger Preset", onClick: () => navigate("/workspace/custom-debugger") },
  // Tools
  { id: "sh4-sim", label: "SH4 Simulator", category: "Tool", onClick: () => navigate("/workspace/sh4-sim") },
  { id: "dsp-playground", label: "DSP Playground", category: "Tool", onClick: () => navigate("/workspace/dsp-playground") },
  { id: "ta-log-analyzer", label: "TA Log Analyzer", category: "Tool", onClick: () => navigate("/workspace/clx2-ta-log-analyzer") },
  { id: "core-log-analyzer", label: "CORE Log Analyzer", category: "Tool", onClick: () => navigate("/workspace/clx2-core-log-analyzer") },
  // Pages
  { id: "home", label: "Home", category: "Page", onClick: () => navigate("/") },
  { id: "docs", label: "Documentation", category: "Page", onClick: () => navigate("/docs") },
];
