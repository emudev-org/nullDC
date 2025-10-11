import { useEffect, useCallback, useMemo, useState } from "react";
import { AppBar, Box, Button, Divider, IconButton, Stack, Switch, Tooltip, Typography, Alert } from "@mui/material";
import PowerSettingsNewIcon from "@mui/icons-material/PowerSettingsNew";
import CloudDoneIcon from "@mui/icons-material/CloudDone";
import CloudOffIcon from "@mui/icons-material/CloudOff";
import SyncIcon from "@mui/icons-material/Sync";
import ChevronLeftIcon from "@mui/icons-material/ChevronLeft";
import ChevronRightIcon from "@mui/icons-material/ChevronRight";
import DarkModeIcon from "@mui/icons-material/DarkMode";
import LightModeIcon from "@mui/icons-material/LightMode";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import PauseIcon from "@mui/icons-material/Pause";
import SkipNextIcon from "@mui/icons-material/SkipNext";
import { useSessionStore } from "../../state/sessionStore";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import { DeviceTreePanel } from "../panels/DeviceTreePanel";
import { WatchesPanel } from "../panels/WatchesPanel";
import { EventLogPanel } from "../panels/EventLogPanel";
import { Sh4DisassemblyPanel, Arm7DisassemblyPanel, DspDisassemblyPanel } from "../panels/DisassemblyPanel";
import { Sh4MemoryPanel, Arm7MemoryPanel } from "../panels/MemoryPanel";
import CallstackPanel from "../panels/CallstackPanel";
import { AudioPanel } from "../panels/AudioPanel";
import { TaInspectorPanel } from "../panels/TaInspectorPanel";
import { CoreInspectorPanel } from "../panels/CoreInspectorPanel";
import { EventsBreakpointsPanel, Sh4BreakpointsPanel, Arm7BreakpointsPanel, DspBreakpointsPanel } from "../panels/BreakpointsPanel";
import { DocumentationPanel } from "../panels/DocumentationPanel";
import { Sh4SimPanel } from "../panels/Sh4SimPanel";
import { DspPlaygroundPanel } from "../panels/DspPlaygroundPanel";
import { AboutDialog } from "./AboutDialog";
import { useAboutModal } from "./useAboutModal";
import { TopNav } from "./TopNav";
import { useThemeMode } from "../../theme/ThemeModeProvider";
import { DEBUGGER_VERSION } from "./aboutVersion";
import { DockingLayout, clearDockingLayout, type PanelDefinition } from "./DockingLayout";

type StoredLayoutPrefs = {
  leftPanelOpen: boolean;
  rightPanelOpen: boolean;
};

const defaultLayoutPrefs: StoredLayoutPrefs = {
  leftPanelOpen: true,
  rightPanelOpen: true,
};

const getLayoutStorageKey = (workspaceId: string) => `nulldc-debugger-layout-${workspaceId}`;

const loadLayoutPrefs = (workspaceId: string): StoredLayoutPrefs => {
  if (typeof window === "undefined") {
    return defaultLayoutPrefs;
  }

  try {
    const raw = window.localStorage.getItem(getLayoutStorageKey(workspaceId));
    if (!raw) {
      return defaultLayoutPrefs;
    }
    const parsed = JSON.parse(raw) as Partial<StoredLayoutPrefs>;
    return {
      leftPanelOpen: parsed.leftPanelOpen ?? defaultLayoutPrefs.leftPanelOpen,
      rightPanelOpen: parsed.rightPanelOpen ?? defaultLayoutPrefs.rightPanelOpen,
    };
  } catch (error) {
    console.warn("Failed to read layout preferences", error);
    return defaultLayoutPrefs;
  }
};

const leftPanelTabs: PanelDefinition[] = [
  { id: "device-tree", title: "Device Tree", component: <DeviceTreePanel /> },
];

