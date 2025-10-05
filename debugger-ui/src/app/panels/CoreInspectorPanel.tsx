import { useMemo } from "react";
import { Panel } from "../layout/Panel";
import { Box, Typography } from "@mui/material";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

export const CoreInspectorPanel = () => {
  const frameLog = useDebuggerDataStore((state) => state.frameLog);
  const entries = useMemo(
    () => (Array.isArray(frameLog) ? frameLog.filter((entry) => entry.subsystem === "core") : []),
    [frameLog],
  );

  const totals = useMemo(
    () => ({
      all: entries.length,
      warnings: entries.filter((entry) => entry.severity === "warn").length,
      errors: entries.filter((entry) => entry.severity === "error").length,
    }),
    [entries],
  );

  const latestMessage = entries.length > 0 ? entries[entries.length - 1]?.message ?? "—" : "—";

  return (
    <Panel title="CORE Inspector">
      {entries.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          No CORE diagnostics available yet.
        </Typography>
      ) : (
        <Box sx={{ p: 2, display: "grid", gridTemplateColumns: "repeat(2, minmax(0, 1fr))", gap: 1 }}>
          <Metric label="Events" value={totals.all} />
          <Metric label="Warnings" value={totals.warnings} />
          <Metric label="Errors" value={totals.errors} />
          <Metric label="Latest" value={latestMessage} />
        </Box>
      )}
    </Panel>
  );
};

const Metric = ({ label, value }: { label: string; value: string | number }) => (
  <Box
    sx={{
      p: 1,
      borderRadius: 1,
      border: "1px solid",
      borderColor: "divider",
    }}
  >
    <Typography variant="caption" color="text.secondary">
      {label}
    </Typography>
    <Typography variant="body2" fontWeight={600}>
      {value}
    </Typography>
  </Box>
);
