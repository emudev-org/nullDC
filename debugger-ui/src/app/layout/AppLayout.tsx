import { useEffect, useCallback, useMemo, useState, useRef } from "react";
import { AppBar, Box, Button, Divider, IconButton, Stack, Switch, Tab, Tabs, Tooltip, Typography, Alert } from "@mui/material";
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
import { Sh4CallstackPanel, Arm7CallstackPanel } from "../panels/CallstackPanel";
import { AudioPanel } from "../panels/AudioPanel";
import { TaInspectorPanel } from "../panels/TaInspectorPanel";
import { CoreInspectorPanel } from "../panels/CoreInspectorPanel";
import { EventsBreakpointsPanel, Sh4BreakpointsPanel, Arm7BreakpointsPanel, DspBreakpointsPanel } from "../panels/BreakpointsPanel";
import { Sh4SimPanel } from "../panels/Sh4SimPanel";
import { DspPlaygroundPanel } from "../panels/DspPlaygroundPanel";
import { useNavigate, useParams } from "react-router-dom";
import { AboutDialog } from "./AboutDialog";
import { useAboutModal } from "./useAboutModal";
import { TopNav } from "./TopNav";
import { useThemeMode } from "../../theme/ThemeModeProvider";

const LAYOUT_STORAGE_KEY = "nulldc-debugger-layout";

type StoredLayoutPrefs = {
  leftPanelOpen: boolean;
  rightPanelOpen: boolean;
};

const defaultLayoutPrefs: StoredLayoutPrefs = {
  leftPanelOpen: true,
  rightPanelOpen: true,
};

