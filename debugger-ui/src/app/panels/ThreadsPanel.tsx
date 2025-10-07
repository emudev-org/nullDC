import { Panel } from "../layout/Panel";
import { List, ListItemButton, ListItemText, Typography } from "@mui/material";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

export const ThreadsPanel = () => {
  const threads = useDebuggerDataStore((state) => state.threads);

  return (
    <Panel>
      {threads.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          No thread information available.
        </Typography>
      ) : (
        <List dense disablePadding>
          {threads.map((thread) => (
            <ListItemButton key={thread.id} sx={{ alignItems: "flex-start" }}>
              <ListItemText
                primary={
                  <Typography variant="body2" fontWeight={600}>
                    {thread.name ?? thread.id} ({thread.state})
                  </Typography>
                }
                secondary={
                  <Typography component="div" variant="caption" sx={{ whiteSpace: "pre-wrap" }}>
                    Core: {thread.core ?? "?"}
                    {thread.backtrace && thread.backtrace.length > 0
                      ? `\n${thread.backtrace.map((frame) => formatFrame(frame)).join("\n")}`
                      : ""}
                  </Typography>
                }
              />
            </ListItemButton>
          ))}
        </List>
      )}
    </Panel>
  );
};

const formatFrame = (frame: { index: number; pc: number; symbol?: string; location?: string }) => {
  const pc = `0x${frame.pc.toString(16).toUpperCase().padStart(8, "0")}`;
  const symbol = frame.symbol ? ` ${frame.symbol}` : "";
  const location = frame.location ? ` (${frame.location})` : "";
  return `#${frame.index} ${pc}${symbol}${location}`;
};
