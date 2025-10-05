import { useEffect, useCallback, useMemo, useState } from "react";
import { AppBar, Box, Button, Divider, IconButton, Tab, Tabs, Toolbar, Tooltip, Typography, Alert } from "@mui/material";
import RefreshIcon from "@mui/icons-material/Refresh";
import PowerSettingsNewIcon from "@mui/icons-material/PowerSettingsNew";
import CloudDoneIcon from "@mui/icons-material/CloudDone";
import CloudOffIcon from "@mui/icons-material/CloudOff";
import SyncIcon from "@mui/icons-material/Sync";
import ChevronLeftIcon from "@mui/icons-material/ChevronLeft";
import ChevronRightIcon from "@mui/icons-material/ChevronRight";
import { useSessionStore } from "../../state/sessionStore";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import { DeviceTreePanel } from "../panels/DeviceTreePanel";
import { WatchPanel } from "../panels/WatchPanel";
import { EventLogPanel } from "../panels/EventLogPanel";
import { Sh4DisassemblyPanel, Arm7DisassemblyPanel, DspDisassemblyPanel } from "../panels/DisassemblyPanel";
import { Sh4MemoryPanel, Arm7MemoryPanel } from "../panels/MemoryPanel";
import { Sh4CallstackPanel, Arm7CallstackPanel } from "../panels/CallstackPanel";
import { AudioPanel } from "../panels/AudioPanel";
import { TaInspectorPanel } from "../panels/TaInspectorPanel";
import { CoreInspectorPanel } from "../panels/CoreInspectorPanel";
import { EventsBreakpointsPanel, Sh4BreakpointsPanel, Arm7BreakpointsPanel } from "../panels/BreakpointsPanel";
import { Sh4SimPanel } from "../panels/Sh4SimPanel";
import { useNavigate, useParams } from "react-router-dom";

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
  { value: "sh4-sim", label: "SH4: Sim", component: <Sh4SimPanel /> },
];

const sidePanelTabs = [
  { value: "device-tree", label: "Device Tree", component: <DeviceTreePanel /> },
  { value: "watch", label: "Watch", component: <WatchPanel /> },
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

  const handleReconnect = useCallback(() => {
    void connect({ force: true });
  }, [connect]);

  const handleDisconnect = useCallback(() => {
    disconnect();
  }, [disconnect]);

  return (
    <Box sx={{ height: "100vh", display: "flex", flexDirection: "column" }}>
      <AppBar position="static" elevation={1} color="default">
        <Toolbar sx={{ gap: 2 }}>
          <Typography variant="h6" sx={{ flexShrink: 0 }}>
            nullDC Debugger
          </Typography>
          <Box sx={{ flexGrow: 1 }} />
          <Tooltip title={`Connection: ${connectionState}`}>
            <IconButton color={connectionState === "connected" ? "primary" : "inherit"}>
              {connectionIcons[connectionState]}
            </IconButton>
          </Tooltip>
          <IconButton color="inherit" onClick={handleReconnect} aria-label="Reconnect">
            <RefreshIcon fontSize="small" />
          </IconButton>
          <Button
            variant="outlined"
            color="inherit"
            onClick={handleDisconnect}
            startIcon={<PowerSettingsNewIcon fontSize="small" />}
          >
            Disconnect
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
            <WatchPanel />
            <Sh4CallstackPanel />
            <Arm7CallstackPanel />
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
    </Box>
  );
};









