import { Panel } from "../layout/Panel";
import { Box, Button, Stack, Typography } from "@mui/material";

export const DspPanel = () => (
  <Panel
    title="DSP Inspector"
    action={
      <Stack direction="row" spacing={1}>
        <Button size="small" variant="outlined">Step</Button>
        <Button size="small" variant="contained">Run</Button>
      </Stack>
    }
  >
    <Box sx={{ p: 2 }}>
      <Typography variant="body2" fontFamily="monospace">
        ACC: 0x1F	SAR: 0x03	PC: 0x12
      </Typography>
      <Typography variant="body2" fontFamily="monospace">
        TEMP: [0x0A, 0x04, 0xFF, 0x10]
      </Typography>
    </Box>
  </Panel>
);
