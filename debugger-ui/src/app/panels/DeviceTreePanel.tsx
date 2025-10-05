import { useCallback, useMemo } from "react";
import type { ReactNode } from "react";
import { SimpleTreeView } from "@mui/x-tree-view/SimpleTreeView";
import { TreeItem } from "@mui/x-tree-view/TreeItem";
import { Panel } from "../layout/Panel";
import type { DeviceNodeDescriptor, RegisterValue } from "../../lib/debuggerSchema";
import {
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


export const DeviceTreePanel = () => {
  const deviceTree = useDebuggerDataStore((state) => state.deviceTree);
  const registersByPath = useDebuggerDataStore((state) => state.registersByPath);
  const initialized = useDebuggerDataStore((state) => state.initialized);
  const addWatch = useDebuggerDataStore((state) => state.addWatch);
  const watchExpressions = useDebuggerDataStore((state) => state.watchExpressions);

  const handleRegisterWatch = useCallback(
    async (expression: string) => {
      if (watchExpressions.includes(expression)) {
        return;
      }
      await addWatch(expression);
    },
    [addWatch, watchExpressions],
  );

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
      // Get live value from registersByPath if available
      const pathRegisters = registersByPath[nodePath];
      const liveRegister = pathRegisters?.find((r) => r.name === register.name);
      const displayValue = liveRegister?.value ?? register.value;

      const expression = `${nodePath}.${register.name.toLowerCase()}`;
      const watched = watchExpressions.includes(expression);
      return (
        <TreeItem
          key={`${nodePath}.reg.${register.name}`}
          itemId={`${nodePath}.reg.${register.name}`}
          label={
            <Stack direction="row" spacing={1} alignItems="center" justifyContent="space-between">
              <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                {register.name} = {displayValue}
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
    [handleRegisterWatch, watchExpressions, registersByPath],
  );

  const renderNode = useCallback(
    (node: DeviceNodeDescriptor): ReactNode => (
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
    <Panel title="Device Tree">
      {!initialized ? (
        <Stack alignItems="center" justifyContent="center" sx={{ p: 2 }} spacing={1}>
          <CircularProgress size={16} />
          <Typography variant="body2" color="text.secondary">
            Loading devices…
          </Typography>
        </Stack>
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
