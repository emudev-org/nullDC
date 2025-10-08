import { memo, useCallback, useEffect, useRef, useState } from "react";
import { Box, Button, CircularProgress, IconButton, InputAdornment, Paper, Stack, TextField, Tooltip, Typography } from "@mui/material";
import CircleIcon from "@mui/icons-material/Circle";
import RadioButtonUncheckedIcon from "@mui/icons-material/RadioButtonUnchecked";
import RadioButtonCheckedIcon from "@mui/icons-material/RadioButtonChecked";
import MyLocationIcon from "@mui/icons-material/MyLocation";
import ArrowForwardIcon from "@mui/icons-material/ArrowForward";
import ArrowDownwardRoundedIcon from "@mui/icons-material/ArrowDownwardRounded";
import ArrowUpwardRoundedIcon from "@mui/icons-material/ArrowUpwardRounded";
import VolumeOffIcon from "@mui/icons-material/VolumeOff";
import VolumeUpIcon from "@mui/icons-material/VolumeUp";
import RefreshIcon from "@mui/icons-material/Refresh";
import type { DisassemblyLine } from "../../lib/debuggerSchema";

const WHEEL_PIXEL_THRESHOLD = 60;
const INSTRUCTIONS_PER_TICK = 6;

export interface DisassemblyViewConfig {
  /** Instruction size in bytes (e.g., 2 for SH4, 4 for ARM7, 1 for DSP) */
  instructionSize: number;
  /** Maximum address value for this target */
  maxAddress: number;
  /** Format an address value for input field display */
  formatAddressInput: (value: number) => string;
  /** Format an address value for disassembly line display */
  formatAddressDisplay: (value: number) => string;
  /** Parse an address string from user input */
  parseAddressInput: (input: string) => number | undefined;
  /** Column widths for the grid layout */
  gridColumns: string;
  /** Label for the primary step button */
  stepLabel: string;
  /** Icon for the primary step button */
  stepIcon: typeof ArrowForwardIcon;
  /** Whether to show step in/out buttons */
  showStepInOut: boolean;
  /** URL parameter name for address (e.g., "address" or "step") */
  urlParamName: string;
}

export interface DisassemblyViewCallbacks {
  /** Fetch disassembly lines starting at the given address */
  onFetchDisassembly: (address: number, count: number) => Promise<DisassemblyLine[]>;
  /** Handle primary step action (Step or Step Over depending on target) */
  onStep: () => Promise<void>;
  /** Handle step in action */
  onStepIn?: () => Promise<void>;
  /** Handle step out action */
  onStepOut?: () => Promise<void>;
  /** Add a breakpoint at the given address */
  onBreakpointAdd: (address: number) => Promise<void>;
  /** Remove a breakpoint by ID */
  onBreakpointRemove: (id: number) => Promise<void>;
  /** Toggle a breakpoint's enabled state */
  onBreakpointToggle: (id: number, enabled: boolean) => Promise<void>;
  /** Handle mute toggle for this view's category */
  onMuteToggle: () => void;
  /** Handle solo toggle for this view's category */
  onSoloToggle: () => void;
}

export interface DisassemblyViewProps {
  /** Configuration for address formatting and display */
  config: DisassemblyViewConfig;
  /** Callbacks for all actions */
  callbacks: DisassemblyViewCallbacks;
  /** Default address to start at */
  defaultAddress: number;
  /** Current program counter value */
  currentPc?: number;
  /** Map of addresses to breakpoint info */
  breakpointsByAddress: Map<number, { id: number; enabled: boolean }>;
  /** Whether the debugger is initialized */
  initialized: boolean;
  /** Current execution state */
  executionState: "running" | "paused";
  /** Category state (muted/soloed) */
  categoryState?: { muted: boolean; soloed: boolean };
  /** Initial address from URL if any */
  initialUrlAddress?: { address: number; fromUrl: boolean };
}

