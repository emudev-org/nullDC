import { useMemo } from "react";
import { Panel } from "../layout/Panel";
import { Box, List, ListItem, Typography } from "@mui/material";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

const severityColor: Record<string, string> = {
  trace: "text.disabled",
  info: "text.primary",
  warn: "warning.main",
  error: "error.main",
};

export const EventLogPanel = () => {
  const entries = useDebuggerDataStore((state) => state.eventLog);
  const rendered = useMemo(() => (Array.isArray(entries) ? entries.slice().reverse() : []), [entries]);

  return (
    <Panel title="Event Log">
      {rendered.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          No runtime events yet.
        </Typography>
      ) : (
        <List dense disablePadding sx={{ maxHeight: "100%", fontFamily: "monospace" }}>
          {rendered.map((entry) => {
            const timestamp = new Date(entry.timestamp).toLocaleTimeString(undefined, {
              hour12: false,
              hour: "2-digit",
              minute: "2-digit",
              second: "2-digit",
              fractionalSecondDigits: 3
            });
            const tag = entry.subsystem.toUpperCase().padEnd(6, " ");

            return (
              <ListItem key={entry.eventId} sx={{ py: 0.25, px: 1 }}>
                <Typography
                  variant="body2"
                  sx={{
                    fontFamily: "monospace",
                    fontSize: "0.75rem",
                    color: severityColor[entry.severity] ?? "text.primary",
                    whiteSpace: "pre",
                  }}
                >
                  {timestamp}{" "}
                  <Box
                    component="span"
                    sx={{
                      display: "inline-block",
                      backgroundColor: "action.selected",
                      borderRadius: 1,
                      px: 0.75,
                      py: 0.25,
                      textAlign: "center",
                      minWidth: "3.5rem",
                    }}
                  >
                    {tag.trim()}
                  </Box>{" "}
                  {entry.message}
                </Typography>
              </ListItem>
            );
          })}
        </List>
      )}
    </Panel>
  );
};

