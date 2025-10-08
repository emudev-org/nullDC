import { memo, useMemo } from "react";
import { Link } from "react-router-dom";
import { Panel } from "../layout/Panel";
import { Box, List, ListItem, ListItemText, Stack, Typography } from "@mui/material";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import type { CallstackFrame } from "../../lib/debuggerSchema";

interface CallstackPanelProps {
  target: "sh4" | "arm7";
  showTitle?: boolean;
}

interface CallstackFrameItemProps {
  frame: CallstackFrame;
  target: string;
}

const CallstackFrameItem = memo(({ frame, target }: CallstackFrameItemProps) => {
  const addressHex = `0x${frame.pc.toString(16).toUpperCase().padStart(8, "0")}`;
  // Add action_guid to force re-navigation even when clicking same address
  const actionGuid = crypto.randomUUID();
  const disassemblyPath = `/${target}-disassembly?address=${addressHex}&action_guid=${actionGuid}`;

  return (
    <ListItem disablePadding sx={{ py: 0 }}>
      <ListItemText
        sx={{ my: 0 }}
        primary={
          <Stack direction="row" spacing={1} alignItems="baseline" sx={{ flexWrap: "wrap" }}>
            <Typography variant="caption" color="text.secondary" sx={{ minWidth: 28 }}>
              #{frame.index}
            </Typography>
            <Link
              to={disassemblyPath}
              style={{
                textDecoration: "none",
                color: "inherit",
                fontFamily: "monospace",
              }}
            >
              <Typography
                variant="body2"
                sx={{
                  fontFamily: "monospace",
                  "&:hover": {
                    textDecoration: "underline",
                    color: "primary.main",
                  },
                }}
              >
                {addressHex}
              </Typography>
            </Link>
            {frame.symbol && (
              <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                {frame.symbol}
              </Typography>
            )}
            {frame.location && (
              <Typography variant="body2" color="text.secondary" sx={{ fontFamily: "monospace" }}>
                {frame.location}
              </Typography>
            )}
          </Stack>
        }
      />
    </ListItem>
  );
});

CallstackFrameItem.displayName = "CallstackFrameItem";

const EMPTY_FRAMES: never[] = [];

const CallstackPanel = ({ target, showTitle = false }: CallstackPanelProps) => {
  const initialized = useDebuggerDataStore((s) => s.initialized);
  const callstacks = useDebuggerDataStore((s) => s.callstacks);
  const frames = useMemo(() => callstacks[target] ?? EMPTY_FRAMES, [callstacks, target]);

  const title = target === "sh4" ? "SH4: Callstack" : "ARM7: Callstack";

  return (
    <Panel title={showTitle ? title : undefined}>
      {!initialized ? (
        <Stack alignItems="center" justifyContent="center" sx={{ height: "100%" }}>
          <Typography variant="body2" color="text.secondary">
            No Data
          </Typography>
        </Stack>
      ) : frames.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          Empty callstack.
        </Typography>
      ) : (
        <Box sx={{ p: 1 }}>
          <List dense disablePadding>
            {frames.map((f) => (
              <CallstackFrameItem key={f.index} frame={f} target={target} />
            ))}
          </List>
        </Box>
      )}
    </Panel>
  );
};

export default CallstackPanel;


