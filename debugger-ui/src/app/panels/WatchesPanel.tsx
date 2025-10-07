import { useCallback } from "react";
import { Panel } from "../layout/Panel";
import { Box, IconButton, List, ListItem, ListItemText, Tooltip, Typography } from "@mui/material";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import AddIcon from "@mui/icons-material/Add";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

interface WatchesPanelProps {
  showTitle?: boolean;
}

export const WatchesPanel = ({ showTitle = false }: WatchesPanelProps) => {
  const initialized = useDebuggerDataStore((state) => state.initialized);
  const watchExpressions = useDebuggerDataStore((state) => state.watchExpressions);
  const watchValues = useDebuggerDataStore((state) => state.watchValues);
  const addWatch = useDebuggerDataStore((state) => state.addWatch);
  const removeWatch = useDebuggerDataStore((state) => state.removeWatch);

  const handleAdd = useCallback(async () => {
    const expression = window.prompt("Add watch expression", "dc.sh4.cpu.pc");
    if (!expression) {
      return;
    }
    await addWatch(expression);
  }, [addWatch]);

  const handleRemove = useCallback(
    async (expression: string) => {
      await removeWatch(expression);
    },
    [removeWatch],
  );

  return (
    <Panel
      title={showTitle ? "Watches" : undefined}
      action={
        <Tooltip title="Add watch">
          <IconButton size="small" color="primary" onClick={handleAdd}>
            <AddIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      }
    >
      {!initialized ? (
        <Box sx={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%" }}>
          <Typography variant="body2" color="text.secondary">
            No Data
          </Typography>
        </Box>
      ) : watchExpressions.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          No watches subscribed.
        </Typography>
      ) : (
        <List dense disablePadding>
          {watchExpressions.map((expression) => (
            <ListItem
              key={expression}
              secondaryAction={
                <IconButton
                  edge="end"
                  size="small"
                  aria-label={`Remove ${expression}`}
                  onClick={() => void handleRemove(expression)}
                >
                  <DeleteOutlineIcon fontSize="small" />
                </IconButton>
              }
            >
              <ListItemText
                primary={
                  <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                    {expression}
                  </Typography>
                }
                secondary={
                  <Typography variant="caption" color="text.secondary">
                    {formatWatchValue(watchValues[expression])}
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

const formatWatchValue = (value: unknown): string => {
  if (value === null || value === undefined) {
    return "—";
  }
  if (typeof value === "number") {
    return `0x${value.toString(16).toUpperCase()}`;
  }
  if (typeof value === "object") {
    try {
      return JSON.stringify(value);
    } catch {
      return String(value);
    }
  }
  return String(value);
};
