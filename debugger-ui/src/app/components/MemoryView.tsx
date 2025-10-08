import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Box, Button, CircularProgress, IconButton, InputAdornment, Stack, TextField, Tooltip, Typography } from "@mui/material";
import RefreshIcon from "@mui/icons-material/Refresh";
import type { MemorySlice } from "../../lib/debuggerSchema";

type MemoryRow = {
  id: number;
  address: number;
  hex: string;
  ascii: string;
};

const WHEEL_PIXEL_THRESHOLD = 60;
const MEMORY_SCROLL_BYTES = 96;
const BYTES_PER_ROW = 16;

export interface MemoryViewConfig {
  /** Format an address value for display */
  formatAddress: (value: number) => string;
  /** Parse an address string from user input */
  parseAddress: (input: string) => number | undefined;
  /** Maximum address value */
  maxAddress: number;
  /** Number of bytes to fetch */
  length: number;
}

export interface MemoryViewCallbacks {
  /** Fetch memory slice starting at the given address */
  onFetchMemory: (address: number, length: number, encoding?: MemorySlice["encoding"], wordSize?: MemorySlice["wordSize"]) => Promise<MemorySlice>;
  /** Handle address change (for URL sync) */
  onAddressChange?: (address: number) => void;
}

export interface MemoryViewProps {
  /** Configuration for address formatting and display */
  config: MemoryViewConfig;
  /** Callbacks for all actions */
  callbacks: MemoryViewCallbacks;
  /** Default address to start at */
  defaultAddress: number;
  /** Whether the debugger is initialized */
  initialized: boolean;
  /** Initial address from URL if any */
  initialUrlAddress?: { address: number; fromUrl: boolean };
  /** Encoding for memory display */
  encoding?: MemorySlice["encoding"];
  /** Word size for memory display */
  wordSize?: MemorySlice["wordSize"];
}

const clampAddress = (value: number, max: number, alignment = 1) => {
  let clamped = value;
  if (clamped < 0) {
    clamped = 0;
  }
  if (clamped > max) {
    clamped = max;
  }
  // Round down to alignment boundary
  return clamped - (clamped % alignment);
};

