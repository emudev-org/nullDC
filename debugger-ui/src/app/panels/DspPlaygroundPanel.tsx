import { Panel } from "../layout/Panel";
import { Box, Typography } from "@mui/material";

export const DspPlaygroundPanel = () => {
  return (
    <Panel title="DSP: Playground">
      <Box sx={{ p: 3 }}>
        <Typography variant="body1" color="text.secondary">
          This view is under consideration.
        </Typography>
      </Box>
    </Panel>
  );
};
