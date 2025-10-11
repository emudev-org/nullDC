import { useEffect, useCallback, useMemo, useState, useRef } from "react";
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
import {
  BscRegistersPanel,
  CcnRegistersPanel,
  CpgRegistersPanel,
  DmacRegistersPanel,
  IntcRegistersPanel,
  RtcRegistersPanel,
  SciRegistersPanel,
  ScifRegistersPanel,
  TmuRegistersPanel,
  UbcRegistersPanel,
} from "../panels/Sh4RegisterPanels";
import {
  SqContentsPanel,
  IcacheContentsPanel,
  OcacheContentsPanel,
  OcramContentsPanel,
  TlbContentsPanel,
} from "../panels/Sh4CachePanels";
import { AboutDialog } from "./AboutDialog";
import { useAboutModal } from "./useAboutModal";
import { TopNav } from "./TopNav";
import { useThemeMode } from "../../theme/ThemeModeProvider";
import { DEBUGGER_VERSION } from "./aboutVersion";
import { DockingLayout, clearDockingLayout, type PanelDefinition } from "./DockingLayout";
import type { DockviewApi } from "dockview";
import { PANEL_IDS, type PanelId } from "../../lib/debuggerSchema";

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

// Create a ref to store the main panel API for adding panels from the device tree
let mainPanelApi: DockviewApi | null = null;

const createLeftPanelTabs = (onOpenPanel?: (panelId: PanelId) => void): PanelDefinition[] => [
  { id: PANEL_IDS.DEVICE_TREE, title: "Device Tree", component: <DeviceTreePanel onOpenPanel={onOpenPanel} /> },
];

const mainTabs: PanelDefinition[] = [
  { id: PANEL_IDS.DOCUMENTATION, title: "Documentation", component: <DocumentationPanel /> },
  { id: PANEL_IDS.SH4_SIM, title: "SH4: Simulator", component: <Sh4SimPanel /> },
  { id: PANEL_IDS.EVENTS, title: "Events: Log", component: <EventLogPanel /> },
  { id: PANEL_IDS.EVENTS_BREAKPOINTS, title: "Events: Breakpoints", component: <EventsBreakpointsPanel /> },
  { id: PANEL_IDS.SH4_DISASSEMBLY, title: "SH4: Disassembly", component: <Sh4DisassemblyPanel /> },
  { id: PANEL_IDS.SH4_MEMORY, title: "SH4: Memory", component: <Sh4MemoryPanel /> },
  { id: PANEL_IDS.SH4_BREAKPOINTS, title: "SH4: Breakpoints", component: <Sh4BreakpointsPanel /> },
  { id: PANEL_IDS.SH4_BSC_REGISTERS, title: "SH4: BSC Registers", component: <BscRegistersPanel /> },
  { id: PANEL_IDS.SH4_CCN_REGISTERS, title: "SH4: CCN Registers", component: <CcnRegistersPanel /> },
  { id: PANEL_IDS.SH4_CPG_REGISTERS, title: "SH4: CPG Registers", component: <CpgRegistersPanel /> },
  { id: PANEL_IDS.SH4_DMAC_REGISTERS, title: "SH4: DMAC Registers", component: <DmacRegistersPanel /> },
  { id: PANEL_IDS.SH4_INTC_REGISTERS, title: "SH4: INTC Registers", component: <IntcRegistersPanel /> },
  { id: PANEL_IDS.SH4_RTC_REGISTERS, title: "SH4: RTC Registers", component: <RtcRegistersPanel /> },
  { id: PANEL_IDS.SH4_SCI_REGISTERS, title: "SH4: SCI Registers", component: <SciRegistersPanel /> },
  { id: PANEL_IDS.SH4_SCIF_REGISTERS, title: "SH4: SCIF Registers", component: <ScifRegistersPanel /> },
  { id: PANEL_IDS.SH4_TMU_REGISTERS, title: "SH4: TMU Registers", component: <TmuRegistersPanel /> },
  { id: PANEL_IDS.SH4_UBC_REGISTERS, title: "SH4: UBC Registers", component: <UbcRegistersPanel /> },
  { id: PANEL_IDS.SH4_SQ_CONTENTS, title: "SH4: Store Queues", component: <SqContentsPanel /> },
  { id: PANEL_IDS.SH4_ICACHE_CONTENTS, title: "SH4: ICACHE", component: <IcacheContentsPanel /> },
  { id: PANEL_IDS.SH4_OCACHE_CONTENTS, title: "SH4: OCACHE", component: <OcacheContentsPanel /> },
  { id: PANEL_IDS.SH4_OCRAM_CONTENTS, title: "SH4: OC-RAM", component: <OcramContentsPanel /> },
  { id: PANEL_IDS.SH4_TLB_CONTENTS, title: "SH4: TLB", component: <TlbContentsPanel /> },
  { id: PANEL_IDS.ARM7_DISASSEMBLY, title: "ARM7: Disassembly", component: <Arm7DisassemblyPanel /> },
  { id: PANEL_IDS.ARM7_MEMORY, title: "ARM7: Memory", component: <Arm7MemoryPanel /> },
  { id: PANEL_IDS.ARM7_BREAKPOINTS, title: "ARM7: Breakpoints", component: <Arm7BreakpointsPanel /> },
  { id: PANEL_IDS.CLX2_TA, title: "CLX2: TA", component: <TaInspectorPanel /> },
  { id: PANEL_IDS.CLX2_CORE, title: "CLX2: CORE", component: <CoreInspectorPanel /> },
  { id: PANEL_IDS.SGC, title: "SGC", component: <AudioPanel /> },
  { id: PANEL_IDS.DSP_DISASSEMBLY, title: "DSP: Disassembly", component: <DspDisassemblyPanel /> },
  { id: PANEL_IDS.DSP_BREAKPOINTS, title: "DSP: Breakpoints", component: <DspBreakpointsPanel /> },
  { id: PANEL_IDS.DSP_PLAYGROUND, title: "DSP: Playground", component: <DspPlaygroundPanel /> },
];