const normalizeAddress = (value: number, max: number, step: number) => {
  const clamped = Math.min(Math.max(0, value), max);
  if (step <= 1) {
    return clamped;
  }
  return clamped - (clamped % step);
};

interface DisassemblyLineItemProps {
  line: DisassemblyLine;
  currentPc?: number;
  executionState: "running" | "paused";
  breakpoint?: { id: number; enabled: boolean };
  config: DisassemblyViewConfig;
  onBreakpointClick: (address: number) => void;
  onBreakpointToggle: (address: number, event: React.MouseEvent) => void;
  lineRef: (address: number, el: HTMLDivElement | null) => void;
}

const DisassemblyLineItem = memo(({
  line,
  currentPc: _currentPc,
  executionState: _executionState,
  breakpoint,
  config,
  onBreakpointClick,
  onBreakpointToggle,
  lineRef,
}: DisassemblyLineItemProps) => {
  const hasBreakpoint = !!breakpoint;
  const breakpointEnabled = breakpoint?.enabled ?? false;

  return (
    <Box
      ref={(el: HTMLDivElement | null) => lineRef(line.address, el)}
      sx={{
        display: "grid",
        gridTemplateColumns: config.gridColumns,
        gap: 1,
        alignItems: "stretch",
        px: 0.5,
        py: 0,
        border: "2px solid transparent",
        borderRadius: 1,
        "&:hover": {
          borderBottomColor: "primary.main",
        },
        "&:hover .breakpoint-gutter": {
          opacity: 1,
        },
      }}
    >
      <Box
        className="breakpoint-gutter"
        sx={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          opacity: hasBreakpoint ? 1 : 0,
          cursor: "pointer",
          transition: "opacity 0.1s",
          alignSelf: "stretch",
        }}
        onClick={(e) => {
          if ((e.shiftKey || e.ctrlKey || e.metaKey) && hasBreakpoint) {
            onBreakpointToggle(line.address, e);
          } else {
            onBreakpointClick(line.address);
          }
        }}
      >
        {hasBreakpoint ? (
          breakpointEnabled ? (
            <CircleIcon sx={{ fontSize: 16, color: "error.main" }} />
          ) : (
            <RadioButtonUncheckedIcon sx={{ fontSize: 16, color: "error.main" }} />
          )
        ) : (
          <RadioButtonUncheckedIcon sx={{ fontSize: 16, color: "text.disabled", opacity: 0.3 }} />
        )}
      </Box>
      <Typography component="span" sx={{ color: "text.secondary" }}>
        {config.formatAddressDisplay(line.address)}
      </Typography>
      <Typography component="span" sx={{ color: "text.secondary" }}>
        {line.bytes}
      </Typography>
      <Typography component="span" sx={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
        {line.disassembly}
      </Typography>
    </Box>
  );
});

DisassemblyLineItem.displayName = "DisassemblyLineItem";

