import { useCallback, useEffect, useMemo, useState, useRef } from "react";
import { Panel } from "../layout/Panel";
import { Box, Button, CircularProgress, IconButton, Stack, TextField, Typography } from "@mui/material";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import type { DisassemblyLine } from "../../lib/debuggerSchema";
import { useSessionStore } from "../../state/sessionStore";

type DisassemblyConfig = {
  title: string;
  target: string;
  defaultAddress: number;
};

const VISIBLE_ROWS = 30;
const BUFFER_ROWS = 128;
const TOTAL_VIRTUAL_ROWS = 1000000;
const ROW_HEIGHT = 24;

const DisassemblyView = ({ title, target, defaultAddress }: DisassemblyConfig) => {
  const client = useSessionStore((state) => state.client);
  const connectionState = useSessionStore((state) => state.connectionState);
  const [disasmCache, setDisasmCache] = useState<Map<number, DisassemblyLine>>(new Map());
  const [loading, setLoading] = useState(false);
  const [scrollOffset, setScrollOffset] = useState(0);
  const [baseAddress, setBaseAddress] = useState(defaultAddress);
  const [addressInput, setAddressInput] = useState(`0x${defaultAddress.toString(16).toUpperCase()}`);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const fetchingRef = useRef(false);

  const fetchDisassembly = useCallback(async (address: number, count: number) => {
    if (!client || connectionState !== "connected" || fetchingRef.current) {
      return;
    }

    fetchingRef.current = true;
    setLoading(true);
    try {
      const result = await client.fetchDisassembly({ target, address, count });
      setDisasmCache(prev => {
        const newCache = new Map(prev);
        result.lines.forEach(line => {
          newCache.set(line.address, line);
        });
        return newCache;
      });
    } catch (error) {
      console.error(`Failed to fetch ${target} disassembly`, error);
    } finally {
      setLoading(false);
      fetchingRef.current = false;
    }
  }, [client, connectionState, target]);

  useEffect(() => {
    void fetchDisassembly(defaultAddress, BUFFER_ROWS);
  }, [fetchDisassembly, defaultAddress]);

  const currentVirtualRow = useMemo(() => {
    return Math.floor(scrollOffset / ROW_HEIGHT);
  }, [scrollOffset]);

  const visibleLines = useMemo<DisassemblyLine[]>(() => {
    const startRow = Math.max(0, currentVirtualRow - 10);
    const endRow = Math.min(TOTAL_VIRTUAL_ROWS, currentVirtualRow + VISIBLE_ROWS + 10);

    const result: DisassemblyLine[] = [];
    for (let rowIndex = startRow; rowIndex < endRow; rowIndex++) {
      const address = baseAddress + rowIndex * 2; // Assuming 2-byte instructions on average
      const cachedLine = disasmCache.get(address);

      if (cachedLine) {
        result.push(cachedLine);
      } else {
        // Placeholder for uncached lines
        result.push({
          address,
          bytes: "??",
          mnemonic: "...",
          operands: "",
          isCurrent: false,
        });
      }
    }
    return result;
  }, [baseAddress, disasmCache, currentVirtualRow]);

  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    const { scrollTop } = e.currentTarget;
    setScrollOffset(scrollTop);

    const currentRow = Math.floor(scrollTop / ROW_HEIGHT);
    const currentAddress = baseAddress + currentRow * 2;

    // Check if we need to fetch more data
    const cachedAddresses = Array.from(disasmCache.keys());
    if (cachedAddresses.length === 0) return;

    const minCachedAddr = Math.min(...cachedAddresses);
    const maxCachedAddr = Math.max(...cachedAddresses);

    if (currentAddress < minCachedAddr + 40) {
      const fetchAddr = Math.max(0, minCachedAddr - BUFFER_ROWS * 2);
      void fetchDisassembly(fetchAddr, BUFFER_ROWS);
    } else if (currentAddress > maxCachedAddr - 40) {
      const fetchAddr = maxCachedAddr + 2;
      void fetchDisassembly(fetchAddr, BUFFER_ROWS);
    }
  }, [baseAddress, disasmCache, fetchDisassembly]);

  const handleAddressChange = useCallback(() => {
    try {
      const parsed = Number.parseInt(addressInput.replace(/^0x/i, ""), 16);
      if (!Number.isNaN(parsed)) {
        setBaseAddress(parsed);
        setDisasmCache(new Map());
        setScrollOffset(0);
        if (scrollContainerRef.current) {
          scrollContainerRef.current.scrollTop = 0;
        }
        void fetchDisassembly(parsed, BUFFER_ROWS);
        setAddressInput(`0x${parsed.toString(16).toUpperCase()}`);
      }
    } catch {
      // Invalid input, ignore
    }
  }, [addressInput, fetchDisassembly]);

  const handlePageUp = useCallback(() => {
    if (scrollContainerRef.current) {
      scrollContainerRef.current.scrollTop = Math.max(0, scrollContainerRef.current.scrollTop - VISIBLE_ROWS * ROW_HEIGHT);
    }
  }, []);

  const handlePageDown = useCallback(() => {
    if (scrollContainerRef.current) {
      scrollContainerRef.current.scrollTop += VISIBLE_ROWS * ROW_HEIGHT;
    }
  }, []);

  return (
    <Panel
      title={title}
      action={
        <Stack direction="row" spacing={1} alignItems="center">
          <TextField
            size="small"
            value={addressInput}
            onChange={(e) => setAddressInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                handleAddressChange();
              }
            }}
            placeholder="0x00000000"
            sx={{ width: 140 }}
            disabled={loading || connectionState !== "connected"}
          />
          <Button size="small" onClick={handleAddressChange} disabled={loading || connectionState !== "connected"}>
            Go
          </Button>
          <IconButton size="small" onClick={handlePageUp} disabled={loading || connectionState !== "connected"}>
            <ArrowUpwardIcon fontSize="small" />
          </IconButton>
          <IconButton size="small" onClick={handlePageDown} disabled={loading || connectionState !== "connected"}>
            <ArrowDownwardIcon fontSize="small" />
          </IconButton>
        </Stack>
      }
    >
      {disasmCache.size === 0 && loading ? (
        <Stack alignItems="center" justifyContent="center" sx={{ height: "100%" }} spacing={1}>
          <CircularProgress size={18} />
          <Typography variant="body2" color="text.secondary">
            Loading disassembly…
          </Typography>
        </Stack>
      ) : (
        <Box
          ref={scrollContainerRef}
          onScroll={handleScroll}
          sx={{
            flex: 1,
            overflow: "auto",
            fontFamily: "monospace",
            fontSize: "13px",
            height: "100%",
            position: "relative",
            "&::-webkit-scrollbar": {
              width: 0,
              height: 0,
            },
            scrollbarWidth: "none",
            msOverflowStyle: "none",
          }}
        >
          <Box sx={{ height: TOTAL_VIRTUAL_ROWS * ROW_HEIGHT, position: "relative" }}>
            <Box
              sx={{
                position: "absolute",
                top: Math.max(0, currentVirtualRow - 10) * ROW_HEIGHT,
                left: 0,
                right: 0,
              }}
            >
              <Box component="pre" sx={{ m: 0, p: 1.5 }}>
                {visibleLines.map((line) => (
                  <Typography
                    key={line.address}
                    component="div"
                    sx={{
                      display: "flex",
                      gap: 2,
                      color: line.isCurrent ? "primary.main" : "inherit",
                      height: ROW_HEIGHT,
                      lineHeight: `${ROW_HEIGHT}px`,
                    }}
                  >
                    <span>{`0x${line.address.toString(16).toUpperCase().padStart(8, "0")}`}</span>
                    <span>{line.bytes.padEnd(11, " ")}</span>
                    <span>{line.mnemonic.padEnd(8, " ")}</span>
                    <span>{line.operands}</span>
                    {line.comment && (
                      <span style={{ color: "var(--mui-palette-text-secondary)" }}>; {line.comment}</span>
                    )}
                  </Typography>
                ))}
              </Box>
            </Box>
          </Box>
        </Box>
      )}
    </Panel>
  );
};

export const Sh4DisassemblyPanel = () => (
  <DisassemblyView title="SH4: Disassembly" target="sh4" defaultAddress={0x8c0000a0} />
);

export const Arm7DisassemblyPanel = () => (
  <DisassemblyView title="ARM7: Disassembly" target="arm7" defaultAddress={0x00200000} />
);

export const DspDisassemblyPanel = () => (
  <DisassemblyView title="DSP: Disassembly" target="dsp" defaultAddress={0x00000000} />
);
