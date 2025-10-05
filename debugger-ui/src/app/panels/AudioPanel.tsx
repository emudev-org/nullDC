import { useMemo } from "react";
import { Panel } from "../layout/Panel";
import { Box, Stack, Typography } from "@mui/material";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

export const AudioPanel = () => {
  const waveform = useDebuggerDataStore((state) => state.waveform);

  const points = useMemo(() => {
    if (!waveform || waveform.samples.length === 0) {
      return "";
    }
    return waveform.samples
      .map((sample, index) => {
        const x = (index / (waveform.samples.length - 1)) * 100;
        const y = 20 - sample * 18;
        return `${x},${y}`;
      })
      .join(" ");
  }, [waveform]);

  return (
    <Panel title="AICA Waveform">
      {!waveform ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          Waiting for waveform data…
        </Typography>
      ) : (
        <Stack sx={{ p: 2, height: "100%" }} spacing={1}>
          <Typography variant="caption" color="text.secondary">
            Channel {waveform.channelId} • {waveform.sampleRate} Hz • {waveform.format}
          </Typography>
          <Box component="svg" viewBox="0 0 100 40" preserveAspectRatio="none" sx={{ width: "100%", height: "100%" }}>
            <polyline fill="none" strokeWidth={1} stroke="var(--mui-palette-primary-light, #81d4fa)" points={points} />
          </Box>
        </Stack>
      )}
    </Panel>
  );
};
