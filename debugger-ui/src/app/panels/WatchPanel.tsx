import { useCallback, useState } from "react";
import { Panel } from "../layout/Panel";
import { IconButton, List, ListItem, ListItemText, Tooltip, Typography, CircularProgress, Stack } from "@mui/material";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import AddIcon from "@mui/icons-material/Add";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

export const WatchPanel = () => {
  const watchExpressions = useDebuggerDataStore((state) => state.watchExpressions);
  const watchValues = useDebuggerDataStore((state) => state.watchValues);
  const addWatch = useDebuggerDataStore((state) => state.addWatch);
  const removeWatch = useDebuggerDataStore((state) => state.removeWatch);
  const initialized = useDebuggerDataStore((state) => state.initialized);
  const [busyExpression, setBusyExpression] = useState<string | null>(null);

  const handleAdd = useCallback(async () => {
    const expression = window.prompt("Add watch expression", "dc.sh4.cpu.pc");
    if (!expression) {
      return;
    }
    setBusyExpression(expression);
    await addWatch(expression);
    setBusyExpression(null);
  }, [addWatch]);

  const handleRemove = useCallback(
    async (expression: string) => {
      setBusyExpression(expression);
      await removeWatch(expression);
      setBusyExpression(null);
    },
    [removeWatch],
  );

  const isBusy = (expression: string) => busyExpression === expression;

  return (
    <Panel
      title="Watch"
      action={
        <Tooltip title="Add watch">
          <IconButton size="small" color="primary" onClick={handleAdd}>
            <AddIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      }
    >
      {!initialized ? (
        <Stack direction="row" alignItems="center" justifyContent="center" sx={{ p: 2 }} spacing={1}>
          <CircularProgress size={16} />
          <Typography variant="body2" color="text.secondary">
            Connecting to debugger…
          </Typography>
        </Stack>
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
                  disabled={isBusy(expression)}
                >
                  {isBusy(expression) ? <CircularProgress size={14} /> : <DeleteOutlineIcon fontSize="small" />}
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
