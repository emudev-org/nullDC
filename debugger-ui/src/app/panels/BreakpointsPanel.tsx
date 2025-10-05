import { Panel } from "../layout/Panel";
import { Autocomplete, Box, Button, Chip, IconButton, List, ListItem, ListItemText, Stack, Switch, TextField, Tooltip, Typography } from "@mui/material";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import AddIcon from "@mui/icons-material/Add";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import { useCallback, useState } from "react";
import type { BreakpointDescriptor } from "../../lib/debuggerSchema";

interface BreakpointsViewProps {
  title: string;
  filter?: (location: string) => boolean;
  addMode: "pc-sh4" | "pc-arm7" | "event";
}

const BreakpointsView = ({ title, filter, addMode }: BreakpointsViewProps) => {
  const breakpoints = useDebuggerDataStore((state) => state.breakpoints);
  const availableEvents = useDebuggerDataStore((state) => state.availableEvents);
  const removeBreakpoint = useDebuggerDataStore((state) => state.removeBreakpoint);
  const addBreakpoint = useDebuggerDataStore((state) => state.addBreakpoint);
  const toggleBreakpoint = useDebuggerDataStore((state) => state.toggleBreakpoint);
  const [newBreakpoint, setNewBreakpoint] = useState("");

  const filteredBreakpoints = filter ? breakpoints.filter((bp) => filter(bp.location)) : breakpoints;

  const handleRemove = useCallback(
    async (id: string) => {
      await removeBreakpoint(id);
    },
    [removeBreakpoint],
  );

  const handleToggle = useCallback(
    async (id: string, enabled: boolean) => {
      await toggleBreakpoint(id, enabled);
    },
    [toggleBreakpoint],
  );

  const handleAdd = useCallback(async () => {
    const trimmed = newBreakpoint.trim();
    if (!trimmed) {
      return;
    }

    let location: string;
    let kind: BreakpointDescriptor["kind"];

    if (addMode === "event") {
      location = trimmed;
      kind = "event";
    } else {
      // Parse address for PC breakpoints
      const normalized = trimmed.replace(/^0x/i, "");
      const parsed = Number.parseInt(normalized, 16);
      if (Number.isNaN(parsed)) {
        return;
      }
      const target = addMode === "pc-sh4" ? "dc.sh4.cpu" : "dc.arm7.cpu";
      location = `${target}.pc == 0x${parsed.toString(16).toUpperCase().padStart(8, "0")}`;
      kind = "code";
    }

    await addBreakpoint(location, kind);
    setNewBreakpoint("");
  }, [newBreakpoint, addBreakpoint, addMode]);

  const placeholder =
    addMode === "event"
      ? "Event name (e.g., dc.aica.channel[0].step)"
      : `Address (e.g., 0x8C0000A0)`;

  return (
    <Panel title={title}>
      <Box sx={{ p: 1.5, borderBottom: "1px solid", borderColor: "divider" }}>
        <Stack direction="row" spacing={1}>
          {addMode === "event" ? (
            <Autocomplete
              size="small"
              fullWidth
              freeSolo
              options={availableEvents}
              value={newBreakpoint}
              onChange={(_, newValue) => setNewBreakpoint(newValue ?? "")}
              onInputChange={(_, newValue) => setNewBreakpoint(newValue)}
              renderInput={(params) => (
                <TextField
                  {...params}
                  placeholder={placeholder}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      void handleAdd();
                    }
                  }}
                  sx={{ "& .MuiOutlinedInput-root": { fontSize: "0.875rem", fontFamily: "monospace" } }}
                />
              )}
            />
          ) : (
            <TextField
              size="small"
              fullWidth
              placeholder={placeholder}
              value={newBreakpoint}
              onChange={(e) => setNewBreakpoint(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  void handleAdd();
                }
              }}
              sx={{ "& .MuiOutlinedInput-root": { fontSize: "0.875rem", fontFamily: "monospace" } }}
            />
          )}
          <Button
            size="small"
            variant="contained"
            startIcon={<AddIcon />}
            onClick={() => void handleAdd()}
            disabled={!newBreakpoint.trim()}
          >
            Add
          </Button>
        </Stack>
      </Box>
      {filteredBreakpoints.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          No breakpoints defined.
        </Typography>
      ) : (
        <List dense disablePadding sx={{ overflowY: "auto", flex: 1 }}>
          {filteredBreakpoints.map((bp) => (
            <ListItem
              key={bp.id}
              secondaryAction={
                <Tooltip title="Remove breakpoint">
                  <IconButton
                    edge="end"
                    size="small"
                    onClick={() => {
                      void handleRemove(bp.id);
                    }}
                  >
                    <DeleteOutlineIcon fontSize="small" />
                  </IconButton>
                </Tooltip>
              }
            >
              <ListItemText
                primary={
                  <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                    {bp.location}
                  </Typography>
                }
                secondary={
                  <Typography variant="caption" color="text.secondary" sx={{ display: "flex", gap: 1, alignItems: "center" }}>
                    <Chip label={bp.kind} size="small" color="primary" variant="outlined" />
                    <Switch
                      size="small"
                      checked={bp.enabled}
                      onChange={(e) => {
                        void handleToggle(bp.id, e.target.checked);
                      }}
                    />
                    <span>Hits: {bp.hitCount}</span>
                  </Typography>
                }
              />
            </ListItem>
          ))}
        </List>
      )}
    </Panel>
  );
};

export const EventsBreakpointsPanel = () => <BreakpointsView title="Events: Breakpoints" addMode="event" />;

export const Sh4BreakpointsPanel = () => (
  <BreakpointsView title="SH4: Breakpoints" filter={(loc) => loc.toLowerCase().includes("sh4")} addMode="pc-sh4" />
);

export const Arm7BreakpointsPanel = () => (
  <BreakpointsView title="ARM7: Breakpoints" filter={(loc) => loc.toLowerCase().includes("arm7")} addMode="pc-arm7" />
);

export const DspBreakpointsPanel = () => (
  <BreakpointsView
    title="DSP: Breakpoints"
    filter={(loc) => {
      const lower = loc.toLowerCase();
      return lower.includes("aica") || lower.includes("dsp");
    }}
    addMode="event"
  />
);
