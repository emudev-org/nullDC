import { Panel } from "../layout/Panel";
import { Box, Typography } from "@mui/material";

export const CoreInspectorPanel = () => (
  <Panel title="CORE Inspector">
    <Box sx={{ p: 2, display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 1 }}>
      {[
        { label: "Visibility Pass", value: "18 lists" },
        { label: "Opaque Pass", value: "120 primitives" },
        { label: "Translucent Pass", value: "34 primitives" },
        { label: "Tile Cache", value: "7 tiles" },
      ].map((entry) => (
        <Box
          key={entry.label}
          sx={{
            p: 1,
            borderRadius: 1,
            border: "1px solid",
            borderColor: "divider",
          }}
        >
          <Typography variant="caption" color="text.secondary">
            {entry.label}
          </Typography>
          <Typography variant="body2" fontWeight={600}>
            {entry.value}
          </Typography>
        </Box>
      ))}
    </Box>
  </Panel>
);
