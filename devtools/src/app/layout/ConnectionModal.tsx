import { useState, useEffect } from "react";
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  Radio,
  RadioGroup,
  FormControlLabel,
  TextField,
  Box,
  Typography,
  Alert,
  Stack,
} from "@mui/material";
import { useSessionStore } from "../../state/sessionStore";

interface ConnectionModalProps {
  open: boolean;
  onClose: () => void;
}

export const ConnectionModal = ({ open, onClose }: ConnectionModalProps) => {
  const mode = useSessionStore((state) => state.mode);
  const availableConnections = useSessionStore((state) => state.availableConnections);
  const selectedConnectionId = useSessionStore((state) => state.selectedConnectionId);
  const setSelectedConnectionId = useSessionStore((state) => state.setSelectedConnectionId);
  const connect = useSessionStore((state) => state.connect);

  const [manualEntry, setManualEntry] = useState("");
  const [useManual, setUseManual] = useState(false);

  useEffect(() => {
    // Reset manual entry when modal opens
    if (open) {
      setManualEntry("");
      setUseManual(false);
    }
  }, [open]);

  const handleConnect = async () => {
    if (useManual && manualEntry.trim()) {
      // Manual connection
      await connect({ connectionId: manualEntry.trim(), force: true });
    } else if (selectedConnectionId) {
      // Selected from list
      await connect({ connectionId: selectedConnectionId, force: true });
    }
    onClose();
  };

  const handleSelectionChange = (id: string) => {
    setSelectedConnectionId(id);
    setUseManual(false);
  };

  const handleManualClick = () => {
    setUseManual(true);
    setSelectedConnectionId(undefined);
  };

  const formatLastSeen = (timestamp: number) => {
    const seconds = Math.floor((Date.now() - timestamp) / 1000);
    if (seconds < 2) return "just now";
    if (seconds < 60) return `${seconds}s ago`;
    return `${Math.floor(seconds / 60)}m ago`;
  };

  const canConnect = useManual ? manualEntry.trim().length > 0 : !!selectedConnectionId;

  return (
    <Dialog open={open} onClose={onClose} maxWidth="sm" fullWidth>
      <DialogTitle>Select Connection</DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ pt: 1 }}>
          {availableConnections.length === 0 && (
            <Alert severity="info">
              {mode === "wasm"
                ? "No emulator instances detected. Start the emulator and wait for announcements."
                : "No WebSocket connections available."}
            </Alert>
          )}

          {availableConnections.length > 0 && (
            <Box>
              <Typography variant="subtitle2" sx={{ mb: 1 }}>
                Available Connections
              </Typography>
              <RadioGroup
                value={useManual ? "" : selectedConnectionId || ""}
                onChange={(e) => handleSelectionChange(e.target.value)}
              >
                {availableConnections.map((conn) => (
                  <FormControlLabel
                    key={conn.id}
                    value={conn.id}
                    control={<Radio />}
                    label={
                      <Box>
                        <Typography variant="body2">{conn.name}</Typography>
                        <Typography variant="caption" color="text.secondary">
                          {conn.mode === "wasm" ? `GUID: ${conn.id.substring(0, 8)}...` : conn.id} •{" "}
                          {formatLastSeen(conn.lastSeen)}
                        </Typography>
                      </Box>
                    }
                  />
                ))}
              </RadioGroup>
            </Box>
          )}

          <Box>
            <Typography variant="subtitle2" sx={{ mb: 1 }}>
              Manual Connection
            </Typography>
            <FormControlLabel
              control={<Radio checked={useManual} onChange={handleManualClick} />}
              label="Enter connection string manually"
            />
            {useManual && (
              <TextField
                fullWidth
                size="small"
                placeholder={
                  mode === "wasm" ? "Enter GUID (e.g., abc123...)" : "Enter WebSocket URL (e.g., ws://localhost:55543/ws)"
                }
                value={manualEntry}
                onChange={(e) => setManualEntry(e.target.value)}
                sx={{ mt: 1 }}
                autoFocus
              />
            )}
          </Box>
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Cancel</Button>
        <Button onClick={handleConnect} variant="contained" disabled={!canConnect}>
          Connect
        </Button>
      </DialogActions>
    </Dialog>
  );
};
