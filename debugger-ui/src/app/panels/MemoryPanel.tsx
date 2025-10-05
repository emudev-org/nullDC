import { useCallback, useEffect, useMemo, useState } from "react";
import { Panel } from "../layout/Panel";
import { DataGrid } from "@mui/x-data-grid";
import type { GridColDef } from "@mui/x-data-grid";
import { Button, CircularProgress, Stack, Typography } from "@mui/material";
import type { MemorySlice } from "../../lib/debuggerSchema";
import { useSessionStore } from "../../state/sessionStore";

type MemoryRow = {
  id: number;
  address: number;
  hex: string;
  ascii: string;
};

const columns: GridColDef<MemoryRow>[] = [
  {
    field: "address",
    headerName: "Address",
    flex: 1,
    valueFormatter: ({ value }) => `0x${Number(value).toString(16).toUpperCase().padStart(8, "0")}`,
  },
  { field: "hex", headerName: "Hex", flex: 2 },
  { field: "ascii", headerName: "ASCII", flex: 1 },
];

const DEFAULT_ADDRESS = 0x8c000000;
const DEFAULT_LENGTH = 256;

export const MemoryPanel = () => {
  const client = useSessionStore((state) => state.client);
  const connectionState = useSessionStore((state) => state.connectionState);
  const [slice, setSlice] = useState<MemorySlice | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchSlice = useCallback(async () => {
    if (!client || connectionState !== "connected") {
      return;
    }
    setLoading(true);
    try {
      const result = await client.fetchMemorySlice(DEFAULT_ADDRESS, DEFAULT_LENGTH);
      setSlice(result);
    } catch (error) {
      console.error("Failed to fetch memory slice", error);
    } finally {
      setLoading(false);
    }
  }, [client, connectionState]);

  useEffect(() => {
    void fetchSlice();
  }, [fetchSlice]);

  const rows = useMemo<MemoryRow[]>(() => {
    if (!slice) {
      return [];
    }
    const bytes = slice.data.match(/.{1,2}/g) ?? [];
    const step = 16;
    const result: MemoryRow[] = [];
    for (let offset = 0; offset < bytes.length; offset += step) {
      const chunk = bytes.slice(offset, offset + step);
      const address = slice.baseAddress + offset;
      const hex = chunk.join(" ");
      const ascii = chunk
        .map((byte) => {
          const charCode = Number.parseInt(byte, 16);
          const char = String.fromCharCode(charCode);
          return /[\x20-\x7E]/.test(char) ? char : ".";
        })
        .join("");
      result.push({ id: offset, address, hex, ascii });
    }
    return result;
  }, [slice]);

  return (
    <Panel
      title="Memory Viewer"
      action={
        <Button size="small" onClick={() => void fetchSlice()} disabled={loading || connectionState !== "connected"}>
          Refresh
        </Button>
      }
    >
      {loading && !slice ? (
        <Stack alignItems="center" justifyContent="center" sx={{ height: "100%" }} spacing={1}>
          <CircularProgress size={18} />
          <Typography variant="body2" color="text.secondary">
            Loading memory...
          </Typography>
        </Stack>
      ) : slice && rows.length > 0 ? (
        <DataGrid
          density="compact"
          disableColumnMenu
          hideFooter
          rows={rows}
          columns={columns}
          sx={{ border: "none", flex: 1 }}
        />
      ) : (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          Memory slice unavailable.
        </Typography>
      )}
    </Panel>
  );
};