const rightPanelTabs: PanelDefinition[] = [
  { id: PANEL_IDS.WATCHES, title: "Watches", component: <WatchesPanel /> },
  { id: PANEL_IDS.SH4_CALLSTACK, title: "SH4: Callstack", component: <CallstackPanel target="sh4" showTitle={false} /> },
  { id: PANEL_IDS.ARM7_CALLSTACK, title: "ARM7: Callstack", component: <CallstackPanel target="arm7" showTitle={false} /> },
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
  const [mainPanelApiState, setMainPanelApiState] = useState<DockviewApi | null>(null);

  // Use a ref to avoid stale closures
  const mainPanelApiRef = useRef<DockviewApi | null>(null);

  useEffect(() => {
    mainPanelApiRef.current = mainPanelApiState;
  }, [mainPanelApiState]);

  const handleOpenPanelInMain = useCallback((panelId: PanelId) => {
    const api = mainPanelApiRef.current;
    if (!api) {
      return;
    }

    // Find panel definition
    const panelDef = mainTabs.find(p => p.id === panelId) || rightPanelTabs.find(p => p.id === panelId);
    if (!panelDef) {
      return;
    }

    // Check if panel already exists
    const existingPanel = api.getPanel(panelId);
    if (existingPanel) {
      // Focus the existing panel
      existingPanel.api.setActive();
      return;
    }

    // Get the currently active group to add the panel next to it
    const activeGroup = api.activeGroup;

    // Add the panel to the main view
    if (activeGroup) {
      api.addPanel({
        id: panelId,
        component: panelId,
        title: panelDef.title,
        position: {
          referenceGroup: activeGroup,
          direction: 'within',
        },
      });
    } else {
      api.addPanel({
        id: panelId,
        component: panelId,
        title: panelDef.title,
      });
    }
  }, []);

  const leftPanelTabs = useMemo(() => createLeftPanelTabs(handleOpenPanelInMain), [handleOpenPanelInMain]);

  const allPanels = useMemo(() => {
    return [...leftPanelTabs, ...mainTabs, ...rightPanelTabs];
  }, [leftPanelTabs]);

  const workspaceTabs = useMemo(() => {
    if (isNarrow) {
      return [...mainTabs, ...leftPanelTabs, ...rightPanelTabs];
    }

    // Custom initial panels based on workspace
    if (workspaceId === 'sh4-debugger') {
      return [
        { id: PANEL_IDS.SH4_DISASSEMBLY, title: "SH4: Disassembly", component: <Sh4DisassemblyPanel /> },
        { id: PANEL_IDS.SH4_MEMORY, title: "SH4: Memory", component: <Sh4MemoryPanel /> },
        { id: PANEL_IDS.SH4_BREAKPOINTS, title: "SH4: Breakpoints", component: <Sh4BreakpointsPanel /> },
      ];
    }

    if (workspaceId === 'arm7-debugger') {
      return [
        { id: PANEL_IDS.ARM7_DISASSEMBLY, title: "ARM7: Disassembly", component: <Arm7DisassemblyPanel /> },
        { id: PANEL_IDS.ARM7_MEMORY, title: "ARM7: Memory", component: <Arm7MemoryPanel /> },
        { id: PANEL_IDS.ARM7_BREAKPOINTS, title: "ARM7: Breakpoints", component: <Arm7BreakpointsPanel /> },
      ];
    }

    if (workspaceId === 'audio-debugger') {
      return [
        { id: PANEL_IDS.SGC, title: "SGC", component: <AudioPanel /> },
        { id: PANEL_IDS.DSP_DISASSEMBLY, title: "DSP: Disassembly", component: <DspDisassemblyPanel /> },
        { id: PANEL_IDS.DSP_BREAKPOINTS, title: "DSP: Breakpoints", component: <DspBreakpointsPanel /> },
      ];
    }

    if (workspaceId === 'mixed-mode-debugger') {
      return [
        { id: PANEL_IDS.SH4_DISASSEMBLY, title: "SH4: Disassembly", component: <Sh4DisassemblyPanel /> },
        { id: PANEL_IDS.ARM7_DISASSEMBLY, title: "ARM7: Disassembly", component: <Arm7DisassemblyPanel /> },
      ];
    }

    return mainTabs;
  }, [isNarrow, workspaceId]);

  const rightPanelTabsForWorkspace = useMemo(() => {
    if (workspaceId === 'sh4-debugger') {
      return [
        { id: PANEL_IDS.WATCHES, title: "Watches", component: <WatchesPanel /> },
        { id: PANEL_IDS.SH4_CALLSTACK, title: "SH4: Callstack", component: <CallstackPanel target="sh4" showTitle={false} /> },
      ];
    }
    if (workspaceId === 'arm7-debugger') {
      return [
        { id: PANEL_IDS.WATCHES, title: "Watches", component: <WatchesPanel /> },
        { id: PANEL_IDS.ARM7_CALLSTACK, title: "ARM7: Callstack", component: <CallstackPanel target="arm7" showTitle={false} /> },
      ];
    }
    if (workspaceId === 'audio-debugger') {
      return [
        { id: PANEL_IDS.WATCHES, title: "Watches", component: <WatchesPanel /> },
      ];
    }
    if (workspaceId === 'mixed-mode-debugger') {
      return [
        { id: PANEL_IDS.WATCHES, title: "Watches", component: <WatchesPanel /> },
        { id: PANEL_IDS.SH4_CALLSTACK, title: "SH4: Callstack", component: <CallstackPanel target="sh4" showTitle={false} /> },
        { id: PANEL_IDS.ARM7_CALLSTACK, title: "ARM7: Callstack", component: <CallstackPanel target="arm7" showTitle={false} /> },
      ];
    }
    return rightPanelTabs;
  }, [workspaceId]);

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
              onReady={setMainPanelApiState}
              defaultLayoutMode={
                workspaceId === 'mixed-mode-debugger' ? 'mixed-mode-debugger-layout' :
                workspaceId === 'sh4-debugger' || workspaceId === 'arm7-debugger' || workspaceId === 'audio-debugger' ? 'sh4-layout' :
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