const mainTabs: PanelDefinition[] = [
  { id: "documentation", title: "Documentation", component: <DocumentationPanel /> },
  { id: "sh4-sim", title: "SH4: Simulator", component: <Sh4SimPanel /> },
  { id: "events", title: "Events: Log", component: <EventLogPanel /> },
  { id: "events-breakpoints", title: "Events: Breakpoints", component: <EventsBreakpointsPanel /> },
  { id: "sh4-disassembly", title: "SH4: Disassembly", component: <Sh4DisassemblyPanel /> },
  { id: "sh4-memory", title: "SH4: Memory", component: <Sh4MemoryPanel /> },
  { id: "sh4-breakpoints", title: "SH4: Breakpoints", component: <Sh4BreakpointsPanel /> },
  { id: "arm7-disassembly", title: "ARM7: Disassembly", component: <Arm7DisassemblyPanel /> },
  { id: "arm7-memory", title: "ARM7: Memory", component: <Arm7MemoryPanel /> },
  { id: "arm7-breakpoints", title: "ARM7: Breakpoints", component: <Arm7BreakpointsPanel /> },
  { id: "ta", title: "TA", component: <TaInspectorPanel /> },
  { id: "core", title: "CORE", component: <CoreInspectorPanel /> },
  { id: "aica", title: "AICA", component: <AudioPanel /> },
  { id: "dsp-disassembly", title: "DSP: Disassembly", component: <DspDisassemblyPanel /> },
  { id: "dsp-breakpoints", title: "DSP: Breakpoints", component: <DspBreakpointsPanel /> },
  { id: "dsp-playground", title: "DSP: Playground", component: <DspPlaygroundPanel /> },
];

const rightPanelTabs: PanelDefinition[] = [
  { id: "watches", title: "Watches", component: <WatchesPanel /> },
  { id: "sh4-callstack", title: "SH4: Callstack", component: <CallstackPanel target="sh4" showTitle={false} /> },
  { id: "arm7-callstack", title: "ARM7: Callstack", component: <CallstackPanel target="arm7" showTitle={false} /> },
];

const connectionIcons = {
  idle: <CloudOffIcon fontSize="small" />,
  error: <CloudOffIcon fontSize="small" />,
  connecting: <SyncIcon fontSize="small" className="spin" />,
  connected: <CloudDoneIcon fontSize="small" />,
};

type Notification = {
  id: string;
  type: "error" | "warning";
  message: string;
  timestamp: number;
};

interface AppLayoutProps {
  workspaceId: string;
}

