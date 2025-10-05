import { useEffect, useCallback, useMemo, useState, useRef } from "react";
import { AppBar, Box, Button, CircularProgress, Divider, IconButton, Stack, Tab, Tabs, Toolbar, Tooltip, Typography, Alert } from "@mui/material";
import PowerSettingsNewIcon from "@mui/icons-material/PowerSettingsNew";
import CloudDoneIcon from "@mui/icons-material/CloudDone";
import CloudOffIcon from "@mui/icons-material/CloudOff";
import SyncIcon from "@mui/icons-material/Sync";
import ChevronLeftIcon from "@mui/icons-material/ChevronLeft";
import ChevronRightIcon from "@mui/icons-material/ChevronRight";
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
  { value: "watches", label: "Watches", component: <WatchesPanel /> },
  { value: "sh4-callstack", label: "SH4: Callstack", component: <Sh4CallstackPanel /> },
  { value: "arm7-callstack", label: "ARM7: Callstack", component: <Arm7CallstackPanel /> },
];

const connectionIcons = {
  idle: <CloudOffIcon fontSize="small" />,
  error: <CloudOffIcon fontSize="small" />,
  connecting: <SyncIcon fontSize="small" className="spin" />,
  connected: <CloudDoneIcon fontSize="small" />,
};

export const AppLayout = () => {
  const connect = useSessionStore((state) => state.connect);
  const disconnect = useSessionStore((state) => state.disconnect);
  const connectionState = useSessionStore((state) => state.connectionState);
  const connectionError = useSessionStore((state) => state.connectionError);
  const session = useSessionStore((state) => state.session);
  const endpoint = useSessionStore((state) => state.endpoint);
  const client = useSessionStore((state) => state.client);
  const initializeData = useDebuggerDataStore((state) => state.initialize);
  const resetData = useDebuggerDataStore((state) => state.reset);
  const navigate = useNavigate();
  const { tab } = useParams();
  const [leftPanelOpen, setLeftPanelOpen] = useState(true);
  const [rightPanelOpen, setRightPanelOpen] = useState(true);
  const [isNarrow, setIsNarrow] = useState(window.innerWidth < 1200);
  const tabsContainerRef = useRef<HTMLDivElement | null>(null);
  const { open: aboutOpen, show: showAbout, hide: hideAbout } = useAboutModal();

  const workspaceTabs = useMemo(() => {
    return isNarrow ? [...mainTabs, ...sidePanelTabs] : mainTabs;
  }, [isNarrow]);

  const validValues = useMemo(() => new Set(workspaceTabs.map(t => t.value)), [workspaceTabs]);
  const currentTab = validValues.has(tab ?? "") ? (tab as string) : workspaceTabs[0].value;

  useEffect(() => {
    const handleResize = () => {
      setIsNarrow(window.innerWidth < 1200);
    };
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

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

  return (
    <Box sx={{ height: "100vh", display: "flex", flexDirection: "column" }}>
      <AppBar position="static" elevation={1} color="default">
        <Toolbar sx={{ gap: 2 }}>
          <Stack direction="row" spacing={1.5} alignItems="center" sx={{ flexShrink: 0 }}>
            <Typography variant="h6">nullDC Debugger</Typography>
            <Divider orientation="vertical" flexItem />
            <Button variant="text" color="primary" onClick={() => navigate("/")}>
              Home
            </Button>
            <Button variant="text" color="primary" onClick={showAbout}>
              About
            </Button>
            <Button
              variant="text"
              color="primary"
              onClick={() => {
                setLeftPanelOpen(true);
                setRightPanelOpen(true);
              }}
            >
              Reset layout
            </Button>
          </Stack>
          <Box sx={{ flexGrow: 1 }} />
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
        </Toolbar>
      </AppBar>
      {connectionError && (
        <Alert severity="error" sx={{ borderRadius: 0 }}>
          {connectionError}
        </Alert>
      )}
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
        {!isNarrow && leftPanelOpen && (
          <Box sx={{ minHeight: 0, width: 280 }}>
            <DeviceTreePanel />
          </Box>
        )}
        {!isNarrow && (
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
          }}
        >
          <Box sx={{ borderRadius: 1, border: "1px solid", borderColor: "divider", minHeight: 0, display: "flex", flexDirection: "column", flex: 1 }}>
            <Tabs
              value={currentTab}
              onChange={(_, value) => navigate(`/${value}`)}
              variant="scrollable"
              scrollButtons
              sx={{ borderBottom: "1px solid", borderColor: "divider" }}
              ref={tabsContainerRef}
            >
              {workspaceTabs.map((tab) => (
                <Tab key={tab.value} value={tab.value} label={tab.label} />
              ))}
            </Tabs>
            <Box sx={{ p: 1.5, height: "calc(100% - 48px)", minHeight: 0, display: "flex", flex: 1 }}>

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
        </Box>
        {!isNarrow && (
          <Box sx={{ display: "flex", alignItems: "center" }}>
            <Tooltip title={rightPanelOpen ? "Hide right panel" : "Show right panel"}>
              <IconButton onClick={() => setRightPanelOpen(!rightPanelOpen)} size="small">
                {rightPanelOpen ? <ChevronRightIcon /> : <ChevronLeftIcon />}
              </IconButton>
            </Tooltip>
          </Box>
        )}
        {!isNarrow && rightPanelOpen && (
          <Box
            sx={{
              display: "grid",
              gridTemplateRows: "minmax(0, 2fr) minmax(0, 1fr) minmax(0, 1fr)",
              gap: 1,
              minHeight: 0,
              width: 340,
            }}
          >
            <WatchesPanel />
            <Sh4CallstackPanel />
            <Arm7CallstackPanel />
          </Box>
        )}
        {connectionState !== "connected" && (
          <Box
            sx={{
              position: "absolute",
              top: 0,
              left: 0,
              right: 0,
              bottom: 0,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              backgroundColor: "rgba(0, 0, 0, 0.3)",
              backdropFilter: "blur(4px)",
              zIndex: 1000,
            }}
          >
            <Stack spacing={2} alignItems="center" sx={{ backgroundColor: "background.paper", p: 4, borderRadius: 2, boxShadow: 3 }}>
              <CircularProgress size={48} />
              <Typography variant="body1" color="text.secondary">
                {connectionState === "connecting" ? "Connecting to debugger..." : connectionState === "error" ? "Connection failed" : "Not connected"}
              </Typography>
            </Stack>
          </Box>
        )}
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
    </Box>
  );
};
