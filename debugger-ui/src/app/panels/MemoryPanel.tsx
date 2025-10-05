import { Panel } from "../layout/Panel";
import { DataGrid } from "@mui/x-data-grid";
import type { GridColDef } from "@mui/x-data-grid";

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
    valueFormatter: ({ value }) => `0x${Number(value).toString(16).padStart(8, "0")}`,
  },
  { field: "hex", headerName: "Hex", flex: 2 },
  { field: "ascii", headerName: "ASCII", flex: 1 },
];

const rows: MemoryRow[] = Array.from({ length: 16 }).map((_, index) => ({
  id: index,
  address: 0x8c000000 + index * 16,
  hex: "DE AD BE EF 00 11 22 33",
  ascii: "....\u0000..",
}));

export const MemoryPanel = () => {
  return (
    <Panel title="Memory Viewer">
      <DataGrid
        density="compact"
        disableColumnMenu
        hideFooter
        rows={rows}
        columns={columns}
        sx={{ border: "none" }}
      />
    </Panel>
  );
};
