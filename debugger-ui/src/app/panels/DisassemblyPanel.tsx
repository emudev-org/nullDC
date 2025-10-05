import { useCallback, useEffect, useState } from "react";
import { Panel } from "../layout/Panel";
import { Box, Button, CircularProgress, Stack, Typography } from "@mui/material";
import type { DisassemblyLine } from "../../lib/debuggerSchema";
import { useSessionStore } from "../../state/sessionStore";

const DEFAULT_ADDRESS = 0x8c0000a0;
const DEFAULT_COUNT = 32;

export const DisassemblyPanel = () => {
  const client = useSessionStore((state) => state.client);
  const connectionState = useSessionStore((state) => state.connectionState);
  const [lines, setLines] = useState<DisassemblyLine[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchDisassembly = useCallback(async () => {
    if (!client || connectionState !== "connected") {
      return;
    }
    setLoading(true);
    try {
      const result = await client.fetchDisassembly(DEFAULT_ADDRESS, DEFAULT_COUNT, 4);
      setLines(result.lines);
    } catch (error) {
      console.error("Failed to fetch disassembly", error);
    } finally {
      setLoading(false);
    }
  }, [client, connectionState]);

  useEffect(() => {
    void fetchDisassembly();
  }, [fetchDisassembly]);

  return (
    <Panel
      title="Disassembly"
      action={
        <Button size="small" onClick={() => void fetchDisassembly()} disabled={loading || connectionState !== "connected"}>
          Refresh
        </Button>
      }
    >
      {loading && lines.length === 0 ? (
        <Stack alignItems="center" justifyContent="center" sx={{ height: "100%" }} spacing={1}>
          <CircularProgress size={18} />
          <Typography variant="body2" color="text.secondary">
            Loading disassembly…
          </Typography>
        </Stack>
      ) : lines.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          Disassembly unavailable.
        </Typography>
      ) : (
        <Box component="pre" sx={{ fontFamily: "monospace", fontSize: 13, m: 0, p: 1.5 }}>
          {lines.map((line) => (
            <Typography
              key={line.address}
              component="div"
              sx={{
                display: "flex",
                gap: 2,
                color: line.isCurrent ? "primary.main" : "inherit",
              }}
            >
              <span>{`0x${line.address.toString(16).toUpperCase().padStart(8, "0")}`}</span>
              <span>{line.bytes.padEnd(11, " ")}</span>
              <span>{line.mnemonic.padEnd(8, " ")}</span>
              <span>{line.operands}</span>
              {line.comment && (
                <span style={{ color: "var(--mui-palette-text-secondary)" }}>; {line.comment}</span>
              )}
            </Typography>
          ))}
        </Box>
      )}
    </Panel>
  );
};
