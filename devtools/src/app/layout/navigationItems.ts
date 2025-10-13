export type NavigationItem = {
  id: string;
  label: string;
  category: "Workspace" | "Tool" | "Page";
  onClick: () => void;
};

export const createNavigationItems = (navigate: (path: string) => void): NavigationItem[] => [
  // Debugger Workspaces
  { id: "sh4-debugger", label: "SH4 Debugger", category: "Workspace", onClick: () => navigate("/workspace/sh4-debugger") },
  { id: "arm7-debugger", label: "ARM7 Debugger", category: "Workspace", onClick: () => navigate("/workspace/arm7-debugger") },
  { id: "audio-debugger", label: "Audio Debugger", category: "Workspace", onClick: () => navigate("/workspace/audio-debugger") },
  { id: "mixed-mode-debugger", label: "Mixed Mode Debugger", category: "Workspace", onClick: () => navigate("/workspace/mixed-mode-debugger") },
  // Tools
  { id: "sh4-sim", label: "SH4 Simulator", category: "Tool", onClick: () => navigate("/tool/sh4-sim") },
  { id: "dsp-playground", label: "DSP Playground", category: "Tool", onClick: () => navigate("/tool/dsp-playground") },
  { id: "ta-log-analyzer", label: "TA Log Analyzer", category: "Tool", onClick: () => navigate("/tool/holly-ta-log-analyzer") },
  { id: "core-log-analyzer", label: "CORE Log Analyzer", category: "Tool", onClick: () => navigate("/tool/holly-core-log-analyzer") },
  // Pages
  { id: "home", label: "Home", category: "Page", onClick: () => navigate("/") },
  { id: "docs", label: "Documentation", category: "Page", onClick: () => navigate("/docs") },
];
