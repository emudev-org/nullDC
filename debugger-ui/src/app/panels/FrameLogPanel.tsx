import { Panel } from "../layout/Panel";
import { List, ListItem, ListItemText, Typography, Chip, Stack } from "@mui/material";

const entries = [
  { subsystem: "ta", message: "Processed tile 12", severity: "info", timestamp: 0.16 },
  { subsystem: "core", message: "Primitive batch committed", severity: "trace", timestamp: 0.18 },
  { subsystem: "aica", message: "DSP step", severity: "trace", timestamp: 0.19 },
];

const severityColor: Record<string, "default" | "primary" | "warning" | "error"> = {
  trace: "default",
  info: "primary",
  warn: "warning",
  error: "error",
};

export const FrameLogPanel = () => {
  return (
    <Panel title="Frame Log">
      <List dense disablePadding>
        {entries.map((entry, idx) => (
          <ListItem key={`${entry.subsystem}-${idx}`} sx={{ alignItems: "flex-start" }}>
            <ListItemText
              primary={
                <Stack direction="row" spacing={1} alignItems="center">
                  <Chip size="small" label={entry.subsystem.toUpperCase()} color="secondary" />
                  <Typography variant="caption" color="text.secondary">
                    {`${entry.timestamp.toFixed(2)} ms`}
                  </Typography>
                </Stack>
              }
              secondary={
                <Typography variant="body2" color={severityColor[entry.severity] ?? "inherit"}>
                  {entry.message}
                </Typography>
              }
            />
          </ListItem>
        ))}
      </List>
    </Panel>
  );
};
