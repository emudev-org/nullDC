import { Panel } from "../layout/Panel";
import {
  Autocomplete,
  Box,
  Button,
  Chip,
  IconButton,
  List,
  ListItem,
  ListItemText,
  Stack,
  Switch,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import AddIcon from "@mui/icons-material/Add";
import VolumeOffIcon from "@mui/icons-material/VolumeOff";
import VolumeUpIcon from "@mui/icons-material/VolumeUp";
import RadioButtonUncheckedIcon from "@mui/icons-material/RadioButtonUnchecked";
import RadioButtonCheckedIcon from "@mui/icons-material/RadioButtonChecked";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import { useSessionStore } from "../../state/sessionStore";
import { categoryStates, setClient, syncCategoryStatesToServer, type BreakpointCategory } from "../../state/breakpointCategoryState";
import { useCallback, useState, useEffect } from "react";
import type { BreakpointDescriptor } from "../../lib/debuggerSchema";

const categorizeBreakpoint = (bp: BreakpointDescriptor): BreakpointCategory => {
  // All event breakpoints go to "events" category
  if (bp.kind === "event") {
    return "events";
  }

  // Code breakpoints are categorized by processor
  const lower = bp.event.toLowerCase();
  if (lower.includes("sh4")) return "sh4";
  if (lower.includes("arm7")) return "arm7";
  if (lower.includes("aica") || lower.includes("dsp")) return "dsp";

  // Default to events if we can't determine
  return "events";
};

const formatBreakpointDisplay = (bp: BreakpointDescriptor): string => {
  if (bp.kind === "event") {
    return bp.event;
  }
  // For code breakpoints, show event == address
  const addressStr = bp.address !== undefined ? `0x${bp.address.toString(16).toUpperCase().padStart(8, "0")}` : "?";
  return `${bp.event} == ${addressStr}`;
};

interface BreakpointsViewProps {
  title: string;
  filter?: (bp: BreakpointDescriptor) => boolean;
  addMode: "pc-sh4" | "pc-arm7" | "event" | "dsp";
  showCategoryControls?: boolean;
}

const CategoryControls = ({
  onMuteToggle,
  onSoloToggle,
  muted,
  soloed,
}: {
  onMuteToggle: () => void;
  onSoloToggle: () => void;
  muted: boolean;
  soloed: boolean;
}) => {
  return (
    <Stack direction="row" spacing={0.5} alignItems="center">
      <Tooltip title={muted ? "Unmute category" : "Mute category"}>
        <IconButton size="small" onClick={onMuteToggle} color={muted ? "warning" : "default"}>
          {muted ? <VolumeOffIcon fontSize="small" /> : <VolumeUpIcon fontSize="small" />}
        </IconButton>
      </Tooltip>
      <Tooltip title={soloed ? "Unsolo category" : "Solo category"}>
        <IconButton size="small" onClick={onSoloToggle} color={soloed ? "primary" : "default"}>
          {soloed ? <RadioButtonCheckedIcon fontSize="small" /> : <RadioButtonUncheckedIcon fontSize="small" />}
        </IconButton>
      </Tooltip>
    </Stack>
  );
};

const BreakpointsView = ({ filter, addMode, showCategoryControls = false }: BreakpointsViewProps) => {
  const breakpoints = useDebuggerDataStore((state) => state.breakpoints);
  const availableEvents = useDebuggerDataStore((state) => state.availableEvents);
  const removeBreakpoint = useDebuggerDataStore((state) => state.removeBreakpoint);
  const addBreakpoint = useDebuggerDataStore((state) => state.addBreakpoint);
  const toggleBreakpoint = useDebuggerDataStore((state) => state.toggleBreakpoint);
  const client = useSessionStore((state) => state.client);
  const [newBreakpoint, setNewBreakpoint] = useState("");

  const filteredBreakpoints = filter ? breakpoints.filter(filter) : breakpoints;

  // Set the client for the shared category state module
  useEffect(() => {
    setClient(client ?? null);
  }, [client]);

  const handleRemove = useCallback(
    async (id: number) => {
      await removeBreakpoint(id);
    },
    [removeBreakpoint],
  );

  const handleToggle = useCallback(
    async (id: number, enabled: boolean) => {
      await toggleBreakpoint(id, enabled);
    },
    [toggleBreakpoint],
  );

  const handleMuteToggle = useCallback((category: BreakpointCategory) => {
    const state = categoryStates.get(category);
    if (state) {
      state.muted = !state.muted;
      if (state.muted) {
        state.soloed = false; // Can't be both muted and soloed
      }
      syncCategoryStatesToServer();
    }
  }, []);

  const handleSoloToggle = useCallback((category: BreakpointCategory) => {
    const state = categoryStates.get(category);
    if (state) {
      state.soloed = !state.soloed;
      if (state.soloed) {
        state.muted = false; // Can't be both muted and soloed
        // Unsolo all other categories
        for (const [cat, s] of categoryStates.entries()) {
          if (cat !== category) {
            s.soloed = false;
          }
        }
      }
      syncCategoryStatesToServer();
    }
  }, []);

  const handleAdd = useCallback(async () => {
    const trimmed = newBreakpoint.trim();
    if (!trimmed) {
      return;
    }

    let event: string;
    let address: number | undefined;
    let kind: BreakpointDescriptor["kind"];

    if (addMode === "event") {
      event = trimmed;
      address = undefined;
      kind = "event";
    } else if (addMode === "dsp") {
      // Parse step value for DSP breakpoints
      const normalized = trimmed.replace(/^0x/i, "");
      const parsed = Number.parseInt(normalized, 16);
      if (Number.isNaN(parsed)) {
        return;
      }
      event = "dc.aica.dsp.step";
      address = parsed;
      kind = "code";
    } else {
      // Parse address for PC breakpoints
      const normalized = trimmed.replace(/^0x/i, "");
      const parsed = Number.parseInt(normalized, 16);
      if (Number.isNaN(parsed)) {
        return;
      }
      const target = addMode === "pc-sh4" ? "dc.sh4.cpu" : addMode === "pc-arm7" ? "dc.aica.arm7" : "dc.sh4.cpu";
      event = `${target}.pc`;
      address = parsed;
      kind = "code";
    }

    await addBreakpoint(event, address, kind);
    setNewBreakpoint("");
  }, [newBreakpoint, addBreakpoint, addMode]);

  const placeholder =
    addMode === "event"
      ? "Event name (e.g., dc.aica.channel[0].step)"
      : addMode === "dsp"
        ? "Step value (e.g., 0, 5, 0x10)"
        : `Address (e.g., 0x8C0000A0)`;

  // Group breakpoints by category for the all-categories view
  const categorizedBreakpoints: Partial<Record<BreakpointCategory, BreakpointDescriptor[]>> = showCategoryControls
    ? filteredBreakpoints.reduce(
        (acc, bp) => {
          const category = categorizeBreakpoint(bp);
          if (!acc[category]) {
            acc[category] = [];
          }
          acc[category].push(bp);
          return acc;
        },
        {} as Record<BreakpointCategory, BreakpointDescriptor[]>,
      )
    : {};

  // Check if any category is soloed
  const anyCategorySoloed = Array.from(categoryStates.values()).some((s) => s.soloed);

  const categoryLabels: Record<BreakpointCategory, string> = {
    events: "Events",
    sh4: "SH4",
    arm7: "ARM7",
    dsp: "DSP",
  };

  return (
    <Panel>
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

      {showCategoryControls ? (
        // Show grouped view with category controls
        <Box sx={{ overflowY: "auto", flex: 1 }}>
          {(["sh4", "arm7", "dsp", "events"] as BreakpointCategory[]).map((category) => {
            const categoryBps: BreakpointDescriptor[] = categorizedBreakpoints[category] || [];
            if (categoryBps.length === 0) return null;

            const state = categoryStates.get(category);
            const isActive = state && !state.muted && (!anyCategorySoloed || state.soloed);

            return (
              <Box key={category}>
                <Box
                  sx={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    px: 1.5,
                    py: 1,
                    bgcolor: "action.hover",
                    borderBottom: "1px solid",
                    borderColor: "divider",
                  }}
                >
                  <Typography variant="subtitle2" sx={{ fontWeight: 600, opacity: isActive ? 1 : 0.5 }}>
                    {categoryLabels[category]}
                  </Typography>
                  <CategoryControls
                    onMuteToggle={() => handleMuteToggle(category)}
                    onSoloToggle={() => handleSoloToggle(category)}
                    muted={state?.muted ?? false}
                    soloed={state?.soloed ?? false}
                  />
                </Box>
                <List dense disablePadding>
                  {categoryBps.map((bp: BreakpointDescriptor) => (
                    <ListItem
                      key={bp.id}
                      sx={{ opacity: isActive ? 1 : 0.4 }}
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
                            {formatBreakpointDisplay(bp)}
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
                          </Typography>
                        }
                      />
                    </ListItem>
                  ))}
                </List>
              </Box>
            );
          })}
          {filteredBreakpoints.length === 0 && (
            <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
              No breakpoints defined.
            </Typography>
          )}
        </Box>
      ) : (
        // Regular single-category view
        <>
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
                        {formatBreakpointDisplay(bp)}
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
                      </Typography>
                    }
                  />
                </ListItem>
              ))}
            </List>
          )}
        </>
      )}
    </Panel>
  );
};

export const EventsBreakpointsPanel = () => <BreakpointsView title="Events: Breakpoints" addMode="event" showCategoryControls={true} />;

export const Sh4BreakpointsPanel = () => (
  <BreakpointsView title="SH4: Breakpoints" filter={(bp) => bp.event.toLowerCase().includes("sh4")} addMode="pc-sh4" />
);

export const Arm7BreakpointsPanel = () => (
  <BreakpointsView title="ARM7: Breakpoints" filter={(bp) => bp.event.toLowerCase().includes("arm7")} addMode="pc-arm7" />
);

export const DspBreakpointsPanel = () => (
  <BreakpointsView
    title="DSP: Breakpoints"
    filter={(bp) => bp.event.toLowerCase().includes("dsp")}
    addMode="dsp"
  />
);
