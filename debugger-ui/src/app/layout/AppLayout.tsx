import { useEffect, useCallback, useMemo } from "react";
import { AppBar, Box, Button, Divider, IconButton, Tab, Tabs, Toolbar, Tooltip, Typography, Alert } from "@mui/material";
import RefreshIcon from "@mui/icons-material/Refresh";
import PowerSettingsNewIcon from "@mui/icons-material/PowerSettingsNew";
import CloudDoneIcon from "@mui/icons-material/CloudDone";
import CloudOffIcon from "@mui/icons-material/CloudOff";
import SyncIcon from "@mui/icons-material/Sync";
import { useSessionStore } from "../../state/sessionStore";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import { DeviceTreePanel } from "../panels/DeviceTreePanel";
import { WatchPanel } from "../panels/WatchPanel";
import { EventLogPanel } from "../panels/EventLogPanel";
import { Sh4DisassemblyPanel, Arm7DisassemblyPanel, DspDisassemblyPanel } from "../panels/DisassemblyPanel";
import { Sh4MemoryPanel, Arm7MemoryPanel } from "../panels/MemoryPanel";
import { AudioPanel } from "../panels/AudioPanel";
import { ThreadsPanel } from "../panels/ThreadsPanel";
import { TaInspectorPanel } from "../panels/TaInspectorPanel";
import { CoreInspectorPanel } from "../panels/CoreInspectorPanel";
import { BreakpointsPanel } from "../panels/BreakpointsPanel";
import { Sh4SimPanel } from "../panels/Sh4SimPanel";
import { useNavigate, useParams } from "react-router-dom";

const workspaceTabs = [
  { value: "events", label: "Event Log", component: <EventLogPanel /> },
  { value: "sh4-disassembly", label: "SH4: Disassembly", component: <Sh4DisassemblyPanel /> },
  { value: "sh4-memory", label: "SH4: Memory", component: <Sh4MemoryPanel /> },
  { value: "arm7-disassembly", label: "ARM7: Disassembly", component: <Arm7DisassemblyPanel /> },
  { value: "arm7-memory", label: "ARM7: Memory", component: <Arm7MemoryPanel /> },
  { value: "breakpoints", label: "Breakpoints", component: <BreakpointsPanel /> },
  { value: "ta", label: "TA", component: <TaInspectorPanel /> },
  { value: "core", label: "CORE", component: <CoreInspectorPanel /> },
  { value: "aica", label: "AICA", component: <AudioPanel /> },
  { value: "dsp-disassembly", label: "DSP: Disassembly", component: <DspDisassemblyPanel /> },
  { value: "sh4-sim", label: "SH4: Sim", component: <Sh4SimPanel /> },
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
  const validValues = useMemo(() => new Set(workspaceTabs.map(t => t.value)), []);
  const currentTab = validValues.has(tab ?? "") ? (tab as string) : workspaceTabs[0].value;

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
          <Divider orientation="vertical" flexItem />
          <Typography variant="body2" color="text.secondary">
            {endpoint ?? "resolving connection"}
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
          display: "grid",
          gridTemplateColumns: "280px minmax(0, 1fr) 340px",
          gap: 1,
          p: 1,
        }}
      >
        <Box sx={{ minHeight: 0 }}>
          <DeviceTreePanel />
        </Box>
        <Box
          sx={{
            minHeight: 0,
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
        <Box
          sx={{
            display: "grid",
            gridTemplateRows: "minmax(0, 2fr) minmax(0, 1fr) minmax(0, 1fr)",
            gap: 1,
            minHeight: 0,
          }}
        >
          <WatchPanel />
          <ThreadsPanel />
          <AudioPanel />
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
    </Box>
  );
};









