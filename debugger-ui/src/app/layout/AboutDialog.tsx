import { useEffect, useState } from "react";
import {
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  Stack,
  Typography,
} from "@mui/material";
import { useSessionStore } from "../../state/sessionStore";
import { DEBUGGER_VERSION } from "./aboutVersion";

type EmulatorInfo = {
  name?: string;
  version?: string;
  build?: string;
};

interface AboutDialogProps {
  open: boolean;
  onClose: () => void;
}

export const AboutDialog = ({ open, onClose }: AboutDialogProps) => {
  const client = useSessionStore((state) => state.client);
  const connectionState = useSessionStore((state) => state.connectionState);
  const [info, setInfo] = useState<EmulatorInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    if (connectionState !== "connected") {
      setLoading(false);
      return;
    }
    if (!client) {
      return;
    }
    if (info) {
      return;
    }
    let cancelled = false;
    setLoading(true);
    setError(null);
    void client
      .fetchEmulatorInfo()
      .then((result) => {
        if (cancelled) {
          return;
        }
        if (result) {
          setInfo(result);
        }
      })
      .catch((err) => {
        if (cancelled) {
          return;
        }
        setError(err instanceof Error ? err.message : String(err));
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [open, client, connectionState, info]);

  const emulatorRows = [
    { label: "Name", value: info?.name ?? "—" },
    { label: "Version", value: info?.version ?? "—" },
    { label: "Build", value: info?.build ?? "—" },
  ];

  useEffect(() => {
    if (!open) {
      setInfo(null);
      setError(null);
      setLoading(false);
    }
  }, [open]);

  return (
    <Dialog open={open} onClose={onClose} maxWidth="xs" fullWidth>
      <DialogTitle>About nullDC Debugger</DialogTitle>
      <DialogContent dividers>
        <Stack spacing={1.5}>
          <Typography variant="body2" color="text.secondary">
            A modern web UI vibed together with LLMs.
          </Typography>
          <Divider />
          {connectionState !== "connected" ? (
            <Typography variant="body2" color="text.secondary">
              Connect to the debugger to view emulator details.
            </Typography>
          ) : loading ? (
            <Box sx={{ display: "flex", alignItems: "center", gap: 2 }}>
              <CircularProgress size={20} />
              <Typography variant="body2" color="text.secondary">
                Loading emulator information…
              </Typography>
            </Box>
          ) : error ? (
            <Typography variant="body2" color="error">
              Failed to load emulator info: {error}
            </Typography>
          ) : (
            <Stack spacing={1.5}>
              {emulatorRows.map((row) => (
                <Stack key={row.label} direction="row" justifyContent="space-between">
                  <Typography variant="body2" fontWeight={600}>
                    {row.label}
                  </Typography>
                  <Typography variant="body2" color="text.secondary">
                    {row.value}
                  </Typography>
                </Stack>
              ))}
            </Stack>
          )}
          <Stack direction="row" justifyContent="space-between">
            <Typography variant="body2" fontWeight={600}>
              Debugger
            </Typography>
            <Typography variant="body2" color="text.secondary">
              {DEBUGGER_VERSION}
            </Typography>
          </Stack>
          <Typography variant="caption" color="text.disabled">
            Connection state: {connectionState}
          </Typography>
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Close</Button>
      </DialogActions>
    </Dialog>
  );
};
