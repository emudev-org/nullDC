import { useMemo } from "react";
import { Panel } from "../layout/Panel";
import { Chip, List, ListItem, ListItemText, Typography } from "@mui/material";
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
          {rendered.map((entry, idx) => (
            <ListItem key={`${entry.timestamp}-${idx}`} sx={{ alignItems: "flex-start" }}>
              <ListItemText
                primary={
                  <Typography variant="caption" color="text.secondary">
                    {new Date(entry.timestamp).toLocaleTimeString(undefined, { hour12: false })}
                  </Typography>
                }
                secondary={
                  <Typography variant="body2" color={severityColor[entry.severity] ?? "inherit"}>
                    <Chip
                      size="small"
                      label={entry.subsystem.toUpperCase()}
                      color={entry.subsystem === "ta" ? "secondary" : "default"}
                      sx={{ mr: 1 }}
                    />
                    {entry.message}
                  </Typography>
                }
              />
            </ListItem>
          ))}
        </List>
      )}
    </Panel>
  );
};
