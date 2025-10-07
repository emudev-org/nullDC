import { useEffect, useState } from "react";
import { Panel } from "../layout/Panel";
import { Box, IconButton, List, ListItem, ListItemText, Stack, Tooltip, Typography } from "@mui/material";
import RefreshIcon from "@mui/icons-material/Refresh";
import { useSessionStore } from "../../state/sessionStore";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import type { CallstackFrame } from "../../lib/debuggerSchema";

interface CallstackPanelProps {
  target: "sh4" | "arm7";
  showTitle?: boolean;
}

const CallstackPanel = ({ target, showTitle = false }: CallstackPanelProps) => {
  const client = useSessionStore((s) => s.client);
  const initialized = useDebuggerDataStore((s) => s.initialized);
  const [frames, setFrames] = useState<CallstackFrame[] | null>(null);
  const [loading, setLoading] = useState(false);

  const title = target === "sh4" ? "SH4: Callstack" : "ARM7: Callstack";

  const refresh = async () => {
    if (!client) return;
    setLoading(true);
    try {
      const res = await client.fetchCallstack(target, 32);
      setFrames(res.frames);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (initialized) {
      void refresh();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client, initialized]);

  return (
    <Panel
      title={showTitle ? title : undefined}
      action={
        <Tooltip title="Refresh">
          <IconButton size="small" onClick={refresh} disabled={loading}>
            <RefreshIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      }
    >
      {!initialized ? (
        <Stack alignItems="center" justifyContent="center" sx={{ height: "100%" }}>
          <Typography variant="body2" color="text.secondary">
            No Data
          </Typography>
        </Stack>
      ) : !frames ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          Loading…
        </Typography>
      ) : frames.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          Empty callstack.
        </Typography>
      ) : (
        <Box sx={{ p: 1 }}>
          <List dense disablePadding>
            {frames.map((f) => (
              <ListItem key={f.index} sx={{ py: 0.25 }}>
                <ListItemText
                  primary={
                    <Stack direction="row" spacing={1} alignItems="baseline" sx={{ flexWrap: "wrap" }}>
                      <Typography variant="caption" color="text.secondary" sx={{ minWidth: 28 }}>
                        #{f.index}
                      </Typography>
                      <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                        0x{f.pc.toString(16)}
                      </Typography>
                      {f.symbol && (
                        <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                          {f.symbol}
                        </Typography>
                      )}
                      {f.location && (
                        <Typography variant="body2" color="text.secondary" sx={{ fontFamily: "monospace" }}>
                          {f.location}
                        </Typography>
                      )}
                    </Stack>
                  }
                />
              </ListItem>
            ))}
          </List>
        </Box>
      )}
    </Panel>
  );
};

export default CallstackPanel;