export const DisassemblyView = ({
  config,
  callbacks,
  defaultAddress,
  currentPc,
  breakpointsByAddress,
  initialized,
  executionState,
  categoryState,
  initialUrlAddress,
}: DisassemblyViewProps) => {
  const [lines, setLines] = useState<DisassemblyLine[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | undefined>();

  // Initialize address from URL or default
  const initialAddress = initialUrlAddress?.address ?? defaultAddress;
  const [address, setAddress] = useState(initialAddress);
  const [addressInput, setAddressInput] = useState(config.formatAddressInput(initialAddress));

  // Update address when URL changes (only on navigation, not on every render)
  useEffect(() => {
    if (initialUrlAddress?.fromUrl) {
      const contextOffset = config.instructionSize * 10;
      const targetAddress = Math.max(0, initialUrlAddress.address - contextOffset);
      const normalizedTarget = normalizeAddress(targetAddress, config.maxAddress, config.instructionSize);
      setAddress(normalizedTarget);

      // Set target for highlighting
      targetAddressRef.current = initialUrlAddress.address;
      targetTimestampRef.current = Date.now();
    }
  }, [initialUrlAddress?.address, initialUrlAddress?.fromUrl, config.instructionSize, config.maxAddress]);

  const requestIdRef = useRef(0);
  const wheelRemainder = useRef(0);
  const pendingScrollSteps = useRef(0);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const currentPcRef = useRef<number | undefined>(currentPc);
  const targetAddressRef = useRef<number | undefined>(undefined);
  const targetTimestampRef = useRef<number | undefined>(undefined);
  const lineRefsMap = useRef<Map<number, HTMLDivElement>>(new Map());
  const [, forceUpdate] = useState(0);

  // Track previous execution state for detecting pauses
  const prevExecutionStateRef = useRef(executionState);

  // Update DOM directly when PC or execution state changes
  useEffect(() => {
    const isPaused = executionState === "paused";
    const oldPc = currentPcRef.current;
    const newPc = currentPc;
    const wasRunning = prevExecutionStateRef.current === "running";

    if (oldPc !== newPc) {
      // Remove current styling from old PC
      if (oldPc !== undefined) {
        const oldElement = lineRefsMap.current.get(oldPc);
        if (oldElement) {
          oldElement.classList.remove("current-instruction");
        }
      }

      // Add current styling to new PC only if paused
      if (newPc !== undefined && isPaused) {
        const newElement = lineRefsMap.current.get(newPc);
        if (newElement) {
          newElement.classList.add("current-instruction");
        }
      }

      // Update ref after processing
      currentPcRef.current = newPc;
    } else if (oldPc !== undefined) {
      // If PC hasn't changed but execution state changed, update styling
      const element = lineRefsMap.current.get(oldPc);
      if (element) {
        if (isPaused) {
          element.classList.add("current-instruction");
        } else {
          element.classList.remove("current-instruction");
        }
      }
    }

    // Auto-scroll to current PC when transitioning from running to paused
    if (isPaused && wasRunning && newPc !== undefined) {
      const contextOffset = config.instructionSize * 10;
      const targetAddress = Math.max(0, newPc - contextOffset);
      const normalizedTarget = normalizeAddress(targetAddress, config.maxAddress, config.instructionSize);
      setAddress(normalizedTarget);
    }

    // Update previous execution state
    prevExecutionStateRef.current = executionState;
  }, [currentPc, executionState, config.instructionSize, config.maxAddress]);

  const fetchDisassembly = useCallback(
    async (addr: number) => {
      const requestId = ++requestIdRef.current;
      setLoading(true);
      setError(undefined);
      try {
        // Calculate how many instructions can fit before hitting max address
        const remainingAddressSpace = config.maxAddress - addr;
        const maxPossibleInstructions = Math.floor(remainingAddressSpace / config.instructionSize) + 1;
        const count = Math.min(64, Math.max(1, maxPossibleInstructions));

        const result = await callbacks.onFetchDisassembly(addr, count);
        if (requestIdRef.current !== requestId) {
          return;
        }
        setLines(result);
      } catch (err) {
        if (requestIdRef.current === requestId) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (requestIdRef.current === requestId) {
          setLoading(false);
        }
      }
    },
    [callbacks, config.maxAddress, config.instructionSize],
  );

  useEffect(() => {
    const normalized = normalizeAddress(address, config.maxAddress, config.instructionSize);
    // Display the address 10 instructions down (the intended target), not the fetch start
    const displayAddress = Math.min(config.maxAddress, normalized + config.instructionSize * 10);
    setAddressInput(config.formatAddressInput(displayAddress));

    if (initialized) {
      void fetchDisassembly(normalized);
    }
  }, [address, fetchDisassembly, config, initialized]);

  // Trigger highlight effect when URL address changes (or action_guid changes for re-clicks)
  useEffect(() => {
    if (!initialUrlAddress?.fromUrl || !initialized) {
      return;
    }

    const targetAddr = initialUrlAddress.address;

    // Helper to apply highlight with auto-removal
    const applyHighlight = (el: HTMLDivElement) => {
      el.classList.remove("target-address");
      // Force reflow to restart animation
      void el.offsetWidth;
      el.classList.add("target-address");

      // Remove class after animation completes (2s animation duration)
      setTimeout(() => {
        el.classList.remove("target-address");
      }, 2000);
    };

    // Check if element is already in DOM
    const element = lineRefsMap.current.get(targetAddr);
    if (element) {
      applyHighlight(element);
    } else {
      // Address not visible yet, try again when it loads
      setTimeout(() => {
        const el = lineRefsMap.current.get(targetAddr);
        if (el) {
          applyHighlight(el);
        }
      }, 100);
    }
  }, [initialUrlAddress, initialized]);

  // Process pending scroll steps after loading completes
  useEffect(() => {
    if (!loading && pendingScrollSteps.current !== 0) {
      const steps = pendingScrollSteps.current;
      pendingScrollSteps.current = 0;
      setAddress((prev: number) => normalizeAddress(prev + steps * config.instructionSize, config.maxAddress, config.instructionSize));
    }
  }, [loading, config.instructionSize, config.maxAddress]);

  const adjustAddress = useCallback(
    (steps: number) => {
      setAddress((prev: number) => {
        const newAddr = normalizeAddress(prev + steps * config.instructionSize, config.maxAddress, config.instructionSize);
        return newAddr;
      });
    },
    [config.instructionSize, config.maxAddress],
  );

  const handleWheel = useCallback(
    (event: WheelEvent) => {
      event.preventDefault();
      wheelRemainder.current += event.deltaY;

      while (Math.abs(wheelRemainder.current) >= WHEEL_PIXEL_THRESHOLD) {
        const direction = wheelRemainder.current > 0 ? 1 : -1;
        const steps = direction * INSTRUCTIONS_PER_TICK;

        if (loading) {
          // Queue the scroll while loading
          pendingScrollSteps.current += steps;
        } else {
          adjustAddress(steps);
        }

        wheelRemainder.current -= direction * WHEEL_PIXEL_THRESHOLD;
      }
    },
    [adjustAddress, loading],
  );

  useEffect(() => {
    const node = containerRef.current;
    if (!node) {
      return;
    }

    const listener = (event: WheelEvent) => {
      handleWheel(event);
    };

    node.addEventListener("wheel", listener, { passive: false });

    return () => {
      node.removeEventListener("wheel", listener);
    };
  }, [handleWheel]);

  const handleAddressSubmit = useCallback(() => {
    const parsed = config.parseAddressInput(addressInput);
    if (parsed === undefined) {
      return;
    }
    // Start 10 instructions before for context
    const contextOffset = config.instructionSize * 10;
    const targetAddress = Math.max(0, parsed - contextOffset);
    const normalizedTarget = normalizeAddress(targetAddress, config.maxAddress, config.instructionSize);

    setAddress(normalizedTarget);

    // Set target address and timestamp for animation trigger
    targetAddressRef.current = parsed;
    targetTimestampRef.current = Date.now();

    // Update DOM when lines are available
    setTimeout(() => {
      const element = lineRefsMap.current.get(parsed);
      if (element) {
        element.classList.remove("target-address");
        // Force reflow to restart animation
        void element.offsetWidth;
        element.classList.add("target-address");
      }
    }, 0);
  }, [addressInput, config]);

  const handleRefresh = useCallback(() => {
    void fetchDisassembly(address);
  }, [fetchDisassembly, address]);

  const handleBreakpointClick = useCallback(
    async (lineAddress: number) => {
      const existing = breakpointsByAddress.get(lineAddress);
      if (existing) {
        await callbacks.onBreakpointRemove(existing.id);
      } else {
        await callbacks.onBreakpointAdd(lineAddress);
      }
    },
    [breakpointsByAddress, callbacks],
  );

  const handleBreakpointToggle = useCallback(
    async (lineAddress: number, event: React.MouseEvent) => {
      event.stopPropagation();
      const existing = breakpointsByAddress.get(lineAddress);
      if (existing) {
        await callbacks.onBreakpointToggle(existing.id, !existing.enabled);
      }
    },
    [breakpointsByAddress, callbacks],
  );

  const handleLineRef = useCallback(
    (address: number, el: HTMLDivElement | null) => {
      if (el) {
        lineRefsMap.current.set(address, el);
        // Apply current styling if this line is the current PC and we're paused
        if (currentPc === address && executionState === "paused") {
          el.classList.add("current-instruction");
        }
      } else {
        lineRefsMap.current.delete(address);
      }
    },
    [currentPc, executionState],
  );

  const handleGotoPC = useCallback(() => {
    if (currentPc !== undefined) {
      // Start 10 instructions before for context
      const contextOffset = config.instructionSize * 10;
      const targetAddress = Math.max(0, currentPc - contextOffset);
      setAddress(targetAddress);
    }
  }, [currentPc, config.instructionSize]);

  const handleStep = useCallback(async () => {
    if (executionState !== "paused") {
      return;
    }
    try {
      await callbacks.onStep();
    } catch (error) {
      console.error("Failed to step", error);
    }
  }, [callbacks, executionState]);

  const handleStepIn = useCallback(async () => {
    if (executionState !== "paused" || !callbacks.onStepIn) {
      return;
    }
    try {
      await callbacks.onStepIn();
    } catch (error) {
      console.error("Failed to step in", error);
    }
  }, [callbacks, executionState]);

  const handleStepOut = useCallback(async () => {
    if (executionState !== "paused" || !callbacks.onStepOut) {
      return;
    }
    try {
      await callbacks.onStepOut();
    } catch (error) {
      console.error("Failed to step out", error);
    }
  }, [callbacks, executionState]);

  const handleMuteToggle = useCallback(() => {
    callbacks.onMuteToggle();
    forceUpdate((v) => v + 1);
  }, [callbacks]);

  const handleSoloToggle = useCallback(() => {
    callbacks.onSoloToggle();
    forceUpdate((v) => v + 1);
  }, [callbacks]);

  const stepDisabled = executionState !== "paused";
  const StepIcon = config.stepIcon;

  return (
    <Paper
      elevation={0}
      sx={{
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        height: "100%",
      }}
    >
      <Box
        sx={{
          px: 2,
          py: 1,
          display: "flex",
          alignItems: "center",
          gap: 1,
          borderBottom: "1px solid",
          borderColor: "divider",
        }}
      >
        <Stack direction="row" spacing={0.5} alignItems="center">
          <Tooltip title={config.stepLabel}>
            <span>
              <IconButton size="small" onClick={handleStep} disabled={stepDisabled}>
                <StepIcon fontSize="small" />
              </IconButton>
            </span>
          </Tooltip>
          {config.showStepInOut && (
            <>
              <Tooltip title="Step In">
                <span>
                  <IconButton size="small" onClick={handleStepIn} disabled={stepDisabled}>
                    <ArrowDownwardRoundedIcon fontSize="small" />
                  </IconButton>
                </span>
              </Tooltip>
              <Tooltip title="Step Out">
                <span>
                  <IconButton size="small" onClick={handleStepOut} disabled={stepDisabled}>
                    <ArrowUpwardRoundedIcon fontSize="small" />
                  </IconButton>
                </span>
              </Tooltip>
            </>
          )}
        </Stack>
        <Stack direction="row" spacing={1} alignItems="center" sx={{ flex: 1, justifyContent: "flex-end" }}>
          <Tooltip title={categoryState?.muted ? "Unmute category" : "Mute category"}>
            <IconButton size="small" onClick={handleMuteToggle} color={categoryState?.muted ? "warning" : "default"}>
              {categoryState?.muted ? <VolumeOffIcon fontSize="small" /> : <VolumeUpIcon fontSize="small" />}
            </IconButton>
          </Tooltip>
          <Tooltip title={categoryState?.soloed ? "Unsolo category" : "Solo category"}>
            <IconButton size="small" onClick={handleSoloToggle} color={categoryState?.soloed ? "primary" : "default"}>
              {categoryState?.soloed ? <RadioButtonCheckedIcon fontSize="small" /> : <RadioButtonUncheckedIcon fontSize="small" />}
            </IconButton>
          </Tooltip>
          <Tooltip title="Go to current PC">
            <span>
              <IconButton size="small" onClick={handleGotoPC} disabled={currentPc === undefined}>
                <MyLocationIcon fontSize="small" />
              </IconButton>
            </span>
          </Tooltip>
          <TextField
            size="small"
            value={addressInput}
            onChange={(event) => setAddressInput(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                handleAddressSubmit();
              }
            }}
            sx={{ width: 160 }}
            slotProps={{
              input: {
                endAdornment: (
                  <InputAdornment position="end">
                    <Button size="small" onClick={handleAddressSubmit} sx={{ minWidth: "auto", px: 1 }}>
                      Go
                    </Button>
                  </InputAdornment>
                ),
              },
            }}
          />
          <Tooltip title="Refresh">
            <IconButton size="small" onClick={handleRefresh} disabled={loading}>
              <RefreshIcon fontSize="small" />
            </IconButton>
          </Tooltip>
        </Stack>
      </Box>
      <Box sx={{ flex: 1, overflow: "auto" }}>
        {!initialized ? (
          <Stack alignItems="center" justifyContent="center" sx={{ height: "100%" }}>
            <Typography variant="body2" color="text.secondary">
              No Data
            </Typography>
          </Stack>
        ) : error ? (
          <Typography variant="body2" color="error" sx={{ p: 2 }}>
            {error}
          </Typography>
        ) : lines.length === 0 && loading ? (
          <Stack alignItems="center" justifyContent="center" sx={{ height: "100%" }} spacing={1}>
            <CircularProgress size={18} />
            <Typography variant="body2" color="text.secondary">
              Loading disassembly.
            </Typography>
          </Stack>
        ) : (
          <Box
            ref={containerRef}
            sx={{
              flex: 1,
              height: "100%",
              overflow: "hidden",
              fontFamily: "monospace",
              fontSize: "13px",
              p: 1.5,
              position: "relative",
              cursor: "default",
            }}
          >
            {loading && (
              <Stack direction="row" spacing={1} alignItems="center" sx={{ position: "absolute", top: 8, right: 16 }}>
                <CircularProgress size={12} thickness={5} />
                <Typography variant="caption" color="text.secondary">
                  Updating
                </Typography>
              </Stack>
            )}
            {lines.length === 0 ? (
              <Typography variant="body2" color="text.secondary">
                No disassembly returned.
              </Typography>
            ) : (
              <Stack
                spacing={0}
                sx={{
                  "& .current-instruction": {
                    color: "primary.main",
                    fontWeight: 600,
                    border: "2px solid",
                    borderColor: "success.main",
                    "&:hover": {
                      borderColor: "success.main !important",
                    },
                  },
                  "& .target-address": {
                    border: "2px solid",
                    borderColor: "warning.main",
                    animation: "fadeOutTarget 2s forwards",
                    "&:hover": {
                      borderColor: "inherit !important",
                    },
                  },
                  "@keyframes fadeOutTarget": {
                    "0%": {
                      borderColor: "warning.main",
                    },
                    "100%": {
                      borderColor: "transparent",
                    },
                  },
                }}
              >
                {lines.map((line) => (
                  <DisassemblyLineItem
                    key={line.address}
                    line={line}
                    currentPc={currentPc}
                    executionState={executionState}
                    breakpoint={breakpointsByAddress.get(line.address)}
                    config={config}
                    onBreakpointClick={handleBreakpointClick}
                    onBreakpointToggle={handleBreakpointToggle}
                    lineRef={handleLineRef}
                  />
                ))}
              </Stack>
            )}
          </Box>
        )}
      </Box>
    </Paper>
  );
};
