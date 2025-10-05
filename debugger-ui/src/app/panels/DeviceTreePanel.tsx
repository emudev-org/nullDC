import { useMemo } from "react";
import { TreeItem, TreeView } from "@mui/lab";
import { Panel } from "../layout/Panel";
import type { DeviceNodeDescriptor } from "../../lib/debuggerSchema";
import { Stack, Typography } from "@mui/material";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";
import ChevronRightIcon from "@mui/icons-material/ChevronRight";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

const renderNode = (node: DeviceNodeDescriptor) => (
  <TreeItem
    key={node.path}
    nodeId={node.path}
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
    {node.children?.map(renderNode)}
  </TreeItem>
);

const gatherExpanded = (nodes: DeviceNodeDescriptor[]): string[] =>
  nodes.flatMap((node) => [node.path, ...(node.children ? gatherExpanded(node.children) : [])]);

export const DeviceTreePanel = () => {
  const deviceTree = useDebuggerDataStore((state) => state.deviceTree);
  const expanded = useMemo(() => gatherExpanded(deviceTree), [deviceTree]);

  return (
    <Panel title="Device Tree">
      {deviceTree.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          Device information unavailable. Ensure the debugger connection is active.
        </Typography>
      ) : (
        <TreeView
          defaultCollapseIcon={<ExpandMoreIcon fontSize="small" />}
          defaultExpandIcon={<ChevronRightIcon fontSize="small" />}
          sx={{ px: 1, py: 1 }}
          multiSelect
          defaultExpanded={expanded}
        >
          {deviceTree.map(renderNode)}
        </TreeView>
      )}
    </Panel>
  );
};
