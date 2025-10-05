import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Panel } from "../layout/Panel";
import { Box, Button, CircularProgress, Stack, TextField, Typography } from "@mui/material";
import type { MemorySlice } from "../../lib/debuggerSchema";
import { useSessionStore } from "../../state/sessionStore";

type MemoryRow = {
  id: number;
  address: number;
  hex: string;
  ascii: string;
};

const formatHexAddress = (value: number) => `0x${value.toString(16).toUpperCase().padStart(8, "0")}`;

const clampAddress = (value: number, max: number) => {
  if (value < 0) {
    return 0;
  }
  if (value > max) {
    return max;
  }
  return value;
};

const WHEEL_PIXEL_THRESHOLD = 60;
const MEMORY_SCROLL_BYTES = 96;
const BYTES_PER_ROW = 16;
const VISIBLE_ROWS = 60;

const MemoryView = ({
  title,
  target,
  defaultAddress,
  length,
  encoding,
  wordSize,
}: {
  title: string;
  target: string;
  defaultAddress: number;
  length: number;
  encoding?: MemorySlice["encoding"];
  wordSize?: MemorySlice["wordSize"];
}) => {
  const client = useSessionStore((state) => state.client);
  const connectionState = useSessionStore((state) => state.connectionState);
  const [slice, setSlice] = useState<MemorySlice | null>(null);
  const [loading, setLoading] = useState(false);
  const [address, setAddress] = useState(defaultAddress);
  const [addressInput, setAddressInput] = useState(formatHexAddress(defaultAddress));
  const requestIdRef = useRef(0);
  const wheelRemainder = useRef(0);

  const maxAddress = useMemo(() => 0xffffffff - Math.max(length - 1, 0), [length]);

  const fetchSlice = useCallback(
    async (addr: number) => {
      if (!client || connectionState !== "connected") {
        return;
      }

      const requestId = ++requestIdRef.current;
      setLoading(true);
      try {
        const result = await client.fetchMemorySlice({
          target,
          address: addr,
          length,
          encoding,
          wordSize,
        });
        if (requestIdRef.current !== requestId) {
          return;
        }
        setSlice(result);
      } catch (error) {
        console.error(`Failed to fetch ${target} memory`, error);
      } finally {
        if (requestIdRef.current === requestId) {
          setLoading(false);
        }
      }
    },
    [client, connectionState, target, length, encoding, wordSize],
  );

  useEffect(() => {
    setAddressInput(formatHexAddress(address));
    void fetchSlice(address);
  }, [address, fetchSlice]);

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
      setAddress((prev) => clampAddress(prev + delta, maxAddress));
    },
    [maxAddress],
  );

  const handleWheel = useCallback(
    (event: React.WheelEvent<HTMLDivElement>) => {
      event.preventDefault();
      wheelRemainder.current += event.deltaY;

      while (Math.abs(wheelRemainder.current) >= WHEEL_PIXEL_THRESHOLD) {
        const direction = wheelRemainder.current > 0 ? 1 : -1;
        adjustAddress(direction * MEMORY_SCROLL_BYTES);
        wheelRemainder.current -= direction * WHEEL_PIXEL_THRESHOLD;
      }
    },
    [adjustAddress],
  );

  const handleAddressSubmit = useCallback(() => {
    const normalized = addressInput.trim();
    const parsed = Number.parseInt(normalized.replace(/^0x/i, ""), 16);
    if (Number.isNaN(parsed)) {
      return;
    }
    setAddress(clampAddress(parsed, maxAddress));
  }, [addressInput, maxAddress]);

  const handleRefresh = useCallback(() => {
    void fetchSlice(address);
  }, [fetchSlice, address]);

  return (
    <Panel
      title={title}
      action={
        <Stack direction="row" spacing={1} alignItems="center">
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
            sx={{ width: 140 }}
            disabled={connectionState !== "connected"}
          />
          <Button size="small" onClick={handleAddressSubmit} disabled={connectionState !== "connected"}>
            Go
          </Button>
          <Button size="small" onClick={handleRefresh} disabled={loading || connectionState !== "connected"}>
            Refresh
          </Button>
        </Stack>
      }
    >
      {loading && !slice ? (
        <Stack alignItems="center" justifyContent="center" sx={{ height: "100%" }} spacing={1}>
          <CircularProgress size={18} />
          <Typography variant="body2" color="text.secondary">
            Loading memory.
          </Typography>
        </Stack>
      ) : slice && rows.length > 0 ? (
        <Box
          onWheel={handleWheel}
          sx={{
            height: "100%",
            flex: 1,
            overflow: "hidden",
            fontFamily: "monospace",
            fontSize: 13,
            p: 1.5,
            position: "relative",
            cursor: "ns-resize",
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
            <Typography component="span" sx={{ width: 100, flexShrink: 0, mr: "1.5em" }}>Address</Typography>
            <Typography component="span" sx={{ width: 380, flexShrink: 0, mr: "4em" }}>Hex</Typography>
            <Typography component="span" sx={{ width: 130, flexShrink: 0 }}>ASCII</Typography>
          </Box>
          <Stack spacing={0.5}>
            {rows.map((row) => (
              <Box
                key={`${target}-${row.id}`}
                sx={{ display: "flex", fontFamily: "monospace", fontSize: 13, alignItems: "baseline" }}>
                <Typography component="span" sx={{ width: 100, flexShrink: 0, mr: "1.5em" }}>{formatHexAddress(row.address)}</Typography>
                <Typography component="span" sx={{ width: 380, flexShrink: 0, whiteSpace: "nowrap", letterSpacing: 0, mr: "4em" }}>{row.hex}</Typography>
                <Typography component="span" sx={{ width: 130, flexShrink: 0, whiteSpace: "pre", letterSpacing: "0.05em" }}>{row.ascii}</Typography>
              </Box>
            ))}
          </Stack>
        </Box>
      ) : (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          Memory slice unavailable.
        </Typography>
      )}
    </Panel>
  );
};

export const Sh4MemoryPanel = () => (
  <MemoryView title="SH4: Memory" target="sh4" defaultAddress={0x8c000000} length={960} />
);

export const Arm7MemoryPanel = () => (
  <MemoryView title="ARM7: Memory" target="arm7" defaultAddress={0x00200000} length={960} />
);
