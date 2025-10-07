import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { Box, Button, CircularProgress, IconButton, InputAdornment, Paper, Stack, TextField, Tooltip, Typography } from "@mui/material";
import CircleIcon from "@mui/icons-material/Circle";
import RadioButtonUncheckedIcon from "@mui/icons-material/RadioButtonUnchecked";
import RadioButtonCheckedIcon from "@mui/icons-material/RadioButtonChecked";
import MyLocationIcon from "@mui/icons-material/MyLocation";
import ArrowForwardIcon from "@mui/icons-material/ArrowForward";
import ArrowDownwardRoundedIcon from "@mui/icons-material/ArrowDownwardRounded";
import ArrowUpwardRoundedIcon from "@mui/icons-material/ArrowUpwardRounded";
import SubdirectoryArrowRightIcon from "@mui/icons-material/SubdirectoryArrowRight";
import VolumeOffIcon from "@mui/icons-material/VolumeOff";
import VolumeUpIcon from "@mui/icons-material/VolumeUp";
import RefreshIcon from "@mui/icons-material/Refresh";
import type { DisassemblyLine } from "../../lib/debuggerSchema";
import { useSessionStore } from "../../state/sessionStore";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import { categoryStates, syncCategoryStatesToServer, type BreakpointCategory } from "../../state/breakpointCategoryState";

const WHEEL_PIXEL_THRESHOLD = 60;
const INSTRUCTIONS_PER_TICK = 6;
const FETCH_LINE_COUNT = 64;

const formatHexAddress = (value: number) => `0x${value.toString(16).toUpperCase().padStart(8, "0")}`;

const instructionSizeForTarget = (target: string) => {
  switch (target) {
    case "sh4":
      return 2;
    case "arm7":
      return 4;
    case "dsp":
      return 1;
    default:
      return 2;
  }
};

const maxAddressForTarget = (target: string) => (target === "dsp" ? 0x7f : 0xffffffff);

const normalizeAddress = (value: number, max: number, step: number) => {
  const clamped = Math.min(Math.max(0, value), max);
  if (step <= 1) {
    return clamped;
  }
  return clamped - (clamped % step);
};

const formatAddressInput = (target: string, value: number) =>
  target === "dsp" ? value.toString() : formatHexAddress(value);

const formatAddressForDisplay = (target: string, value: number) =>
  target === "dsp" ? value.toString().padStart(3, "0") : formatHexAddress(value);

const parseAddressInput = (target: string, input: string) => {
  const trimmed = input.trim();
  if (!trimmed) {
    return undefined;
  }
  if (/^0x/i.test(trimmed)) {
    const parsed = Number.parseInt(trimmed.replace(/^0x/i, ""), 16);
    return Number.isNaN(parsed) ? undefined : parsed;
  }
  const base = target === "dsp" ? 10 : 16;
  const parsed = Number.parseInt(trimmed, base);
  return Number.isNaN(parsed) ? undefined : parsed;
};