export const MemoryView = ({
  config,
  callbacks,
  defaultAddress,
  initialized,
  initialUrlAddress,
  encoding,
  wordSize,
}: MemoryViewProps) => {
  const [slice, setSlice] = useState<MemorySlice | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | undefined>();

  // Initialize address from URL or default
  const initialAddress = initialUrlAddress?.address ?? defaultAddress;
  const [address, setAddress] = useState(initialAddress);
  const [addressInput, setAddressInput] = useState(config.formatAddress(initialAddress));

  const requestIdRef = useRef(0);
  const wheelRemainder = useRef(0);
  const pendingScrollDelta = useRef(0);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const targetAddressRef = useRef<number | undefined>(undefined);
  const targetTimestampRef = useRef<number | undefined>(undefined);
  const lineRefsMap = useRef<Map<number, HTMLDivElement>>(new Map());
  const urlHighlightTriggeredRef = useRef(false);

  const fetchSlice = useCallback(
    async (addr: number) => {
      const requestId = ++requestIdRef.current;
      setLoading(true);
      setError(undefined);
      try {
        // Calculate how many bytes can fit before hitting max address
        const remainingAddressSpace = 0xffffffff - addr;
        const adjustedLength = Math.min(config.length, Math.max(1, remainingAddressSpace + 1));

        const result = await callbacks.onFetchMemory(addr, adjustedLength, encoding, wordSize);
        if (requestIdRef.current !== requestId) {
          return;
        }
        setSlice(result);
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
    [callbacks, config.length, encoding, wordSize],
  );

  useEffect(() => {
    const normalized = clampAddress(address, config.maxAddress, BYTES_PER_ROW);
    // Display the address 10 rows down (the intended target), not the fetch start
    const displayAddress = Math.min(0xffffffff - (BYTES_PER_ROW - 1), normalized + BYTES_PER_ROW * 10);
    setAddressInput(config.formatAddress(displayAddress));

    if (initialized) {
      void fetchSlice(normalized);
    }
  }, [address, fetchSlice, config, initialized]);

  // Trigger highlight effect when loaded from URL
  useEffect(() => {
    if (!initialUrlAddress?.fromUrl || !initialized || !slice || urlHighlightTriggeredRef.current) {
      return;
    }

    // Mark as triggered immediately to prevent re-triggering on scroll
    urlHighlightTriggeredRef.current = true;

    // Align the address to row boundary for proper highlighting
    const alignedAddress = clampAddress(initialUrlAddress.address, 0xffffffff - (BYTES_PER_ROW - 1), BYTES_PER_ROW);

    // Set target address for animation trigger
    targetAddressRef.current = alignedAddress;
    targetTimestampRef.current = Date.now();

    // Update DOM when rows are available
    setTimeout(() => {
      const element = lineRefsMap.current.get(alignedAddress);
      if (element) {
        element.classList.remove("target-address");
        // Force reflow to restart animation
        void element.offsetWidth;
        element.classList.add("target-address");
      }
    }, 0);
  }, [initialUrlAddress, initialized, slice]);

  // Process pending scroll delta after loading completes
  useEffect(() => {
    if (!loading && pendingScrollDelta.current !== 0) {
      const delta = pendingScrollDelta.current;
      pendingScrollDelta.current = 0;
      setAddress((prev) => clampAddress(prev + delta, config.maxAddress, BYTES_PER_ROW));
    }
  }, [loading, config.maxAddress]);

  const rows = useMemo<MemoryRow[]>(() => {
    if (!slice) {
      return [];
    }
    const bytes = slice.data.match(/.{1,2}/g) ?? [];
    const result: MemoryRow[] = [];
    for (let offset = 0; offset < bytes.length; offset += BYTES_PER_ROW) {
      const chunk = bytes.slice(offset, offset + BYTES_PER_ROW);
      const rowAddress = slice.baseAddress + offset;
      const hex = chunk.join(" ");
      const ascii = chunk
        .map((byte) => {
          const charCode = Number.parseInt(byte, 16);
          const char = String.fromCharCode(charCode);
          return /[\x20-\x7E]/.test(char) ? char : ".";
        })
        .join("");
      result.push({ id: offset, address: rowAddress, hex, ascii });
    }
    return result;
  }, [slice]);

  const adjustAddress = useCallback(
    (delta: number) => {
      setAddress((prev) => {
        const newAddr = clampAddress(prev + delta, config.maxAddress, BYTES_PER_ROW);
        callbacks.onAddressChange?.(newAddr);
        return newAddr;
      });
    },
    [config.maxAddress, callbacks],
  );

  const handleWheel = useCallback(
    (event: WheelEvent) => {
      event.preventDefault();
      wheelRemainder.current += event.deltaY;

      while (Math.abs(wheelRemainder.current) >= WHEEL_PIXEL_THRESHOLD) {
        const direction = wheelRemainder.current > 0 ? 1 : -1;
        const delta = direction * MEMORY_SCROLL_BYTES;

        if (loading) {
          // Queue the scroll while loading
          pendingScrollDelta.current += delta;
        } else {
          adjustAddress(delta);
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
    const parsed = config.parseAddress(addressInput);
    if (parsed === undefined) {
      return;
    }
    // Align down to row boundary
    const alignedAddress = clampAddress(parsed, 0xffffffff - (BYTES_PER_ROW - 1), BYTES_PER_ROW);

    // Update the input field to show the aligned address
    setAddressInput(config.formatAddress(alignedAddress));

    // Start 10 rows before for context
    const contextOffset = BYTES_PER_ROW * 10;
    const targetAddress = Math.max(0, alignedAddress - contextOffset);
    const clampedTarget = clampAddress(targetAddress, config.maxAddress, BYTES_PER_ROW);

    setAddress(clampedTarget);
    callbacks.onAddressChange?.(alignedAddress);

    // Set target address (aligned) and timestamp for animation trigger
    targetAddressRef.current = alignedAddress;
    targetTimestampRef.current = Date.now();

    // Update DOM when rows are available
    setTimeout(() => {
      const element = lineRefsMap.current.get(alignedAddress);
      if (element) {
        element.classList.remove("target-address");
        // Force reflow to restart animation
        void element.offsetWidth;
        element.classList.add("target-address");
      }
    }, 0);
  }, [addressInput, config, callbacks]);

  const handleRefresh = useCallback(() => {
    void fetchSlice(address);
  }, [fetchSlice, address]);

  return (
    <Box
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
        <Stack direction="row" spacing={1} alignItems="center" sx={{ flex: 1 }}>
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
            sx={{ flex: 1 }}
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
        ) : loading && !slice ? (
          <Stack alignItems="center" justifyContent="center" sx={{ height: "100%" }} spacing={1}>
            <CircularProgress size={18} />
            <Typography variant="body2" color="text.secondary">
              Loading memory.
            </Typography>
          </Stack>
        ) : slice && rows.length > 0 ? (
          <Box
            ref={containerRef}
            sx={{
              height: "100%",
              flex: 1,
              overflow: "hidden",
              fontFamily: "monospace",
              fontSize: 13,
              p: 1.5,
              position: "relative",
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
            <Box sx={{ display: "flex", mb: 1, color: "text.secondary", fontWeight: 600, fontFamily: "monospace", fontSize: 13 }}>
              <Typography component="span" sx={{ width: 100, flexShrink: 0, mr: "1.5em" }}>
                Address
              </Typography>
              <Typography component="span" sx={{ width: 380, flexShrink: 0, mr: "4em" }}>
                Hex
              </Typography>
              <Typography component="span" sx={{ width: 130, flexShrink: 0 }}>
                ASCII
              </Typography>
            </Box>
            <Stack
              spacing={0}
              sx={{
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
              {rows.map((row) => (
                <Box
                  key={row.id}
                  ref={(el: HTMLDivElement | null) => {
                    if (el) {
                      lineRefsMap.current.set(row.address, el);
                    } else {
                      lineRefsMap.current.delete(row.address);
                    }
                  }}
                  sx={{
                    display: "flex",
                    fontFamily: "monospace",
                    fontSize: 13,
                    alignItems: "baseline",
                    border: "2px solid transparent",
                    borderRadius: 1,
                    px: 0.5,
                    py: 0,
                    "&:hover": {
                      borderBottomColor: "primary.main",
                    },
                  }}
                >
                  <Typography component="span" sx={{ width: 100, flexShrink: 0, mr: "1.5em" }}>
                    {config.formatAddress(row.address)}
                  </Typography>
                  <Typography component="span" sx={{ width: 380, flexShrink: 0, whiteSpace: "nowrap", letterSpacing: 0, mr: "4em" }}>
                    {row.hex}
                  </Typography>
                  <Typography component="span" sx={{ width: 130, flexShrink: 0, whiteSpace: "pre", letterSpacing: "0.05em" }}>
                    {row.ascii}
                  </Typography>
                </Box>
              ))}
            </Stack>
          </Box>
        ) : (
          <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
            Memory slice unavailable.
          </Typography>
        )}
      </Box>
    </Box>
  );
};
