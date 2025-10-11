import { Box, Typography } from "@mui/material";
import { Panel } from "../layout/Panel";

// Empty content panel component
const EmptyContentPanel = ({ title }: { title: string }) => (
  <Panel>
    <Box sx={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", p: 2 }}>
      <Typography variant="body2" color="text.secondary">
        {title} - Coming Soon
      </Typography>
    </Box>
  </Panel>
);

// Memory panel with all zeros (placeholder)
const ZeroMemoryPanel = ({ title }: { title: string }) => (
  <Panel>
    <Box sx={{ display: "flex", flexDirection: "column", height: "100%", p: 2 }}>
      <Typography variant="h6" sx={{ mb: 2 }}>{title}</Typography>
      <Box sx={{ flex: 1, fontFamily: "monospace", fontSize: "0.875rem", overflow: "auto" }}>
        <Typography variant="body2" color="text.secondary">
          Memory contents (all zeros)
        </Typography>
      </Box>
    </Box>
  </Panel>
);

export const SqContentsPanel = () => <ZeroMemoryPanel title="Store Queues Contents" />;
export const IcacheContentsPanel = () => <EmptyContentPanel title="Instruction Cache Contents" />;
export const OcacheContentsPanel = () => <EmptyContentPanel title="Operand Cache Contents" />;
export const OcramContentsPanel = () => <ZeroMemoryPanel title="Operand RAM Contents" />;
export const TlbContentsPanel = () => <EmptyContentPanel title="TLB Contents" />;
