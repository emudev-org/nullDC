import { Panel } from "../layout/Panel";
import { Typography } from "@mui/material";

export const TaInspectorPanel = () => {
  return (
    <Panel title="TA">
      <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
        This view is under consideration.
      </Typography>
    </Panel>
  );
};
