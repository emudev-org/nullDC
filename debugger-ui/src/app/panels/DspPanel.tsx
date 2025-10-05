import { Panel } from "../layout/Panel";
import { Box, Button, Stack, Typography } from "@mui/material";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

export const DspPanel = () => {
  const waveform = useDebuggerDataStore((state) => state.waveform);

  const rms =
    waveform && waveform.samples.length > 0
      ? Math.sqrt(
          waveform.samples.reduce((sum, sample) => sum + sample * sample, 0) / waveform.samples.length,
        ).toFixed(3)
      : "0.000";

  return (
    <Panel
      title="DSP Inspector"
      action={
        <Stack direction="row" spacing={1}>
          <Button size="small" variant="outlined" disabled>
            Step
          </Button>
          <Button size="small" variant="contained" disabled>
            Run
          </Button>
        </Stack>
      }
    >
      {waveform ? (
        <Box sx={{ p: 2, display: "grid", gap: 1 }}>
          <Typography variant="body2" fontFamily="monospace">
            RMS: {rms}
          </Typography>
          <Typography variant="body2" fontFamily="monospace">
            Samples: {waveform.samples.length}
          </Typography>
        </Box>
      ) : (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          Awaiting DSP waveform samples.
        </Typography>
      )}
    </Panel>
  );
};
