import { useCallback, useMemo, useState } from "react";
import { Panel } from "../layout/Panel";
import { Autocomplete, Box, Button, IconButton, List, ListItem, ListItemText, Stack, TextField, Typography } from "@mui/material";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import type { DeviceNodeDescriptor } from "../../lib/debuggerSchema";

interface WatchesPanelProps {
  showTitle?: boolean;
}

// Helper to collect all register paths from device tree
const collectRegisterPaths = (nodes: DeviceNodeDescriptor[]): string[] => {
  const paths: string[] = [];
  for (const node of nodes) {
    // Add register paths - node.path already contains the full path
    if (node.registers) {
      for (const reg of node.registers) {
        paths.push(`${node.path}.${reg.name}`);
      }
    }

    // Recursively collect from children
    if (node.children) {
      paths.push(...collectRegisterPaths(node.children));
    }
  }
  return paths;
};

export const WatchesPanel = ({ showTitle = false }: WatchesPanelProps) => {
  const initialized = useDebuggerDataStore((state) => state.initialized);
  const watchExpressions = useDebuggerDataStore((state) => state.watchExpressions);
  const watchValues = useDebuggerDataStore((state) => state.watchValues);
  const deviceTree = useDebuggerDataStore((state) => state.deviceTree);
  const addWatch = useDebuggerDataStore((state) => state.addWatch);
  const removeWatch = useDebuggerDataStore((state) => state.removeWatch);
  const [newWatch, setNewWatch] = useState("");

  // Collect all available register paths from device tree
  const availablePaths = useMemo(() => collectRegisterPaths(deviceTree), [deviceTree]);

  const handleAdd = useCallback(async () => {
    const trimmed = newWatch.trim();
    if (!trimmed) {
      return;
    }
    await addWatch(trimmed);
    setNewWatch("");
  }, [newWatch, addWatch]);

  const handleRemove = useCallback(
    async (expression: string) => {
      await removeWatch(expression);
    },
    [removeWatch],
  );

  return (
    <Panel title={showTitle ? "Watches" : undefined}>
      <Box sx={{ p: 1.5, borderBottom: "1px solid", borderColor: "divider" }}>
        <Stack direction="row" spacing={1}>
          <Autocomplete
            size="small"
            fullWidth
            freeSolo
            options={availablePaths}
            value={newWatch}
            onChange={(_, newValue) => setNewWatch(newValue ?? "")}
            onInputChange={(_, newValue) => setNewWatch(newValue)}
            renderInput={(params) => (
              <TextField
                {...params}
                placeholder="Enter Expression"
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    void handleAdd();
                  }
                }}
                sx={{ "& .MuiOutlinedInput-root": { fontSize: "0.875rem", fontFamily: "monospace" } }}
              />
            )}
          />
          <Button size="small" variant="contained" onClick={handleAdd} sx={{ minWidth: 60 }}>
            Add
          </Button>
        </Stack>
      </Box>
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