export const AppLayout = ({ workspaceId }: AppLayoutProps) => {
  const connect = useSessionStore((state) => state.connect);
  const disconnect = useSessionStore((state) => state.disconnect);
  const connectionState = useSessionStore((state) => state.connectionState);
  const connectionError = useSessionStore((state) => state.connectionError);
  const endpoint = useSessionStore((state) => state.endpoint);
  const client = useSessionStore((state) => state.client);
  const executionState = useSessionStore((state) => state.executionState);
  const initializeData = useDebuggerDataStore((state) => state.initialize);
  const breakpointHit = useDebuggerDataStore((state) => state.breakpointHit);
  const errorMessage = useDebuggerDataStore((state) => state.errorMessage);
  const clearError = useDebuggerDataStore((state) => state.clearError);
  const [leftPanelOpen, setLeftPanelOpen] = useState(() => loadLayoutPrefs(workspaceId).leftPanelOpen);
  const [rightPanelOpen, setRightPanelOpen] = useState(() => loadLayoutPrefs(workspaceId).rightPanelOpen);
  const [isNarrow, setIsNarrow] = useState(window.innerWidth < 1200);
  const { open: aboutOpen, show: showAbout, hide: hideAbout } = useAboutModal();
  const { mode, toggleMode } = useThemeMode();
  const isDarkMode = mode === "dark";
  const [notifications, setNotifications] = useState<Notification[]>([]);

  const workspaceTabs = useMemo(() => {
    if (isNarrow) {
      return [...mainTabs, ...leftPanelTabs, ...rightPanelTabs];
    }

    // Custom initial panels based on workspace
    if (workspaceId === 'sh4-debugger') {
      return [
        { id: "sh4-disassembly", title: "SH4: Disassembly", component: <Sh4DisassemblyPanel /> },
        { id: "sh4-memory", title: "SH4: Memory", component: <Sh4MemoryPanel /> },
        { id: "sh4-breakpoints", title: "SH4: Breakpoints", component: <Sh4BreakpointsPanel /> },
      ];
    }

    if (workspaceId === 'arm7-debugger') {
      return [
        { id: "arm7-disassembly", title: "ARM7: Disassembly", component: <Arm7DisassemblyPanel /> },
        { id: "arm7-memory", title: "ARM7: Memory", component: <Arm7MemoryPanel /> },
        { id: "arm7-breakpoints", title: "ARM7: Breakpoints", component: <Arm7BreakpointsPanel /> },
      ];
    }

    if (workspaceId === 'dsp-debugger') {
      return [
        { id: "dsp-disassembly", title: "DSP: Disassembly", component: <DspDisassemblyPanel /> },
        { id: "aica", title: "AICA", component: <AudioPanel /> },
        { id: "dsp-breakpoints", title: "DSP: Breakpoints", component: <DspBreakpointsPanel /> },
      ];
    }

    if (workspaceId === 'mixed-mode-debugger') {
      return [
        { id: "sh4-disassembly", title: "SH4: Disassembly", component: <Sh4DisassemblyPanel /> },
        { id: "arm7-disassembly", title: "ARM7: Disassembly", component: <Arm7DisassemblyPanel /> },
      ];
    }

    return mainTabs;
  }, [isNarrow, workspaceId]);

  const rightPanelTabsForWorkspace = useMemo(() => {
    if (workspaceId === 'sh4-debugger') {
      return [
        { id: "watches", title: "Watches", component: <WatchesPanel /> },
        { id: "sh4-callstack", title: "SH4: Callstack", component: <CallstackPanel target="sh4" showTitle={false} /> },
      ];
    }
    if (workspaceId === 'arm7-debugger') {
      return [
        { id: "watches", title: "Watches", component: <WatchesPanel /> },
        { id: "arm7-callstack", title: "ARM7: Callstack", component: <CallstackPanel target="arm7" showTitle={false} /> },
      ];
    }
    if (workspaceId === 'dsp-debugger') {
      return [
        { id: "watches", title: "Watches", component: <WatchesPanel /> },
      ];
    }
    if (workspaceId === 'mixed-mode-debugger') {
      return [
        { id: "watches", title: "Watches", component: <WatchesPanel /> },
        { id: "sh4-callstack", title: "SH4: Callstack", component: <CallstackPanel target="sh4" showTitle={false} /> },
        { id: "arm7-callstack", title: "ARM7: Callstack", component: <CallstackPanel target="arm7" showTitle={false} /> },
      ];
    }
    return rightPanelTabs;
  }, [workspaceId]);

  const allPanels = useMemo(() => {
    return [...leftPanelTabs, ...mainTabs, ...rightPanelTabs];
  }, []);

  const showLeftPanel = !isNarrow && leftPanelOpen;
  const showRightPanel = !isNarrow && rightPanelOpen;
  const showLeftToggle = !isNarrow;
  const showRightToggle = !isNarrow;

  // Load layout preferences when workspaceId changes
  useEffect(() => {
    const prefs = loadLayoutPrefs(workspaceId);
    setLeftPanelOpen(prefs.leftPanelOpen);
    setRightPanelOpen(prefs.rightPanelOpen);
  }, [workspaceId]);

  useEffect(() => {
    const handleResize = () => {
      setIsNarrow(window.innerWidth < 1200);
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    const prefs: StoredLayoutPrefs = {
      leftPanelOpen,
      rightPanelOpen,
    };

    try {
      window.localStorage.setItem(getLayoutStorageKey(workspaceId), JSON.stringify(prefs));
    } catch (error) {
      console.warn("Failed to persist layout preferences", error);
    }
  }, [leftPanelOpen, rightPanelOpen, workspaceId]);

  useEffect(() => {
    void connect();
  }, [connect]);

  useEffect(() => {
    if (connectionState === "connected" && client) {
      void initializeData(client);
    }
  }, [client, connectionState, initializeData]);


  // Add error messages to notification stack
  useEffect(() => {
    if (errorMessage) {
      const notification: Notification = {
        id: `error-${Date.now()}`,
        type: "error",
        message: errorMessage,
        timestamp: Date.now(),
      };
      setNotifications((prev) => [...prev, notification]);

      // Auto-remove after 5 seconds
      setTimeout(() => {
        setNotifications((prev) => prev.filter((n) => n.id !== notification.id));
      }, 5000);

      // Clear the error from the store
      clearError();
    }
  }, [errorMessage, clearError]);

  // Add breakpoint hits to notification stack
  useEffect(() => {
    if (breakpointHit) {
      const bpDisplay = breakpointHit.breakpoint.kind === "event"
        ? breakpointHit.breakpoint.event
        : `${breakpointHit.breakpoint.event} == 0x${(breakpointHit.breakpoint.address ?? 0).toString(16).toUpperCase().padStart(8, "0")}`;
      const notification: Notification = {
        id: `breakpoint-${Date.now()}`,
        type: "warning",
        message: `Breakpoint hit: ${bpDisplay}`,
        timestamp: Date.now(),
      };
      setNotifications((prev) => [...prev, notification]);

      // Auto-remove after 5 seconds
      setTimeout(() => {
        setNotifications((prev) => prev.filter((n) => n.id !== notification.id));
      }, 5000);
    }
  }, [breakpointHit]);


  const handleToggleConnection = useCallback(() => {
    if (connectionState === "connected") {
      disconnect();
    } else {
      void connect({ force: true });
    }
  }, [connectionState, disconnect, connect]);

  const handleToggleTheme = useCallback(() => {
    toggleMode();
  }, [toggleMode]);

  const handleResetLayout = useCallback(() => {
    setLeftPanelOpen(defaultLayoutPrefs.leftPanelOpen);
    setRightPanelOpen(defaultLayoutPrefs.rightPanelOpen);

    if (typeof window === "undefined") {
      return;
    }

    try {
      window.localStorage.removeItem(getLayoutStorageKey(workspaceId));
      clearDockingLayout(workspaceId); // Center panel
      clearDockingLayout(`${workspaceId}-left`); // Left panel
      clearDockingLayout(`${workspaceId}-right`); // Right panel
      // Reload to reset docking layout
      window.location.reload();
    } catch (error) {
      console.warn("Failed to clear layout preferences", error);
    }
  }, [workspaceId]);

  const handleRun = useCallback(async () => {
    if (!client || connectionState !== "connected") {
      return;
    }
    try {
      await client.runUntil();
      // State will be updated via notification from server
    } catch (error) {
      console.error("Failed to run", error);
    }
  }, [client, connectionState]);

  const handlePause = useCallback(async () => {
    if (!client || connectionState !== "connected") {
      return;
    }
    try {
      await client.pause("sh4");
      // State will be updated via notification from server
    } catch (error) {
      console.error("Failed to pause", error);
    }
  }, [client, connectionState]);

  const handleRunToBreakpoint = useCallback(async () => {
    if (!client || connectionState !== "connected") {
      return;
    }
    try {
      // Run until breakpoint
      await client.runUntil();
      // State will be updated via notification from server
    } catch (error) {
      console.error("Failed to run to breakpoint", error);
    }
  }, [client, connectionState]);

  return (
    <Box sx={{ height: "100vh", display: "flex", flexDirection: "column" }}>
      <AppBar position="static" elevation={1} color="default">
        <TopNav
          onAboutClick={showAbout}
          onResetLayout={handleResetLayout}
          currentPage={workspaceId}
          centerSection={
            <Stack direction="row" spacing={1} alignItems="center">
              <Tooltip title="Run">
                <span>
                  <IconButton
                    size="small"
                    color={connectionState === "connected" && executionState === "paused" ? "success" : "default"}
                    disabled={connectionState !== "connected" || executionState === "running"}
                    onClick={handleRun}
                  >
                    <PlayArrowIcon />
                  </IconButton>
                </span>
              </Tooltip>
              <Tooltip title="Pause">
                <span>
                  <IconButton
                    size="small"
                    color={connectionState === "connected" && executionState === "running" ? "warning" : "default"}
                    disabled={connectionState !== "connected" || executionState === "paused"}
                    onClick={handlePause}
                  >
                    <PauseIcon />
                  </IconButton>
                </span>
              </Tooltip>
              <Tooltip title="Run to next breakpoint">
                <span>
                  <IconButton
                    size="small"
                    color={connectionState === "connected" && executionState === "paused" ? "primary" : "default"}
                    disabled={connectionState !== "connected" || executionState === "running"}
                    onClick={handleRunToBreakpoint}
                  >
                    <SkipNextIcon />
                  </IconButton>
                </span>
              </Tooltip>
            </Stack>
          }
          rightSection={
            <Stack direction="row" spacing={1.5} alignItems="center">
              <Stack direction="row" spacing={0.5} alignItems="center">
                {isDarkMode ? (
                  <DarkModeIcon fontSize="small" color="primary" />
                ) : (
                  <LightModeIcon fontSize="small" color="warning" />
                )}
                <Tooltip title={isDarkMode ? "Dark mode" : "Light mode"}>
                  <Switch
                    size="small"
                    checked={isDarkMode}
                    onChange={handleToggleTheme}
                    inputProps={{ "aria-label": "Toggle dark mode" }}
                  />
                </Tooltip>
              </Stack>
              <Tooltip title={`Connection: ${connectionState}`}>
                <IconButton color={connectionState === "connected" ? "primary" : "inherit"}>
                  {connectionIcons[connectionState]}
                </IconButton>
              </Tooltip>
              <Button
                variant="outlined"
                color="inherit"
                onClick={handleToggleConnection}
                startIcon={<PowerSettingsNewIcon fontSize="small" />}
              >
                {connectionState === "connected" ? "Disconnect" : "Connect"}
              </Button>
            </Stack>
          }
        />
      </AppBar>
      {connectionError && (
        <Alert severity="error" sx={{ borderRadius: 0 }}>
          {connectionError}
        </Alert>
      )}
      <Box sx={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        <Box
          sx={{
            flex: 1,
            overflow: "hidden",
            display: "flex",
            gap: 1,
            p: 1,
            position: "relative",
          }}
        >
          {showLeftPanel && (
            <Box sx={{ minHeight: 0, width: 280 }}>
              <DockingLayout
                key={`${workspaceId}-left`}
                panels={leftPanelTabs}
                allPanels={allPanels}
                workspaceId={`${workspaceId}-left`}
              />
            </Box>
          )}
          {showLeftToggle && (
            <Box sx={{ display: "flex", alignItems: "center" }}>
              <Tooltip title={leftPanelOpen ? "Hide left panel" : "Show left panel"}>
                <IconButton onClick={() => setLeftPanelOpen(!leftPanelOpen)} size="small">
                  {leftPanelOpen ? <ChevronLeftIcon /> : <ChevronRightIcon />}
                </IconButton>
              </Tooltip>
            </Box>
          )}
          <Box
            sx={{
              minHeight: 0,
              minWidth: 0,
              flex: 1,
            }}
          >
            <DockingLayout
              key={workspaceId}
              panels={workspaceTabs}
              allPanels={allPanels}
              workspaceId={workspaceId}
              defaultLayoutMode={
                workspaceId === 'mixed-mode-debugger' ? 'mixed-mode-debugger-layout' :
                workspaceId === 'sh4-debugger' || workspaceId === 'arm7-debugger' || workspaceId === 'dsp-debugger' ? 'sh4-layout' :
                'tabs'
              }
            />
          </Box>
          {showRightToggle && (
            <Box sx={{ display: "flex", alignItems: "center" }}>
              <Tooltip title={rightPanelOpen ? "Hide right panel" : "Show right panel"}>
                <IconButton onClick={() => setRightPanelOpen(!rightPanelOpen)} size="small">
                  {rightPanelOpen ? <ChevronRightIcon /> : <ChevronLeftIcon />}
                </IconButton>
              </Tooltip>
            </Box>
          )}
          {showRightPanel && (
            <Box sx={{ minHeight: 0, width: 340 }}>
              <DockingLayout
                key={`${workspaceId}-right`}
                panels={rightPanelTabsForWorkspace}
                allPanels={allPanels}
                workspaceId={`${workspaceId}-right`}
                defaultLayoutMode="vertical-stack"
              />
            </Box>
          )}
        </Box>
      </Box>
      <Divider />
      <Box
        component="footer"
        sx={{
          px: 2,
          py: 0.75,
          display: "flex",
          alignItems: "center",
          gap: 2,
          typography: "caption",
          color: "text.secondary",
        }}
      >
        <Typography variant="caption">Connection: {connectionState}</Typography>
        <Divider orientation="vertical" flexItem />
        <Typography variant="caption">Endpoint: {endpoint ?? "-"}</Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Typography variant="caption">nullDC Debugger {DEBUGGER_VERSION}</Typography>
      </Box>
      <AboutDialog open={aboutOpen} onClose={hideAbout} />
      <Box
        sx={{
          position: "fixed",
          bottom: 16,
          right: 16,
          zIndex: 9999,
          display: "flex",
          flexDirection: "column-reverse",
          gap: 1,
          pointerEvents: "none",
        }}
      >
        {notifications.map((notification) => (
          <Alert
            key={notification.id}
            severity={notification.type}
            variant="filled"
            onClose={() => setNotifications((prev) => prev.filter((n) => n.id !== notification.id))}
            sx={{ pointerEvents: "auto" }}
          >
            {notification.message}
          </Alert>
        ))}
      </Box>
    </Box>
  );
};
