import { Box, Typography } from "@mui/material";
import { Panel } from "../layout/Panel";

// Empty register panel component
const EmptyRegisterPanel = ({ title }: { title: string }) => (
  <Panel>
    <Box sx={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", p: 2 }}>
      <Typography variant="body2" color="text.secondary">
        {title} - Coming Soon
      </Typography>
    </Box>
  </Panel>
);

export const BscRegistersPanel = () => <EmptyRegisterPanel title="Bus State Controller Registers" />;
export const CcnRegistersPanel = () => <EmptyRegisterPanel title="Cache Controller Registers" />;
export const CpgRegistersPanel = () => <EmptyRegisterPanel title="Clock Pulse Generator Registers" />;
export const DmacRegistersPanel = () => <EmptyRegisterPanel title="Direct Memory Access Controller Registers" />;
export const IntcRegistersPanel = () => <EmptyRegisterPanel title="Interrupt Controller Registers" />;
export const RtcRegistersPanel = () => <EmptyRegisterPanel title="Real Time Clock Registers" />;
export const SciRegistersPanel = () => <EmptyRegisterPanel title="Serial Communications Interface Registers" />;
export const ScifRegistersPanel = () => <EmptyRegisterPanel title="Serial Communications Interface w/ FIFO Registers" />;
export const TmuRegistersPanel = () => <EmptyRegisterPanel title="Timer Unit Registers" />;
export const UbcRegistersPanel = () => <EmptyRegisterPanel title="User Break Controller Registers" />;
