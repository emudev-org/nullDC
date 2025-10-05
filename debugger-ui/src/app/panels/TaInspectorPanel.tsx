import { useMemo } from "react";
import { Panel } from "../layout/Panel";
import { List, ListItem, ListItemText, Typography } from "@mui/material";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

export const TaInspectorPanel = () => {
  const frameLog = useDebuggerDataStore((state) => state.frameLog);
  const entries = useMemo(
    () => (Array.isArray(frameLog) ? frameLog.filter((entry) => entry.subsystem === "ta") : []),
    [frameLog],
  );

  return (
    <Panel title="TA Debugger">
      {entries.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          No TA activity recorded for the current session.
        </Typography>
      ) : (
        <List dense disablePadding>
          {entries
            .slice(-10)
            .reverse()
            .map((entry, idx) => (
              <ListItem key={`${entry.timestamp}-${idx}`}>
                <ListItemText
                  primary={entry.message}
                  secondary={new Date(entry.timestamp).toLocaleTimeString(undefined, { hour12: false })}
                />
              </ListItem>
            ))}
        </List>
      )}
    </Panel>
  );
};