const loadLayoutPrefs = (): StoredLayoutPrefs => {
  if (typeof window === "undefined") {
    return defaultLayoutPrefs;
  }

  try {
    const raw = window.localStorage.getItem(LAYOUT_STORAGE_KEY);
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

const mainTabs = [
  { value: "events", label: "Events: Log", component: <EventLogPanel /> },
  { value: "events-breakpoints", label: "Events: Breakpoints", component: <EventsBreakpointsPanel /> },
  { value: "sh4-disassembly", label: "SH4: Disassembly", component: <Sh4DisassemblyPanel /> },
  { value: "sh4-memory", label: "SH4: Memory", component: <Sh4MemoryPanel /> },
  { value: "sh4-breakpoints", label: "SH4: Breakpoints", component: <Sh4BreakpointsPanel /> },
  { value: "arm7-disassembly", label: "ARM7: Disassembly", component: <Arm7DisassemblyPanel /> },
  { value: "arm7-memory", label: "ARM7: Memory", component: <Arm7MemoryPanel /> },
  { value: "arm7-breakpoints", label: "ARM7: Breakpoints", component: <Arm7BreakpointsPanel /> },
  { value: "ta", label: "TA", component: <TaInspectorPanel /> },
  { value: "core", label: "CORE", component: <CoreInspectorPanel /> },
  { value: "aica", label: "AICA", component: <AudioPanel /> },
  { value: "dsp-disassembly", label: "DSP: Disassembly", component: <DspDisassemblyPanel /> },
  { value: "dsp-breakpoints", label: "DSP: Breakpoints", component: <DspBreakpointsPanel /> },
  { value: "dsp-playground", label: "DSP: Playground", component: <DspPlaygroundPanel /> },
  { value: "sh4-sim", label: "SH4: Sim", component: <Sh4SimPanel /> },
];

const sidePanelTabs = [
  { value: "device-tree", label: "Device Tree", component: <DeviceTreePanel /> },
  { value: "watches", label: "Watches", component: <WatchesPanel showTitle={false} /> },
  { value: "sh4-callstack", label: "SH4: Callstack", component: <Sh4CallstackPanel showTitle={false} /> },
  { value: "arm7-callstack", label: "ARM7: Callstack", component: <Arm7CallstackPanel showTitle={false} /> },
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

export const AppLayout = () => {
  const connect = useSessionStore((state) => state.connect);
  const disconnect = useSessionStore((state) => state.disconnect);
  const connectionState = useSessionStore((state) => state.connectionState);
  const connectionError = useSessionStore((state) => state.connectionError);
  const session = useSessionStore((state) => state.session);
  const endpoint = useSessionStore((state) => state.endpoint);
  const client = useSessionStore((state) => state.client);
  const executionState = useSessionStore((state) => state.executionState);
  const initializeData = useDebuggerDataStore((state) => state.initialize);
  const resetData = useDebuggerDataStore((state) => state.reset);
  const breakpointHit = useDebuggerDataStore((state) => state.breakpointHit);
  const errorMessage = useDebuggerDataStore((state) => state.errorMessage);
  const clearError = useDebuggerDataStore((state) => state.clearError);
  const navigate = useNavigate();
  const { tab } = useParams();
  const [leftPanelOpen, setLeftPanelOpen] = useState(() => loadLayoutPrefs().leftPanelOpen);
  const [rightPanelOpen, setRightPanelOpen] = useState(() => loadLayoutPrefs().rightPanelOpen);
  const [isNarrow, setIsNarrow] = useState(window.innerWidth < 1200);
  const tabsContainerRef = useRef<HTMLDivElement | null>(null);
  const { open: aboutOpen, show: showAbout, hide: hideAbout } = useAboutModal();
  const { mode, toggleMode } = useThemeMode();
  const isDarkMode = mode === "dark";
  const [notifications, setNotifications] = useState<Notification[]>([]);

  const workspaceTabs = useMemo(() => {
    return isNarrow ? [...mainTabs, ...sidePanelTabs] : mainTabs;
  }, [isNarrow]);

  const validValues = useMemo(() => new Set(workspaceTabs.map(t => t.value)), [workspaceTabs]);
  const currentTab = validValues.has(tab ?? "") ? (tab as string) : workspaceTabs[0].value;
  const sidePanelsLocked = currentTab === "sh4-sim" || currentTab === "dsp-playground";
  const showLeftPanel = !isNarrow && !sidePanelsLocked && leftPanelOpen;
  const showRightPanel = !isNarrow && !sidePanelsLocked && rightPanelOpen;
  const showLeftToggle = !isNarrow && !sidePanelsLocked;
  const showRightToggle = !isNarrow && !sidePanelsLocked;

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
      window.localStorage.setItem(LAYOUT_STORAGE_KEY, JSON.stringify(prefs));
    } catch (error) {
      console.warn("Failed to persist layout preferences", error);
    }
  }, [leftPanelOpen, rightPanelOpen]);

  useEffect(() => {
    void connect();
  }, [connect]);

  useEffect(() => {
    if (connectionState === "connected" && client) {
      void initializeData(client);
    }
  }, [client, connectionState, initializeData]);

  useEffect(() => {
    if (connectionState === "idle" || connectionState === "error") {
      resetData();
    }
  }, [connectionState, resetData]);

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
      const notification: Notification = {
        id: `breakpoint-${Date.now()}`,
        type: "warning",
        message: `Breakpoint hit: ${breakpointHit.breakpoint.location}`,
        timestamp: Date.now(),
      };
      setNotifications((prev) => [...prev, notification]);

      // Auto-remove after 5 seconds
      setTimeout(() => {
        setNotifications((prev) => prev.filter((n) => n.id !== notification.id));
      }, 5000);
    }
  }, [breakpointHit]);

  useEffect(() => {
    const container = tabsContainerRef.current;
    if (!container) {
      return;
    }
    const scroller = container.querySelector<HTMLElement>(".MuiTabs-scroller");
    if (!scroller) {
      return;
    }

    const handleWheel = (event: WheelEvent) => {
      if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) {
        return;
      }
      event.preventDefault();
      scroller.scrollLeft -= event.deltaY;
    };

    scroller.addEventListener("wheel", handleWheel, { passive: false });
    return () => {
      scroller.removeEventListener("wheel", handleWheel);
    };
  }, [workspaceTabs, isNarrow]);

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
      window.localStorage.removeItem(LAYOUT_STORAGE_KEY);
    } catch (error) {
      console.warn("Failed to clear layout preferences", error);
    }
  }, []);

  const handleRun = useCallback(async () => {
    if (!client || connectionState !== "connected") {
      return;
    }
    try {
      await (client as any).rpc.call("control.runUntil", {
        target: "sh4",
        type: "interrupt",
      });
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
      await (client as any).rpc.call("control.pause", {
        target: "sh4",
      });
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
      // Run until breakpoint - using sh4 as default target
      await (client as any).rpc.call("control.runUntil", {
        target: "sh4",
        type: "interrupt",
      });
      // State will be updated via notification from server
    } catch (error) {
      console.error("Failed to run to breakpoint", error);
    }
  }, [client, connectionState]);

  return (
    <Box sx={{ height: "100vh", display: "flex", flexDirection: "column" }}>
      <AppBar position="static" elevation={1} color="default">
        <TopNav
          onHomeClick={() => navigate("/")}
          onDocsClick={() => navigate("/docs")}
          onAboutClick={showAbout}
          onResetLayout={handleResetLayout}
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
          active="workspace"
        />
      </AppBar>
      {connectionError && (
        <Alert severity="error" sx={{ borderRadius: 0 }}>
          {connectionError}
        </Alert>
      )}
      <Box sx={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        <Box sx={{ px: 1, pt: 1, pb: 0.5 }}>
          <Box sx={{ display: "flex", justifyContent: "center", mb: 0.5 }}>
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
          </Box>
          <Tabs
            value={currentTab}
            onChange={(_, value) => navigate(`/${value}`)}
            variant="scrollable"
            scrollButtons
            ref={tabsContainerRef}
            sx={{ border: "1px solid", borderColor: "divider", borderRadius: 1 }}
          >
            {workspaceTabs.map((tab) => (
              <Tab key={tab.value} value={tab.value} label={tab.label} />
            ))}
          </Tabs>
        </Box>
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
              <DeviceTreePanel />
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
              display: "flex",
              flexDirection: "column",
              gap: 1,
              flex: 1,
              borderRadius: 1,
              border: "1px solid",
              borderColor: "divider",
            }}
          >
            <Box sx={{ flex: 1, minHeight: 0, p: 1.5, display: "flex" }}>
              {workspaceTabs.map((tab) => (
                <Box
                  key={tab.value}
                  role="tabpanel"
                  hidden={currentTab !== tab.value}
                  sx={{
                    height: "100%",
                    minHeight: 0,
                    flex: 1,
                    display: currentTab === tab.value ? "flex" : "none",
                    flexDirection: "column",
                    minWidth: "0px", maxWidth: "100%"
                  }}
                >
                  {currentTab === tab.value && (
                    <Box sx={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
                      {tab.component}
                    </Box>
                  )}
                </Box>
              ))}
            </Box>
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
            <Box
              sx={{
                display: "grid",
                gridTemplateRows: "minmax(0, 2fr) minmax(0, 1fr) minmax(0, 1fr)",
                gap: 1,
                minHeight: 0,
                width: 340,
              }}
            >
              <WatchesPanel showTitle={true} />
              <Sh4CallstackPanel showTitle={true} />
              <Arm7CallstackPanel showTitle={true} />
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
        {session && (
          <Typography variant="caption" sx={{ display: "flex", gap: 1 }}>
            <span>Session ID:</span>
            <span>{session.sessionId}</span>
          </Typography>
        )}
        <Divider orientation="vertical" flexItem />
        <Typography variant="caption">Endpoint: {endpoint ?? "-"}</Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Typography variant="caption">nullDC Debugger UI prototype</Typography>
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
