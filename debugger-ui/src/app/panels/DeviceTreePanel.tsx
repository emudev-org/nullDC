import { useCallback, useEffect, useMemo, useState } from "react";
import { SimpleTreeView } from "@mui/x-tree-view/SimpleTreeView";
import { TreeItem } from "@mui/x-tree-view/TreeItem";
import { Panel } from "../layout/Panel";
import type { DeviceNodeDescriptor, RegisterValue } from "../../lib/debuggerSchema";
import {
  Button,
  CircularProgress,
  IconButton,
  Stack,
  Tooltip,
  Typography,
} from "@mui/material";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";
import ChevronRightIcon from "@mui/icons-material/ChevronRight";
import AddIcon from "@mui/icons-material/Add";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import { useSessionStore } from "../../state/sessionStore";

const gatherExpanded = (nodes: DeviceNodeDescriptor[]): string[] =>
  nodes.flatMap((node) => [node.path, ...(node.children ? gatherExpanded(node.children) : [])]);

export const DeviceTreePanel = () => {
  const deviceTree = useDebuggerDataStore((state) => state.deviceTree);
  const refreshDeviceTree = useDebuggerDataStore((state) => state.refreshDeviceTree);
  const initialized = useDebuggerDataStore((state) => state.initialized);
  const addWatch = useDebuggerDataStore((state) => state.addWatch);
  const watchExpressions = useDebuggerDataStore((state) => state.watchExpressions);
  const connectionState = useSessionStore((state) => state.connectionState);

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | undefined>();

  const handleRefresh = useCallback(async () => {
    setLoading(true);
    setError(undefined);
    try {
      await refreshDeviceTree();
    } catch (err) {
      console.error("Failed to load device tree", err);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [refreshDeviceTree]);

  const handleRegisterWatch = useCallback(
    async (expression: string) => {
      if (watchExpressions.includes(expression)) {
        return;
      }
      await addWatch(expression);
    },
    [addWatch, watchExpressions],
  );

  useEffect(() => {
    if (connectionState === "connected" && initialized && deviceTree.length === 0 && !loading) {
      void handleRefresh();
    }
    if (connectionState !== "connected") {
      setError(undefined);
    }
  }, [connectionState, initialized, deviceTree.length, loading, handleRefresh]);

  const expandedItems = useMemo(() => {
    if (!deviceTree.length) {
      return [];
    }
    const secondLevel = deviceTree.flatMap((node) => node.children?.map((child) => child.path) ?? []);
    return [...deviceTree.map((node) => node.path), ...secondLevel];
  }, [deviceTree]);
  const treeKey = useMemo(() => expandedItems.join("|"), [expandedItems]);

  const renderRegister = useCallback(
    (nodePath: string, register: RegisterValue) => {
      const expression = `${nodePath}.${register.name.toLowerCase()}`;
      const watched = watchExpressions.includes(expression);
      return (
        <TreeItem
          key={`${nodePath}.reg.${register.name}`}
          itemId={`${nodePath}.reg.${register.name}`}
          label={
            <Stack direction="row" spacing={1} alignItems="center" justifyContent="space-between">
              <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                {register.name} = {register.value}
              </Typography>
              <Tooltip title={watched ? "Already in watch" : "Add to watch"}>
                <span>
                  <IconButton
                    size="small"
                    color="primary"
                    disabled={watched}
                    onClick={(event) => {
                      event.stopPropagation();
                      void handleRegisterWatch(expression);
                    }}
                  >
                    <AddIcon fontSize="inherit" />
                  </IconButton>
                </span>
              </Tooltip>
            </Stack>
          }
        />
      );
    },
    [handleRegisterWatch, watchExpressions],
  );

  const renderNode = useCallback(
    (node: DeviceNodeDescriptor): React.ReactNode => (
      <TreeItem
        key={node.path}
        itemId={node.path}
        label={
          <Stack direction="row" spacing={1} alignItems="center">
            <Typography variant="body2" fontWeight={500}>
              {node.label}
            </Typography>
            <Typography variant="caption" color="text.secondary">
              {node.kind}
            </Typography>
          </Stack>
        }
      >
        {node.registers?.map((register) => renderRegister(node.path, register))}
        {node.children?.map((child) => renderNode(child))}
      </TreeItem>
    ),
    [renderRegister],
  );

  return (
    <Panel
      title="Device Tree"
      action={
        <Button size="small" onClick={() => void handleRefresh()} disabled={loading || connectionState !== "connected"}>
          Refresh
        </Button>
      }
    >
      {loading ? (
        <Stack alignItems="center" justifyContent="center" sx={{ p: 2 }} spacing={1}>
          <CircularProgress size={16} />
          <Typography variant="body2" color="text.secondary">
            Loading devices…
          </Typography>
        </Stack>
      ) : error ? (
        <Typography variant="body2" color="error" sx={{ p: 2 }}>
          {error}
        </Typography>
      ) : deviceTree.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          Device information unavailable. Ensure the debugger connection is active.
        </Typography>
      ) : (
        <SimpleTreeView
          key={treeKey}
          defaultExpandedItems={expandedItems}
          slots={{ collapseIcon: ExpandMoreIcon, expandIcon: ChevronRightIcon }}
          multiSelect
          sx={{ px: 1, py: 1, height: "100%", minHeight: 0, flex: 1 }}
        >
          {deviceTree.map((node) => renderNode(node))}
        </SimpleTreeView>
      )}
    </Panel>
  );
};
