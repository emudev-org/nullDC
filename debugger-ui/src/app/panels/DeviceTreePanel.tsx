import { useCallback, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { SimpleTreeView } from "@mui/x-tree-view/SimpleTreeView";
import { TreeItem } from "@mui/x-tree-view/TreeItem";
import { Panel } from "../layout/Panel";
import type { DeviceNodeDescriptor, RegisterValue } from "../../lib/debuggerSchema";
import {
  Box,
  CircularProgress,
  Divider,
  IconButton,
  Stack,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";
import ChevronRightIcon from "@mui/icons-material/ChevronRight";
import AddIcon from "@mui/icons-material/Add";
import SearchIcon from "@mui/icons-material/Search";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";


export const DeviceTreePanel = () => {
  const deviceTree = useDebuggerDataStore((state) => state.deviceTree);
  const registersByPath = useDebuggerDataStore((state) => state.registersByPath);
  const initialized = useDebuggerDataStore((state) => state.initialized);
  const addWatch = useDebuggerDataStore((state) => state.addWatch);
  const watchExpressions = useDebuggerDataStore((state) => state.watchExpressions);
  const [searchQuery, setSearchQuery] = useState("");

  const handleRegisterWatch = useCallback(
    async (expression: string) => {
      if (watchExpressions.includes(expression)) {
        return;
      }
      await addWatch(expression);
    },
    [addWatch, watchExpressions],
  );

  // Filter tree based on search query (hierarchical filtering - each word filters progressively)
  const filterNode = useCallback(
    (node: DeviceNodeDescriptor, query: string, depth: number = 0): DeviceNodeDescriptor | null => {
      if (!query) return node;

      // Split query into words and filter out empty strings
      const queryWords = query.toLowerCase().split(/\s+/).filter(word => word.length > 0);
      if (queryWords.length === 0) return node;

      const nodePath = node.path.toLowerCase();
      const nodeLabel = node.label.toLowerCase();

      // Check if this node matches the current level's query word
      const currentWord = queryWords[depth] || queryWords[queryWords.length - 1];
      const nodeMatches = nodePath.includes(currentWord) || nodeLabel.includes(currentWord);

      // Filter registers - check if any register matches remaining query words
      const remainingWords = queryWords.slice(depth + (nodeMatches ? 1 : 0));
      const filteredRegisters = node.registers?.filter((reg) => {
        const regName = reg.name.toLowerCase();
        // Register must match all remaining words
        return remainingWords.every((word) => regName.includes(word));
      });

      // Filter children recursively
      // If this node matches, children use next query word (depth + 1)
      // If this node doesn't match, children use same query word (depth)
      const nextDepth = nodeMatches ? depth + 1 : depth;
      const filteredChildren = node.children
        ?.map((child) => filterNode(child, query, nextDepth))
        .filter((child): child is DeviceNodeDescriptor => child !== null);

      const hasMatchingRegisters = filteredRegisters && filteredRegisters.length > 0;
      const hasMatchingChildren = filteredChildren && filteredChildren.length > 0;

      // Include this node if:
      // 1. It has matching children, OR
      // 2. It has matching registers and all query words up to this point are satisfied
      if (hasMatchingChildren || hasMatchingRegisters) {
        return {
          ...node,
          registers: hasMatchingRegisters ? filteredRegisters : undefined,
          children: filteredChildren,
        };
      }

      return null;
    },
    [],
  );

  const filteredTree = useMemo(() => {
    if (!searchQuery) return deviceTree;
    return deviceTree
      .map((node) => filterNode(node, searchQuery))
      .filter((node): node is DeviceNodeDescriptor => node !== null);
  }, [deviceTree, searchQuery, filterNode]);

  // Collect all node paths for expansion when searching
  const collectAllPaths = useCallback((nodes: DeviceNodeDescriptor[]): string[] => {
    const paths: string[] = [];
    for (const node of nodes) {
      paths.push(node.path);
      if (node.children) {
        paths.push(...collectAllPaths(node.children));
      }
    }
    return paths;
  }, []);

  const expandedItems = useMemo(() => {
    if (!filteredTree.length) {
      return [];
    }
    // If searching, expand all matching nodes
    if (searchQuery) {
      return collectAllPaths(filteredTree);
    }
    // Otherwise, expand first two levels by default
    const secondLevel = filteredTree.flatMap((node) => node.children?.map((child) => child.path) ?? []);
    return [...filteredTree.map((node) => node.path), ...secondLevel];
  }, [filteredTree, searchQuery, collectAllPaths]);

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
        <Box sx={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
          <Box sx={{ px: 1.5, py: 1 }}>
            <TextField
              size="small"
              fullWidth
              placeholder="Search devices..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              InputProps={{
                startAdornment: <SearchIcon fontSize="small" sx={{ mr: 1, color: "text.secondary" }} />,
              }}
              sx={{ "& .MuiOutlinedInput-root": { fontSize: "0.875rem" } }}
            />
          </Box>
          <Divider />
          <SimpleTreeView
            key={treeKey}
            defaultExpandedItems={expandedItems}
            slots={{ collapseIcon: ExpandMoreIcon, expandIcon: ChevronRightIcon }}
            multiSelect
            sx={{ px: 1, py: 1, flex: 1, overflowY: "auto" }}
          >
            {filteredTree.map((node) => renderNode(node))}
          </SimpleTreeView>
        </Box>
      )}
    </Panel>
  );
};
