import { useCallback, useEffect, useMemo, useState, useRef } from "react";
import { Box, Button, CircularProgress, IconButton, Stack, TextField, Typography } from "@mui/material";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import type { MemorySlice } from "../../lib/debuggerSchema";
import { Panel } from "../layout/Panel";
import { useSessionStore } from "../../state/sessionStore";

type MemoryRow = {
  address: number;
  hex: string;
  ascii: string;
};

type MemoryViewConfig = {
  title: string;
  target: string;
  defaultAddress: number;
  encoding?: MemorySlice["encoding"];
  wordSize?: MemorySlice["wordSize"];
};

const BYTES_PER_ROW = 16;
const VISIBLE_ROWS = 30;
const BUFFER_ROWS = 100;
const TOTAL_VIRTUAL_ROWS = 1000000;
const ROW_HEIGHT = 28;

const MemoryView = ({ title, target, defaultAddress, encoding, wordSize }: MemoryViewConfig) => {
  const client = useSessionStore((state) => state.client);
  const connectionState = useSessionStore((state) => state.connectionState);
  const [memoryCache, setMemoryCache] = useState<Map<number, number>>(new Map());
  const [loading, setLoading] = useState(false);
  const [scrollOffset, setScrollOffset] = useState(0);
  const [baseAddress, setBaseAddress] = useState(defaultAddress);
  const [addressInput, setAddressInput] = useState(`0x${defaultAddress.toString(16).toUpperCase()}`);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const fetchingRef = useRef(false);

  const fetchMemoryPage = useCallback(async (address: number, length: number) => {
    if (!client || connectionState !== "connected" || fetchingRef.current) {
      return;
    }

    fetchingRef.current = true;
    setLoading(true);
    try {
      const result = await client.fetchMemorySlice({
        target,
        address,
        length,
        encoding,
        wordSize,
      });

      const bytes = result.data.match(/.{1,2}/g) ?? [];
      setMemoryCache(prev => {
        const newCache = new Map(prev);
        bytes.forEach((byte, index) => {
          newCache.set(result.baseAddress + index, Number.parseInt(byte, 16));
        });
        return newCache;
      });
    } catch (error) {
      console.error(`Failed to fetch ${target} memory`, error);
    } finally {
      setLoading(false);
      fetchingRef.current = false;
    }
  }, [client, connectionState, target, encoding, wordSize]);

  useEffect(() => {
    void fetchMemoryPage(defaultAddress, BUFFER_ROWS * BYTES_PER_ROW);
  }, [fetchMemoryPage, defaultAddress]);

  const currentVirtualRow = useMemo(() => {
    return Math.floor(scrollOffset / ROW_HEIGHT);
  }, [scrollOffset]);

  const visibleRows = useMemo<MemoryRow[]>(() => {
    const startRow = Math.max(0, currentVirtualRow - 10);
    const endRow = Math.min(TOTAL_VIRTUAL_ROWS, currentVirtualRow + VISIBLE_ROWS + 10);

    const result: MemoryRow[] = [];
    for (let rowIndex = startRow; rowIndex < endRow; rowIndex++) {
      const address = baseAddress + rowIndex * BYTES_PER_ROW;
      const rowBytes: number[] = [];

      for (let i = 0; i < BYTES_PER_ROW; i++) {
        const byteAddr = address + i;
        const byte = memoryCache.get(byteAddr);
        rowBytes.push(byte ?? 0);
      }

      const hex = rowBytes.map(b => b.toString(16).toUpperCase().padStart(2, "0")).join(" ");
      const ascii = rowBytes
        .map(b => {
          const char = String.fromCharCode(b);
          return /[\x20-\x7E]/.test(char) ? char : ".";
        })
        .join("");

      result.push({ address, hex, ascii });
    }
    return result;
  }, [baseAddress, memoryCache, currentVirtualRow]);

  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    const { scrollTop } = e.currentTarget;
    setScrollOffset(scrollTop);

    const currentRow = Math.floor(scrollTop / ROW_HEIGHT);
    const currentAddress = baseAddress + currentRow * BYTES_PER_ROW;

    // Check if we need to fetch more data
    const minCachedAddr = Math.min(...Array.from(memoryCache.keys()));
    const maxCachedAddr = Math.max(...Array.from(memoryCache.keys()));

    if (currentAddress < minCachedAddr + BYTES_PER_ROW * 20) {
      const fetchAddr = Math.max(0, minCachedAddr - BUFFER_ROWS * BYTES_PER_ROW);
      void fetchMemoryPage(fetchAddr, BUFFER_ROWS * BYTES_PER_ROW);
    } else if (currentAddress > maxCachedAddr - BYTES_PER_ROW * 20) {
      const fetchAddr = maxCachedAddr + 1;
      void fetchMemoryPage(fetchAddr, BUFFER_ROWS * BYTES_PER_ROW);
    }
  }, [baseAddress, memoryCache, fetchMemoryPage]);


  const handleAddressChange = useCallback(() => {
    try {
      const parsed = Number.parseInt(addressInput.replace(/^0x/i, ""), 16);
      if (!Number.isNaN(parsed)) {
        setBaseAddress(parsed);
        setMemoryCache(new Map());
        setScrollOffset(0);
        if (scrollContainerRef.current) {
          scrollContainerRef.current.scrollTop = 0;
        }
        void fetchMemoryPage(parsed, BUFFER_ROWS * BYTES_PER_ROW);
        setAddressInput(`0x${parsed.toString(16).toUpperCase()}`);
      }
    } catch {
      // Invalid input, ignore
    }
  }, [addressInput, fetchMemoryPage]);

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
      {memoryCache.size === 0 && loading ? (
        <Stack alignItems="center" justifyContent="center" sx={{ height: "100%" }} spacing={1}>
          <CircularProgress size={18} />
          <Typography variant="body2" color="text.secondary">
            Loading memory…
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
              <Box component="table" sx={{ width: "100%", borderCollapse: "collapse" }}>
                <Box component="thead" sx={{ position: "sticky", top: 0, bgcolor: "background.paper", zIndex: 1 }}>
                  <Box component="tr">
                    <Box component="th" sx={{ textAlign: "left", p: 1, borderBottom: 1, borderColor: "divider" }}>
                      Address
                    </Box>
                    <Box component="th" sx={{ textAlign: "left", p: 1, borderBottom: 1, borderColor: "divider" }}>
                      Hex
                    </Box>
                    <Box component="th" sx={{ textAlign: "left", p: 1, borderBottom: 1, borderColor: "divider" }}>
                      ASCII
                    </Box>
                  </Box>
                </Box>
                <Box component="tbody">
                  {visibleRows.map((row) => (
                    <Box component="tr" key={row.address} sx={{ height: ROW_HEIGHT }}>
                      <Box component="td" sx={{ p: 1, borderBottom: 1, borderColor: "divider", whiteSpace: "nowrap" }}>
                        {`0x${row.address.toString(16).toUpperCase().padStart(8, "0")}`}
                      </Box>
                      <Box component="td" sx={{ p: 1, borderBottom: 1, borderColor: "divider", whiteSpace: "nowrap" }}>
                        {row.hex}
                      </Box>
                      <Box component="td" sx={{ p: 1, borderBottom: 1, borderColor: "divider", whiteSpace: "nowrap" }}>
                        {row.ascii}
                      </Box>
                    </Box>
                  ))}
                </Box>
              </Box>
            </Box>
          </Box>
        </Box>
      )}
    </Panel>
  );
};

export const Sh4MemoryPanel = () => (
  <MemoryView title="SH4: Memory" target="sh4" defaultAddress={0x8c000000} />
);

export const Arm7MemoryPanel = () => (
  <MemoryView title="ARM7: Memory" target="arm7" defaultAddress={0x00200000} />
);
