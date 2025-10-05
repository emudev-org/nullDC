import { Panel } from "../layout/Panel";
import { Box, Typography } from "@mui/material";

const placeholder = [
  { address: 0x8C0000A0, bytes: "02 45", mnemonic: "mov.l", operands: "@r15+, r1", current: true },
  { address: 0x8C0000A2, bytes: "6E F6", mnemonic: "mov", operands: "r15, r14" },
  { address: 0x8C0000A4, bytes: "4F 22", mnemonic: "sts.l", operands: "pr, @-r15" },
];

export const DisassemblyPanel = () => {
  return (
    <Panel title="Disassembly">
      <Box component="pre" sx={{ fontFamily: "monospace", fontSize: 13, m: 0, p: 1.5 }}>
        {placeholder.map((line) => (
          <Typography
            key={line.address}
            component="div"
            sx={{
              display: "flex",
              gap: 2,
              color: line.current ? "primary.main" : "inherit",
            }}
          >
            <span>{`0x${line.address.toString(16).padStart(8, "0")}`}</span>
            <span>{line.bytes.padEnd(11, " ")}</span>
            <span>{line.mnemonic.padEnd(8, " ")}</span>
            <span>{line.operands}</span>
          </Typography>
        ))}
      </Box>
    </Panel>
  );
};
