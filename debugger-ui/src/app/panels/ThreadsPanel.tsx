import { Panel } from "../layout/Panel";
import { List, ListItemButton, ListItemText, Typography } from "@mui/material";

const threads = [
  {
    id: "main",
    name: "Main Thread",
    state: "running",
    backtrace: ["_start", "kernel_main", "game_loop"],
  },
  {
    id: "audio",
    name: "AICA Worker",
    state: "blocked",
    backtrace: ["aica_wait", "aica_mix"],
  },
];

export const ThreadsPanel = () => {
  return (
    <Panel title="Threads">
      <List dense disablePadding>
        {threads.map((thread) => (
          <ListItemButton key={thread.id} sx={{ alignItems: "flex-start" }}>
            <ListItemText
              primary={
                <Typography variant="body2" fontWeight={600}>
                  {thread.name}
                </Typography>
              }
              secondary={
                <Typography component="div" variant="caption" sx={{ whiteSpace: "pre-wrap" }}>
                  {`State: ${thread.state}`}
                  {"\n"}
                  {thread.backtrace.join("\n")}
                </Typography>
              }
            />
          </ListItemButton>
        ))}
      </List>
    </Panel>
  );
};
