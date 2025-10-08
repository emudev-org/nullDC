import { useCallback, useMemo, useState } from "react";
import { Panel } from "../layout/Panel";
import { Autocomplete, Box, Button, IconButton, List, ListItem, Stack, TextField, Typography } from "@mui/material";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import type { DeviceNodeDescriptor } from "../../lib/debuggerSchema";

// Component for individual watch item with inline editing
const WatchItem = ({ expression, value, onRemove, onEdit, onModifyExpression }: {
  expression: string;
  value: unknown;
  onRemove: () => void;
  onEdit: (newValue: string) => Promise<void>;
  onModifyExpression: (newExpression: string) => Promise<void>;
}) => {
  const [isEditingValue, setIsEditingValue] = useState(false);
  const [isEditingExpression, setIsEditingExpression] = useState(false);
  const [editValue, setEditValue] = useState("");
  const [editExpression, setEditExpression] = useState("");
  const [hasValueError, setHasValueError] = useState(false);
  const [hasExpressionError, setHasExpressionError] = useState(false);

  const handleDoubleClickValue = () => {
    setEditValue(formatWatchValue(value));
    setIsEditingValue(true);
    setHasValueError(false);
  };

  const handleDoubleClickExpression = () => {
    setEditExpression(expression);
    setIsEditingExpression(true);
    setHasExpressionError(false);
  };

  const handleSaveValue = async () => {
    try {
      await onEdit(editValue);
      setIsEditingValue(false);
      setHasValueError(false);
    } catch {
      // Flash red border on error - keep edit box open
      setHasValueError(true);
      setTimeout(() => setHasValueError(false), 500);
    }
  };

  const handleSaveExpression = async () => {
    try {
      await onModifyExpression(editExpression);
      setIsEditingExpression(false);
      setHasExpressionError(false);
    } catch {
      // Flash red border on error - keep edit box open
      setHasExpressionError(true);
      setTimeout(() => setHasExpressionError(false), 500);
    }
  };

  const handleKeyDownValue = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleSaveValue().catch((error) => {
        console.error("handleSaveValue failed:", error);
      });
    } else if (e.key === "Escape") {
      setIsEditingValue(false);
      setHasValueError(false);
    }
  };

  const handleKeyDownExpression = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleSaveExpression().catch((error) => {
        console.error("handleSaveExpression failed:", error);
      });
    } else if (e.key === "Escape") {
      setIsEditingExpression(false);
      setHasExpressionError(false);
    }
  };

  return (
    <ListItem
      sx={{ py: 0.25, px: 1.5 }}
      secondaryAction={
        <IconButton
          edge="end"
          size="small"
          aria-label={`Remove ${expression}`}
          onClick={onRemove}
        >
          <DeleteOutlineIcon fontSize="small" />
        </IconButton>
      }
    >
      {isEditingExpression || isEditingValue ? (
        <Stack direction="row" spacing={1} sx={{ flex: 1, pr: 1 }}>
          {isEditingExpression ? (
            <TextField
              size="small"
              value={editExpression}
              onChange={(e) => setEditExpression(e.target.value)}
              onKeyDown={handleKeyDownExpression}
              onBlur={() => {
                setIsEditingExpression(false);
                setHasExpressionError(false);
              }}
              autoFocus
              error={hasExpressionError}
              sx={{
                flex: 1,
                "& .MuiOutlinedInput-root": {
                  fontSize: "0.813rem",
                  fontFamily: "monospace",
                  height: "24px",
                },
              }}
            />
          ) : (
            <>
              <Typography
                variant="body2"
                onDoubleClick={handleDoubleClickExpression}
                sx={{
                  fontFamily: "monospace",
                  fontSize: "0.813rem",
                  whiteSpace: "nowrap",
                  cursor: "text",
                }}
              >
                {expression}
              </Typography>
              <Typography
                variant="body2"
                sx={{
                  fontFamily: "monospace",
                  fontSize: "0.813rem",
                  whiteSpace: "nowrap",
                }}
              >
                =
              </Typography>
              {isEditingValue ? (
                <TextField
                  size="small"
                  value={editValue}
                  onChange={(e) => setEditValue(e.target.value)}
                  onKeyDown={handleKeyDownValue}
                  onBlur={() => {
                    setIsEditingValue(false);
                    setHasValueError(false);
                  }}
                  autoFocus
                  error={hasValueError}
                  sx={{
                    flex: 1,
                    "& .MuiOutlinedInput-root": {
                      fontSize: "0.813rem",
                      fontFamily: "monospace",
                      height: "24px",
                    },
                  }}
                />
              ) : (
                <Typography
                  variant="body2"
                  onDoubleClick={handleDoubleClickValue}
                  sx={{
                    fontFamily: "monospace",
                    fontSize: "0.813rem",
                    cursor: "text",
                  }}
                >
                  {formatWatchValue(value)}
                </Typography>
              )}
            </>
          )}
        </Stack>
      ) : (
        <Typography
          variant="body2"
          sx={{
            fontFamily: "monospace",
            fontSize: "0.813rem",
            wordBreak: "break-word",
            pr: 1,
          }}
        >
          <Box
            component="span"
            onDoubleClick={handleDoubleClickExpression}
            sx={{ cursor: "text" }}
          >
            {expression}
          </Box>
          {" = "}
          <Box
            component="span"
            onDoubleClick={handleDoubleClickValue}
            sx={{ cursor: "text" }}
          >
            {formatWatchValue(value)}
          </Box>
        </Typography>
      )}
    </ListItem>
  );
};

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

export const WatchesPanel = () => {
  const initialized = useDebuggerDataStore((state) => state.initialized);
  const watches = useDebuggerDataStore((state) => state.watches);
  const deviceTree = useDebuggerDataStore((state) => state.deviceTree);
  const addWatch = useDebuggerDataStore((state) => state.addWatch);
  const removeWatch = useDebuggerDataStore((state) => state.removeWatch);
  const editWatch = useDebuggerDataStore((state) => state.editWatch);
  const modifyWatchExpression = useDebuggerDataStore((state) => state.modifyWatchExpression);
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
    async (watchId: string) => {
      await removeWatch(watchId);
    },
    [removeWatch],
  );

  const handleEdit = useCallback(
    async (watchId: string, value: string): Promise<void> => {
      await editWatch(watchId, value);
    },
    [editWatch],
  );

  const handleModifyExpression = useCallback(
    async (watchId: string, newExpression: string): Promise<void> => {
      await modifyWatchExpression(watchId, newExpression);
    },
    [modifyWatchExpression],
  );

  return (
    <Panel>
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
                placeholder="Enter expression"
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
          <Button size="small" variant="contained" onClick={handleAdd} sx={{ minWidth: 85 }}>
            Add Watch
          </Button>
        </Stack>
      </Box>
      {!initialized ? (
        <Box sx={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%" }}>
          <Typography variant="body2" color="text.secondary">
            No Data
          </Typography>
        </Box>
      ) : watches.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          No watches subscribed.
        </Typography>
      ) : (
        <List dense disablePadding>
          {watches.map((watch) => (
            <WatchItem
              key={watch.id}
              expression={watch.expression}
              value={watch.value}
              onRemove={() => void handleRemove(watch.id)}
              onEdit={(newValue) => handleEdit(watch.id, newValue)}
              onModifyExpression={(newExpression) => handleModifyExpression(watch.id, newExpression)}
            />
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
