import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { WheelEvent as ReactWheelEvent } from "react";
import { Panel } from "../layout/Panel";
import { Box, Button, CircularProgress, IconButton, Stack, TextField, Typography } from "@mui/material";
import ArrowUpwardIcon from "@mui/icons-material/ArrowUpward";
import ArrowDownwardIcon from "@mui/icons-material/ArrowDownward";
import type { DisassemblyLine } from "../../lib/debuggerSchema";
import { useSessionStore } from "../../state/sessionStore";

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
  title,
  target,
  defaultAddress,
}: {
  title: string;
  target: string;
  defaultAddress: number;
}) => {
  const client = useSessionStore((state) => state.client);
  const connectionState = useSessionStore((state) => state.connectionState);
  const [lines, setLines] = useState<DisassemblyLine[]>([]);
  const [loading, setLoading] = useState(false);
  const [address, setAddress] = useState(defaultAddress);
  const [addressInput, setAddressInput] = useState(formatAddressInput(target, defaultAddress));
  const [error, setError] = useState<string | undefined>();
  const requestIdRef = useRef(0);
  const wheelRemainder = useRef(0);

  const instructionSize = useMemo(() => instructionSizeForTarget(target), [target]);
  const maxAddress = useMemo(() => maxAddressForTarget(target), [target]);

  const fetchDisassembly = useCallback(
    async (addr: number) => {
      if (!client || connectionState !== "connected") {
        return;
      }
      const requestId = ++requestIdRef.current;
      setLoading(true);
      setError(undefined);
      try {
        const result = await client.fetchDisassembly({ target, address: addr, count: FETCH_LINE_COUNT });
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
    [client, connectionState, target],
  );

  useEffect(() => {
    const normalized = normalizeAddress(address, maxAddress, instructionSize);
    setAddressInput(formatAddressInput(target, normalized));
    void fetchDisassembly(normalized);
  }, [address, fetchDisassembly, instructionSize, maxAddress, target]);

  const adjustAddress = useCallback(
    (steps: number) => {
      setAddress((prev) => normalizeAddress(prev + steps * instructionSize, maxAddress, instructionSize));
    },
    [instructionSize, maxAddress],
  );

  const handleWheel = useCallback(
    (event: ReactWheelEvent<HTMLDivElement>) => {
      event.preventDefault();
      wheelRemainder.current += event.deltaY;

      while (Math.abs(wheelRemainder.current) >= WHEEL_PIXEL_THRESHOLD) {
        const direction = wheelRemainder.current > 0 ? 1 : -1;
        adjustAddress(direction * INSTRUCTIONS_PER_TICK);
        wheelRemainder.current -= direction * WHEEL_PIXEL_THRESHOLD;
      }
    },
    [adjustAddress],
  );

  const handleAddressSubmit = useCallback(() => {
    const parsed = parseAddressInput(target, addressInput);
    if (parsed === undefined) {
      return;
    }
    setAddress(normalizeAddress(parsed, maxAddress, instructionSize));
  }, [addressInput, instructionSize, maxAddress, target]);

  const handlePageUp = useCallback(() => {
    adjustAddress(-FETCH_LINE_COUNT);
  }, [adjustAddress]);

  const handlePageDown = useCallback(() => {
    adjustAddress(FETCH_LINE_COUNT);
  }, [adjustAddress]);

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
            sx={{ width: 160 }}
            disabled={connectionState !== "connected"}
          />
          <Button size="small" onClick={handleAddressSubmit} disabled={connectionState !== "connected"}>
            Go
          </Button>
          <IconButton size="small" onClick={handlePageUp} disabled={connectionState !== "connected"}>
            <ArrowUpwardIcon fontSize="small" />
          </IconButton>
          <IconButton size="small" onClick={handlePageDown} disabled={connectionState !== "connected"}>
            <ArrowDownwardIcon fontSize="small" />
          </IconButton>
        </Stack>
      }
    >
      {connectionState !== "connected" ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          Connect to view disassembly.
        </Typography>
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
          onWheel={handleWheel}
          sx={{
            flex: 1,
            height: "100%",
            overflow: "hidden",
            fontFamily: "monospace",
            fontSize: "13px",
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
          {lines.length === 0 ? (
            <Typography variant="body2" color="text.secondary">
              No disassembly returned.
            </Typography>
          ) : (
            <Stack spacing={0.25}>
              {lines.map((line) => {
                const commentText = line.comment ? `; ${line.comment}` : "";
                const mnemonicSegment = line.operands ? `${line.mnemonic} ${line.operands}` : line.mnemonic;
                return (
                  <Box
                    key={`${line.address}-${line.mnemonic}-${line.operands}`}
                    sx={{
                      display: "grid",
                      gridTemplateColumns: target === "dsp" ? "80px 120px 1fr" : "140px 140px 1fr",
                      gap: 1,
                      alignItems: "center",
                      color: line.isCurrent ? "primary.main" : "inherit",
                      fontWeight: line.isCurrent ? 600 : 400,
                      borderLeft: line.isBreakpoint ? "2px solid var(--mui-palette-warning-main)" : "2px solid transparent",
                      px: 0.5,
                      py: 0.25,
                    }}
                  >
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
