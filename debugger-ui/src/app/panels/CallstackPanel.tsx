import { useEffect, useState } from "react";
import { Panel } from "../layout/Panel";
import { Box, IconButton, List, ListItem, ListItemText, Stack, Tooltip, Typography } from "@mui/material";
import RefreshIcon from "@mui/icons-material/Refresh";
import { useSessionStore } from "../../state/sessionStore";
import type { CallstackFrame } from "../../lib/debuggerSchema";

const CallstackView = ({ title, target, showTitle = false }: { title: string; target: "sh4" | "arm7"; showTitle?: boolean }) => {
  const client = useSessionStore((s) => s.client);
  const connectionState = useSessionStore((s) => s.connectionState);
  const [frames, setFrames] = useState<CallstackFrame[] | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = async () => {
    if (!client || connectionState !== "connected") return;
    setLoading(true);
    try {
      const res = await client.fetchCallstack(target, 32);
      setFrames(res.frames);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client, connectionState]);

  return (
    <Panel
      title={showTitle ? title : undefined}
      action={
        <Tooltip title="Refresh">
          <span>
            <IconButton size="small" onClick={refresh} disabled={connectionState !== "connected" || loading}>
              <RefreshIcon fontSize="small" />
            </IconButton>
          </span>
        </Tooltip>
      }
    >
      {!frames ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          {connectionState === "connected" ? "Loading…" : "Not connected."}
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

export const Sh4CallstackPanel = ({ showTitle = false }: { showTitle?: boolean }) => <CallstackView title="SH4: Callstack" target="sh4" showTitle={showTitle} />;
export const Arm7CallstackPanel = ({ showTitle = false }: { showTitle?: boolean }) => <CallstackView title="ARM7: Callstack" target="arm7" showTitle={showTitle} />;


