import { useMemo } from "react";
import { Panel } from "../layout/Panel";
import { Chip, List, ListItem, ListItemText, Stack, Typography } from "@mui/material";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

const severityColor: Record<string, "default" | "primary" | "warning" | "error"> = {
  trace: "default",
  info: "primary",
  warn: "warning",
  error: "error",
};

export const EventLogPanel = () => {
  const entries = useDebuggerDataStore((state) => state.frameLog);
  const rendered = useMemo(() => (Array.isArray(entries) ? entries.slice().reverse() : []), [entries]);

  return (
    <Panel title="Event Log">
      {rendered.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          No runtime events yet.
        </Typography>
      ) : (
        <List dense disablePadding sx={{ maxHeight: "100%" }}>
          {rendered.map((entry) => (
            <ListItem key={entry.eventId} sx={{ alignItems: "flex-start" }}>
              <ListItemText
                primary={
                  <Typography variant="caption" color="text.secondary">
                    {new Date(entry.timestamp).toLocaleTimeString(undefined, { hour12: false })}
                  </Typography>
                }
                secondary={
                  <Stack direction="row" spacing={1} alignItems="center" component="span">
                    <Chip
                      size="small"
                      label={entry.subsystem.toUpperCase()}
                      color={entry.subsystem === "ta" ? "secondary" : "default"}
                    />
                    <Typography
                      component="span"
                      variant="body2"
                      color={severityColor[entry.severity] ?? "inherit"}
                    >
                      {entry.message}
                    </Typography>
                  </Stack>
                }
                secondaryTypographyProps={{ component: "div" }}
              />
            </ListItem>
          ))}
        </List>
      )}
    </Panel>
  );
};

