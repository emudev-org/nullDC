import { useCallback, useEffect, useState } from "react";
import { Panel } from "../layout/Panel";
import { Box, Button, CircularProgress, Stack, Typography } from "@mui/material";
import type { DisassemblyLine } from "../../lib/debuggerSchema";
import { useSessionStore } from "../../state/sessionStore";

type DisassemblyConfig = {
  title: string;
  target: string;
  defaultAddress: number;
  count: number;
  context?: number;
};

const DisassemblyView = ({ title, target, defaultAddress, count, context }: DisassemblyConfig) => {
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
      const result = await client.fetchDisassembly({ target, address: defaultAddress, count, context });
      setLines(result.lines);
    } catch (error) {
      console.error(`Failed to fetch ${target} disassembly`, error);
    } finally {
      setLoading(false);
    }
  }, [client, connectionState, target, defaultAddress, count, context]);

  useEffect(() => {
    void fetchDisassembly();
  }, [fetchDisassembly]);

  return (
    <Panel
      title={title}
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
              key={`${target}-${line.address}`}
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

export const Sh4DisassemblyPanel = () => (
  <DisassemblyView title="SH4: Disassembly" target="sh4" defaultAddress={0x8c0000a0} count={32} context={4} />
);

export const Arm7DisassemblyPanel = () => (
  <DisassemblyView title="ARM7: Disassembly" target="arm7" defaultAddress={0x00200000} count={24} context={4} />
);

export const DspDisassemblyPanel = () => (
  <DisassemblyView title="DSP: Disassembly" target="dsp" defaultAddress={0x00000000} count={32} context={2} />
);
