import { useCallback, useState } from "react";
import {
  AppBar,
  Box,
  Button,
  Divider,
  IconButton,
  Tab,
  Tabs,
  TextField,
  Toolbar,
  Tooltip,
  Typography,
  ToggleButton,
  ToggleButtonGroup,
  Alert,
} from "@mui/material";
import RefreshIcon from "@mui/icons-material/Refresh";
import PowerSettingsNewIcon from "@mui/icons-material/PowerSettingsNew";
import CloudDoneIcon from "@mui/icons-material/CloudDone";
import CloudOffIcon from "@mui/icons-material/CloudOff";
import SyncIcon from "@mui/icons-material/Sync";
import { useSessionStore } from "../../state/sessionStore";
import { DeviceTreePanel } from "../panels/DeviceTreePanel";
import { WatchPanel } from "../panels/WatchPanel";
import { MemoryPanel } from "../panels/MemoryPanel";
import { DisassemblyPanel } from "../panels/DisassemblyPanel";
import { FrameLogPanel } from "../panels/FrameLogPanel";
import { AudioPanel } from "../panels/AudioPanel";
import { ThreadsPanel } from "../panels/ThreadsPanel";
import { TaInspectorPanel } from "../panels/TaInspectorPanel";
import { CoreInspectorPanel } from "../panels/CoreInspectorPanel";
import { DspPanel } from "../panels/DspPanel";
import { BreakpointsPanel } from "../panels/BreakpointsPanel";

const workspaceTabs = [
  { value: "disassembly", label: "Disassembly", component: <DisassemblyPanel /> },
  { value: "memory", label: "Memory", component: <MemoryPanel /> },
  { value: "ta", label: "TA", component: <TaInspectorPanel /> },
  { value: "core", label: "CORE", component: <CoreInspectorPanel /> },
  { value: "aica", label: "AICA", component: <AudioPanel /> },
  { value: "dsp", label: "DSP", component: <DspPanel /> },
];

const connectionIcons = {
  idle: <CloudOffIcon fontSize="small" />,
  error: <CloudOffIcon fontSize="small" />,
  connecting: <SyncIcon fontSize="small" className="spin" />,
  connected: <CloudDoneIcon fontSize="small" />,
};

export const AppLayout = () => {
  const { connect, disconnect, connectionState, connectionError, mode, setMode } = useSessionStore();
  const [endpoint, setEndpoint] = useState("ws://127.0.0.1:9000/ws");
  const [channelName, setChannelName] = useState("nulldc-debugger");
  const [activeTab, setActiveTab] = useState(workspaceTabs[0].value);

  const handleConnect = useCallback(() => {
    connect({
      mode,
      endpoint: mode === "native" ? endpoint : channelName,
      clientName: "nullDC UI",
      clientVersion: "0.1.0",
      transportOptions: mode === "wasm" ? { channelName } : undefined,
    });
  }, [connect, mode, endpoint, channelName]);

  const handleDisconnect = useCallback(() => {
    disconnect();
  }, [disconnect]);

  const disabled = connectionState === "connecting";

  return (
    <Box sx={{ height: "100vh", display: "flex", flexDirection: "column" }}>
      <AppBar position="static" elevation={1} color="default">
        <Toolbar sx={{ gap: 2 }}>
          <Typography variant="h6" sx={{ flexShrink: 0 }}>
            nullDC Debugger
          </Typography>
          <Divider orientation="vertical" flexItem />
          <ToggleButtonGroup
            size="small"
            exclusive
            value={mode}
            onChange={(_, next) => next && setMode(next)}
          >
            <ToggleButton value="native">Native</ToggleButton>
            <ToggleButton value="wasm">WASM</ToggleButton>
          </ToggleButtonGroup>
          {mode === "native" ? (
            <TextField
              label="WebSocket"
              size="small"
              sx={{ minWidth: 260 }}
              value={endpoint}
              onChange={(event) => setEndpoint(event.target.value)}
            />
          ) : (
            <TextField
              label="Channel"
              size="small"
              sx={{ minWidth: 220 }}
              value={channelName}
              onChange={(event) => setChannelName(event.target.value)}
            />
          )}
          <Button
            variant="contained"
            color="primary"
            onClick={handleConnect}
            disabled={disabled}
            startIcon={<SyncIcon fontSize="small" />}
          >
            Connect
          </Button>
          <Button
            variant="outlined"
            color="inherit"
            onClick={handleDisconnect}
            startIcon={<PowerSettingsNewIcon fontSize="small" />}
          >
            Disconnect
          </Button>
          <Box sx={{ flexGrow: 1 }} />
          <Tooltip title={`Connection: ${connectionState}`}>
            <IconButton color={connectionState === "connected" ? "primary" : "inherit"}>
              {connectionIcons[connectionState]}
            </IconButton>
          </Tooltip>
          <IconButton color="inherit">
            <RefreshIcon fontSize="small" />
          </IconButton>
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
          }}
        >
          <Box sx={{ borderRadius: 1, border: "1px solid", borderColor: "divider" }}>
            <Tabs
              value={activeTab}
              onChange={(_, value) => setActiveTab(value)}
              variant="scrollable"
              scrollButtons="auto"
              sx={{ borderBottom: "1px solid", borderColor: "divider" }}
            >
              {workspaceTabs.map((tab) => (
                <Tab key={tab.value} value={tab.value} label={tab.label} />
              ))}
            </Tabs>
            <Box sx={{ p: 1.5, height: "calc(100% - 48px)", minHeight: 0 }}>
              {workspaceTabs.map((tab) => (
                <Box
                  key={tab.value}
                  role="tabpanel"
                  hidden={activeTab !== tab.value}
                  sx={{ height: "100%" }}
                >
                  {activeTab === tab.value && <Box sx={{ height: "100%" }}>{tab.component}</Box>}
                </Box>
              ))}
            </Box>
          </Box>
          <Box sx={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 1, minHeight: 0 }}>
            <BreakpointsPanel />
            <FrameLogPanel />
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
        <Typography variant="caption">Session: {connectionState}</Typography>
        <Divider orientation="vertical" flexItem />
        <Typography variant="caption">Mode: {mode.toUpperCase()}</Typography>
        <Box sx={{ flexGrow: 1 }} />
        <Typography variant="caption">nullDC Debugger UI prototype</Typography>
      </Box>
    </Box>
  );
};