const DisassemblyView = ({
  target,
  defaultAddress,
}: {
  target: string;
  defaultAddress: number;
}) => {
  const [searchParams, setSearchParams] = useSearchParams();
  const client = useSessionStore((state) => state.client);
  const initialized = useDebuggerDataStore((state) => state.initialized);
  const breakpoints = useDebuggerDataStore((state) => state.breakpoints);
  const registersByPath = useDebuggerDataStore((state) => state.registersByPath);
  const addBreakpoint = useDebuggerDataStore((state) => state.addBreakpoint);
  const removeBreakpoint = useDebuggerDataStore((state) => state.removeBreakpoint);
  const toggleBreakpoint = useDebuggerDataStore((state) => state.toggleBreakpoint);
  const [lines, setLines] = useState<DisassemblyLine[]>([]);
  const [loading, setLoading] = useState(false);

  // Initialize address from URL or default
  const initialAddressData = useMemo(() => {
    const paramName = target === "dsp" ? "step" : "address";
    const addressParam = searchParams.get(paramName);
    if (addressParam) {
      const parsed = parseAddressInput(target, addressParam);
      if (parsed !== undefined) {
        return { address: parsed, fromUrl: true };
      }
    }
    return { address: defaultAddress, fromUrl: false };
  }, [searchParams, target, defaultAddress]);

  const [address, setAddress] = useState(initialAddressData.address);
  const [addressInput, setAddressInput] = useState(formatAddressInput(target, initialAddressData.address));
  const [error, setError] = useState<string | undefined>();
  const requestIdRef = useRef(0);
  const wheelRemainder = useRef(0);
  const pendingScrollSteps = useRef(0);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const currentPcRef = useRef<number | undefined>(undefined);
  const targetAddressRef = useRef<number | undefined>(undefined);
  const targetTimestampRef = useRef<number | undefined>(undefined);
  const lineRefsMap = useRef<Map<number, HTMLDivElement>>(new Map());
  const [, forceUpdate] = useState(0);

  const instructionSize = useMemo(() => instructionSizeForTarget(target), [target]);
  const maxAddress = useMemo(() => maxAddressForTarget(target), [target]);

  // Get category for this disassembly view
  const category: BreakpointCategory = target === "sh4" ? "sh4" : target === "arm7" ? "arm7" : "dsp";
  const categoryState = categoryStates.get(category);

  const handleMuteToggle = useCallback(() => {
    const state = categoryStates.get(category);
    if (state) {
      state.muted = !state.muted;
      if (state.muted) {
        state.soloed = false; // Can't be both muted and soloed
      }
      forceUpdate((v) => v + 1);
      syncCategoryStatesToServer();
    }
  }, [category]);

  const handleSoloToggle = useCallback(() => {
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
      forceUpdate((v) => v + 1);
      syncCategoryStatesToServer();
    }
  }, [category]);

  // Get current PC/step value for this target and update DOM directly
  useEffect(() => {
    const cpuPath = target === "dsp" ? "dc.aica.dsp" : target === "sh4" ? "dc.sh4.cpu" : "dc.aica.arm7";
    const counterName = target === "dsp" ? "STEP" : "PC";
    const registers = registersByPath[cpuPath];
    const pcReg = registers?.find((r) => r.name === counterName);

    let newPc: number | undefined;
    if (pcReg?.value) {
      const parsed = Number.parseInt(pcReg.value.replace(/^0x/i, ""), 16);
      newPc = Number.isNaN(parsed) ? undefined : parsed;
    }

    // Update DOM directly without triggering re-render
    const oldPc = currentPcRef.current;
    if (oldPc !== newPc) {
      // Remove current styling from old PC
      if (oldPc !== undefined) {
        const oldElement = lineRefsMap.current.get(oldPc);
        if (oldElement) {
          oldElement.classList.remove("current-instruction");
        }
      }

      // Add current styling to new PC
      if (newPc !== undefined) {
        const newElement = lineRefsMap.current.get(newPc);
        if (newElement) {
          newElement.classList.add("current-instruction");
        }
      }

      currentPcRef.current = newPc;
    }
  }, [registersByPath, target]);

  // Map addresses to breakpoints
  const breakpointsByAddress = useMemo(() => {
    const map = new Map<number, { id: string; enabled: boolean }>();
    const cpuPath = target === "dsp" ? "dc.aica.dsp" : target === "sh4" ? "dc.sh4.cpu" : "dc.aica.arm7";
    const counterName = target === "dsp" ? "step" : "pc";

    for (const bp of breakpoints) {
      // Match pattern like "dc.sh4.cpu.pc == 0x8C0000A0" or "dc.aica.dsp.step == 0x20"
      const match = bp.location.match(new RegExp(`${cpuPath}\\.${counterName}\\s*==\\s*0x([0-9A-Fa-f]+)`));
      if (match) {
        const addr = Number.parseInt(match[1], 16);
        map.set(addr, { id: bp.id, enabled: bp.enabled });
      }
    }
    return map;
  }, [breakpoints, target]);

  const fetchDisassembly = useCallback(
    async (addr: number) => {
      if (!client) {
        return;
      }
      const requestId = ++requestIdRef.current;
      setLoading(true);
      setError(undefined);
      try {
        // Calculate how many instructions can fit before hitting max address
        const remainingAddressSpace = maxAddress - addr;
        const maxPossibleInstructions = Math.floor(remainingAddressSpace / instructionSize) + 1;
        const count = Math.min(FETCH_LINE_COUNT, Math.max(1, maxPossibleInstructions));

        const result = await client.fetchDisassembly({ target, address: addr, count });
        if (requestIdRef.current !== requestId) {
          return;
        }
        setLines(result.lines);
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
    [client, target, maxAddress, instructionSize],
  );

  useEffect(() => {
    const normalized = normalizeAddress(address, maxAddress, instructionSize);
    // Display the address 10 instructions down (the intended target), not the fetch start
    const displayAddress = Math.min(maxAddress, normalized + instructionSize * 10);
    setAddressInput(formatAddressInput(target, displayAddress));

    if (initialized) {
      void fetchDisassembly(normalized);
    }
  }, [address, fetchDisassembly, instructionSize, maxAddress, target, initialized]);

  // Trigger highlight effect when loaded from URL
  useEffect(() => {
    if (!initialAddressData.fromUrl || !initialized || lines.length === 0) {
      return;
    }

    // Set target address for animation trigger
    targetAddressRef.current = initialAddressData.address;
    targetTimestampRef.current = Date.now();

    // Update DOM when lines are available
    setTimeout(() => {
      const element = lineRefsMap.current.get(initialAddressData.address);
      if (element) {
        element.classList.remove("target-address");
        // Force reflow to restart animation
        void element.offsetWidth;
        element.classList.add("target-address");
      }
    }, 0);
  }, [initialAddressData, initialized, lines]);

  // Process pending scroll steps after loading completes
  useEffect(() => {
    if (!loading && pendingScrollSteps.current !== 0) {
      const steps = pendingScrollSteps.current;
      pendingScrollSteps.current = 0;
      setAddress((prev) => normalizeAddress(prev + steps * instructionSize, maxAddress, instructionSize));
    }
  }, [loading, instructionSize, maxAddress]);

  const adjustAddress = useCallback(
    (steps: number) => {
      setAddress((prev) => {
        const newAddr = normalizeAddress(prev + steps * instructionSize, maxAddress, instructionSize);
        const paramName = target === "dsp" ? "step" : "address";
        setSearchParams({ [paramName]: formatAddressInput(target, newAddr) });
        return newAddr;
      });
    },
    [instructionSize, maxAddress, setSearchParams, target],
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
    const parsed = parseAddressInput(target, addressInput);
    if (parsed === undefined) {
      return;
    }
    // Start 10 instructions before for context
    const contextOffset = instructionSize * 10;
    const targetAddress = Math.max(0, parsed - contextOffset);
    const normalizedTarget = normalizeAddress(targetAddress, maxAddress, instructionSize);

    setAddress(normalizedTarget);
    const paramName = target === "dsp" ? "step" : "address";
    setSearchParams({ [paramName]: formatAddressInput(target, parsed) });

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
  }, [addressInput, instructionSize, maxAddress, target, setSearchParams]);

  const handleRefresh = useCallback(() => {
    void fetchDisassembly(address);
  }, [fetchDisassembly, address]);

  const handleBreakpointClick = useCallback(
    async (lineAddress: number) => {
      const existing = breakpointsByAddress.get(lineAddress);
      if (existing) {
        // Remove if exists
        await removeBreakpoint(existing.id);
      } else {
        // Add new breakpoint
        const cpuPath = target === "dsp" ? "dc.aica.dsp" : target === "sh4" ? "dc.sh4.cpu" : "dc.aica.arm7";
        const counterName = target === "dsp" ? "step" : "pc";
        const location = `${cpuPath}.${counterName} == ${formatHexAddress(lineAddress)}`;
        await addBreakpoint(location, "code");
      }
    },
    [breakpointsByAddress, target, addBreakpoint, removeBreakpoint],
  );

  const handleBreakpointToggle = useCallback(
    async (lineAddress: number, event: React.MouseEvent) => {
      event.stopPropagation();
      const existing = breakpointsByAddress.get(lineAddress);
      if (existing) {
        await toggleBreakpoint(existing.id, !existing.enabled);
      }
    },
    [breakpointsByAddress, toggleBreakpoint],
  );

  const handleGotoPC = useCallback(() => {
    if (currentPcRef.current !== undefined) {
      // Start 10 instructions before for context
      const contextOffset = instructionSize * 10;
      const targetAddress = Math.max(0, currentPcRef.current - contextOffset);
      setAddress(targetAddress);
    }
  }, [instructionSize]);

  const handleStep = useCallback(async () => {
    if (!client) {
      return;
    }
    try {
      await (client as any).rpc.call("control.step", {
        target,
        granularity: "instruction",
      });
      // State will be updated via notification from server
    } catch (error) {
      console.error("Failed to step", error);
    }
  }, [client, target]);

  const handleStepIn = useCallback(async () => {
    if (!client) {
      return;
    }
    try {
      await (client as any).rpc.call("control.step", {
        target,
        granularity: "instruction",
        modifiers: ["into"],
      });
      // State will be updated via notification from server
    } catch (error) {
      console.error("Failed to step in", error);
    }
  }, [client, target]);

  const handleStepOut = useCallback(async () => {
    if (!client) {
      return;
    }
    try {
      await (client as any).rpc.call("control.step", {
        target,
        granularity: "instruction",
        modifiers: ["out"],
      });
      // State will be updated via notification from server
    } catch (error) {
      console.error("Failed to step out", error);
    }
  }, [client, target]);

  const showStepInOut = target === "sh4" || target === "arm7";
  const isDsp = target === "dsp";
  const stepLabel = isDsp ? "STEP" : "Step Over";
  const StepIcon = isDsp ? ArrowForwardIcon : SubdirectoryArrowRightIcon;

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
          <Tooltip title={stepLabel}>
            <IconButton
              size="small"
              onClick={handleStep}
            >
              <StepIcon fontSize="small" />
            </IconButton>
          </Tooltip>
          {showStepInOut && (
            <>
              <Tooltip title="Step In">
                <IconButton
                  size="small"
                  onClick={handleStepIn}
                >
                  <ArrowDownwardRoundedIcon fontSize="small" />
                </IconButton>
              </Tooltip>
              <Tooltip title="Step Out">
                <IconButton
                  size="small"
                  onClick={handleStepOut}
                >
                  <ArrowUpwardRoundedIcon fontSize="small" />
                </IconButton>
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
          <Tooltip title={target === "dsp" ? "Go to current STEP" : "Go to current PC"}>
            <span>
              <IconButton
                size="small"
                onClick={handleGotoPC}
                disabled={currentPcRef.current === undefined}
              >
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
                },
                "& .target-address": {
                  border: "2px solid",
                  borderColor: "warning.main",
                  animation: "fadeOutTarget 2s forwards",
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
              {lines.map((line) => {
                const commentText = line.comment ? `; ${line.comment}` : "";
                const mnemonicSegment = line.operands ? `${line.mnemonic} ${line.operands}` : line.mnemonic;
                const breakpoint = breakpointsByAddress.get(line.address);
                const hasBreakpoint = !!breakpoint;
                const breakpointEnabled = breakpoint?.enabled ?? false;

                return (
                  <Box
                    key={`${line.address}-${hasBreakpoint}-${breakpointEnabled}`}
                    ref={(el: HTMLDivElement | null) => {
                      if (el) {
                        lineRefsMap.current.set(line.address, el);
                        // Apply current styling if this line is the current PC
                        if (currentPcRef.current === line.address) {
                          el.classList.add("current-instruction");
                        }
                      } else {
                        lineRefsMap.current.delete(line.address);
                      }
                    }}
                    sx={{
                      display: "grid",
                      gridTemplateColumns: target === "dsp" ? "24px 80px 120px 1fr" : "24px 140px 140px 1fr",
                      gap: 1,
                      alignItems: "stretch",
                      px: 0.5,
                      py: 0,
                      border: "2px solid transparent",
                      borderRadius: 1,
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
                          void handleBreakpointToggle(line.address, e);
                        } else {
                          void handleBreakpointClick(line.address);
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
                      {formatAddressForDisplay(target, line.address)}
                    </Typography>
                    <Typography component="span" sx={{ color: "text.secondary" }}>
                      {line.bytes}
                    </Typography>
                    <Typography component="span" sx={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
                      {mnemonicSegment}
                      {commentText && (
                        <Box component="span" sx={{ color: "text.secondary", ml: 1, whiteSpace: "pre-wrap", wordBreak: "break-word" }}>
                          {commentText}
                        </Box>
                      )}
                    </Typography>
                  </Box>
                );
              })}
            </Stack>
          )}
        </Box>
      )}
      </Box>
    </Paper>
  );
};

export const Sh4DisassemblyPanel = () => (
  <DisassemblyView target="sh4" defaultAddress={0x8c0000a0} />
);

export const Arm7DisassemblyPanel = () => (
  <DisassemblyView target="arm7" defaultAddress={0x00200000} />
);

export const DspDisassemblyPanel = () => (
  <DisassemblyView target="dsp" defaultAddress={0x00000000} />
);
