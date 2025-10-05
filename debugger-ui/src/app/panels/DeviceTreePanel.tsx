import { useMemo } from "react";
import { TreeItem, TreeView } from "@mui/lab";
import { Panel } from "../layout/Panel";
import type { DeviceNodeDescriptor } from "../../lib/debuggerSchema";
import { Stack, Typography } from "@mui/material";
import ExpandMoreIcon from "@mui/icons-material/ExpandMore";
import ChevronRightIcon from "@mui/icons-material/ChevronRight";

const placeholderTree: DeviceNodeDescriptor[] = [
  {
    path: "dc",
    label: "Dreamcast",
    kind: "bus",
    children: [
      {
        path: "dc.sh4",
        label: "SH4 CPU",
        kind: "processor",
        children: [
          { path: "dc.sh4.cpu", label: "Core", kind: "processor" },
          { path: "dc.sh4.icache", label: "I-Cache", kind: "peripheral" },
          { path: "dc.sh4.dcache", label: "D-Cache", kind: "peripheral" },
          { path: "dc.sh4.tlb", label: "TLB", kind: "peripheral" },
        ],
      },
      {
        path: "dc.holly",
        label: "Holly",
        kind: "peripheral",
        children: [
          { path: "dc.holly.dmac", label: "DMA Controller", kind: "peripheral" },
          { path: "dc.holly.pvr", label: "PowerVR", kind: "pipeline" },
        ],
      },
      {
        path: "dc.aica",
        label: "AICA",
        kind: "coprocessor",
        children: [
          { path: "dc.aica.channels", label: "Channels", kind: "channel" },
          { path: "dc.aica.dsp", label: "DSP", kind: "coprocessor" },
        ],
      },
    ],
  },
];

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

export const DeviceTreePanel = () => {
  const data = useMemo(() => placeholderTree, []);

  return (
    <Panel title="Device Tree">
      <TreeView
        defaultCollapseIcon={<ExpandMoreIcon fontSize="small" />}
        defaultExpandIcon={<ChevronRightIcon fontSize="small" />}
        sx={{ px: 1, py: 1 }}
        multiSelect
        defaultExpanded={["dc", "dc.sh4", "dc.holly", "dc.aica"]}
      >
        {data.map(renderNode)}
      </TreeView>
    </Panel>
  );
};
