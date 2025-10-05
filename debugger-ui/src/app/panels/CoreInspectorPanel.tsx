import { useState, useRef, useCallback, useMemo } from "react";
import { Panel } from "../layout/Panel";
import { Box, Typography, Button, CircularProgress } from "@mui/material";
import UploadFileIcon from "@mui/icons-material/UploadFile";

interface TileData {
  base: number;
  control: number;
  opaque: number;
  opaque_mod: number;
  trans: number;
  trans_mod: number;
  puncht: number;
  pixels: PixelWrite[][];
  x: number;
  y: number;
  global_ops: GlobalOp[];
}

interface PixelWrite {
  type: "ISP" | "TSP";
  seq: number;
  params: Record<string, any>;
  status?: string;
  writtenDepth?: number;
  readStencil?: number;
  prim?: PrimData;
  IB?: number;
  IO?: number;
  T?: Array<{ textel: number; u: number; v: number; mipLevel: number }>;
  TF?: number;
  CC?: number;
  FC?: number;
  FU?: number;
  BM?: number;
  BU?: {
    final: number;
    src_blend: number;
    dst_blend: number;
    src: number;
    dst: number;
    at: number;
  };
}

interface GlobalOp {
  type: string;
  seq: number;
}

interface PrimData {
  type: "QARR" | "STRIP" | "TARR";
  culled: boolean;
  pixels: number;
  seq: number;
  params: Record<string, any>;
}

const RENDER_MODES: Record<number, string> = {
  0: "OPAQUE",
  1: "PUNCHTHROUGH_PASS0",
  2: "PUNCHTHROUGH_PASSN",
  3: "PUNCHTHROUGH_MV",
  4: "TRANSLUCENT_AUTOSORT",
  5: "TRANSLUCENT_PRESORT",
  6: "MODIFIER",
};

const DEPTH_MODES: Record<number, string> = {
  0: "NEVER",
  1: "LESS",
  2: "EQUAL",
  3: "LESS_EQUAL",
  4: "GREATER",
  5: "NOT_EQUAL",
  6: "GREATER_EQUAL",
  7: "ALWAYS",
};

export const CoreInspectorPanel = () => {
  const [loading, setLoading] = useState(false);
  const [tiles, setTiles] = useState<TileData[][]>([]);
  const [bgtag, setBgtag] = useState<number | null>(null);
  const [selectedPixel, setSelectedPixel] = useState<{
    tileId: number;
    tileX: number;
    tileY: number;
    localX: number;
    localY: number;
  } | null>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const parselog = useCallback((logData: string) => {
    const lines = logData.split("\n");
    if (lines.length === 0) {
      alert("No log data found");
      return;
    }

    if (lines[0].trim() !== "REFSW2LOG: 0") {
      alert("Bad log file format");
      return;
    }

    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.fillStyle = "red";
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    let currentBgtag: number | null = null;
    const parsedTiles: TileData[][] = Array.from({ length: 4096 }, () => []);
    let currentTile: TileData | null = null;
    let currentIsp: PixelWrite | null = null;
    let currentTsp: PixelWrite | null = null;
    let currentObject: { base: number; object: number; prims: PrimData[] } | null = null;
    let currentPrim: PrimData | null = null;

    for (let i = 1; i < lines.length; i++) {
      const line = lines[i].trim();
      if (line === "") continue;

      const parts = line.split(": ");

      switch (parts[0]) {
        case "BGTAG": {
          currentBgtag = Number.parseInt(parts[1], 16);
          break;
        }

        case "TILE": {
          const TILE = parts[1].split(" ");
          const tile_cfg = Number.parseInt(TILE[1], 16);
          const tile_x = (tile_cfg >> 2) & 0x3f;
          const tile_y = (tile_cfg >> 8) & 0x3f;
          const tile_xy = (tile_cfg >> 2) & 0xfff;

          currentTile = {
            base: Number.parseInt(TILE[0], 16),
            control: tile_cfg,
            opaque: Number.parseInt(TILE[2], 16),
            opaque_mod: Number.parseInt(TILE[3], 16),
            trans: Number.parseInt(TILE[4], 16),
            trans_mod: Number.parseInt(TILE[5], 16),
            puncht: Number.parseInt(TILE[6], 16),
            pixels: Array.from({ length: 1024 }, () => []),
            x: tile_x,
            y: tile_y,
            global_ops: [],
          };
          parsedTiles[tile_xy].push(currentTile);
          break;
        }

        case "OPAQ":
        case "OPAQ_MOD":
        case "OP_PARAMS":
        case "PT":
        case "PT_MOD":
        case "PT_MOD_PARAMS":
        case "PT_N":
        case "PT_N_PARAMS":
        case "PT_PARAMS":
        case "TR_AS":
        case "TR_AS_N":
        case "TR_PARAMS":
        case "TR_PS":
        case "STENCIL_SUM_AND":
        case "STENCIL_SUM_OR":
        case "ZKEEP":
        case "ZCLEAR": {
          if (currentTile) {
            currentTile.global_ops.push({ type: parts[0], seq: i });
          }
          break;
        }

        case "TSP": {
          if (!currentTile) break;
          const TSP = parts[1].split(" ");
          currentTsp = {
            type: "TSP",
            seq: i,
            params: {
              index: Number.parseInt(TSP[0]),
              x: Number.parseFloat(TSP[1]),
              y: Number.parseFloat(TSP[2]),
              inVolume: Number.parseInt(TSP[3]),
              invW: Number.parseFloat(TSP[4]),
              alphaTest: Number.parseInt(TSP[5]),
              isp: Number.parseInt(TSP[6], 16),
              tsp: Number.parseInt(TSP[7], 16),
              tcw: Number.parseInt(TSP[8], 16),
              tag: Number.parseInt(TSP[9], 16),
            },
          };
          currentTile.pixels[currentTsp.params.index].push(currentTsp);
          break;
        }

        case "CC":
        case "FC":
        case "FU":
        case "IB":
        case "IO":
        case "TF":
        case "BM": {
          if (currentTsp) {
            (currentTsp as any)[parts[0]] = Number.parseInt(parts[1], 16);
          }
          break;
        }

        case "BU": {
          if (!currentTsp) break;
          const BU = parts[1].split(" ");
          currentTsp.BU = {
            final: Number.parseInt(BU[0], 16),
            src_blend: Number.parseInt(BU[1], 16),
            dst_blend: Number.parseInt(BU[2], 16),
            src: Number.parseInt(BU[3], 16),
            dst: Number.parseInt(BU[4], 16),
            at: Number.parseInt(BU[5], 10),
          };
          break;
        }

        case "T": {
          if (!currentTsp) break;
          const T = parts[1].split(" ");
          if (!currentTsp.T) {
            currentTsp.T = [];
          }
          currentTsp.T.push({
            textel: Number.parseInt(T[0], 16),
            u: Number.parseInt(T[1]),
            v: Number.parseInt(T[2]),
            mipLevel: Number.parseInt(T[3]),
          });
          break;
        }

        case "OBJECT": {
          const OBJECT = parts[1].split(" ");
          currentObject = {
            base: Number.parseInt(OBJECT[0], 16),
            object: Number.parseInt(OBJECT[1], 16),
            prims: [],
          };
          break;
        }

        case "QARR": {
          const QARR = parts[1].split(" ");
          currentPrim = {
            type: "QARR",
            culled: false,
            pixels: 0,
            seq: i,
            params: {
              tag: Number.parseInt(QARR[0], 16),
              x0: Number.parseFloat(QARR[1]),
              y0: Number.parseFloat(QARR[2]),
              z0: Number.parseFloat(QARR[3]),
              x1: Number.parseFloat(QARR[4]),
              y1: Number.parseFloat(QARR[5]),
              z1: Number.parseFloat(QARR[6]),
              x2: Number.parseFloat(QARR[7]),
              y2: Number.parseFloat(QARR[8]),
              z2: Number.parseFloat(QARR[9]),
              x3: Number.parseFloat(QARR[10]),
              y3: Number.parseFloat(QARR[11]),
              z3: Number.parseFloat(QARR[12]),
              num: Number.parseInt(QARR[13], 10),
            },
          };
          if (currentObject) currentObject.prims.push(currentPrim);
          break;
        }

        case "STRIP": {
          const STRIP = parts[1].split(" ");
          currentPrim = {
            type: "STRIP",
            culled: false,
            pixels: 0,
            seq: i,
            params: {
              tag: Number.parseInt(STRIP[0], 16),
              x0: Number.parseFloat(STRIP[1]),
              y0: Number.parseFloat(STRIP[2]),
              z0: Number.parseFloat(STRIP[3]),
              x1: Number.parseFloat(STRIP[4]),
              y1: Number.parseFloat(STRIP[5]),
              z1: Number.parseFloat(STRIP[6]),
              x2: Number.parseFloat(STRIP[7]),
              y2: Number.parseFloat(STRIP[8]),
              z2: Number.parseFloat(STRIP[9]),
              num: Number.parseInt(STRIP[10], 10),
            },
          };
          if (currentObject) currentObject.prims.push(currentPrim);
          break;
        }

        case "TARR": {
          const TARR = parts[1].split(" ");
          currentPrim = {
            type: "TARR",
            culled: false,
            pixels: 0,
            seq: i,
            params: {
              tag: Number.parseInt(TARR[0], 16),
              x0: Number.parseFloat(TARR[1]),
              y0: Number.parseFloat(TARR[2]),
              z0: Number.parseFloat(TARR[3]),
              x1: Number.parseFloat(TARR[4]),
              y1: Number.parseFloat(TARR[5]),
              z1: Number.parseFloat(TARR[6]),
              x2: Number.parseFloat(TARR[7]),
              y2: Number.parseFloat(TARR[8]),
              z2: Number.parseFloat(TARR[9]),
              num: Number.parseInt(TARR[10], 10),
            },
          };
          if (currentObject) currentObject.prims.push(currentPrim);
          break;
        }

        case "CULLED": {
          if (currentPrim) currentPrim.culled = true;
          break;
        }

        case "ISP": {
          if (!currentTile || !currentPrim) break;
          const ISP = parts[1].split(" ");
          currentIsp = {
            type: "ISP",
            seq: i,
            params: {
              index: Number.parseInt(ISP[0], 10),
              render_mode: Number.parseInt(ISP[1], 10),
              mode: Number.parseInt(ISP[2]),
              x: Number.parseFloat(ISP[3]),
              y: Number.parseFloat(ISP[4]),
              invW: Number.parseFloat(ISP[5]),
              tag: Number.parseInt(ISP[6], 16),
            },
            status: "UNKNOWN",
            prim: currentPrim,
          };
          currentTile.pixels[currentIsp.params.index].push(currentIsp);
          currentPrim.pixels++;
          break;
        }

        case "ALREADY_DRAWN":
        case "ZFAIL":
        case "ZFAIL2":
        case "ZFAIL3":
        case "ZFAIL4":
        case "ZFAIL5":
        case "ZFAIL6":
        case "ZFAIL7": {
          if (currentIsp) currentIsp.status = parts[0];
          break;
        }

        case "RENDERED": {
          if (currentIsp) {
            currentIsp.status = "RENDERED";
            if (parts.length > 1) {
              currentIsp.writtenDepth = Number.parseFloat(parts[1]);
            }
          }
          break;
        }

        case "STENCIL": {
          if (currentIsp) {
            currentIsp.status = "STENCIL";
            currentIsp.readStencil = Number.parseInt(parts[1], 16);
          }
          break;
        }

        case "PIXELS": {
          if (!currentTile) break;
          for (let j = 0; j < 1024; j++) {
            const COLOR = lines[i + 1 + j].trim();
            const x = j % 32;
            const y = Math.floor(j / 32);
            ctx.fillStyle = `#${COLOR.substring(2)}`;
            ctx.fillRect(currentTile.x * 32 + x, currentTile.y * 32 + y, 1, 1);
          }
          i += 1024;
          break;
        }
      }
    }

    setTiles(parsedTiles);
    setBgtag(currentBgtag);
  }, []);

  const handleFileChange = useCallback(
    (files: FileList | null) => {
      if (!files || files.length === 0) return;

      setLoading(true);
      const file = files[0];
      const reader = new FileReader();
      reader.onload = (event) => {
        if (event.target?.result) {
          parselog(event.target.result as string);
        }
        setLoading(false);
      };
      reader.readAsText(file);
    },
    [parselog],
  );

  const handleCanvasClick = useCallback(
    (event: React.MouseEvent<HTMLCanvasElement>) => {
      if (!canvasRef.current) return;
      const rect = canvasRef.current.getBoundingClientRect();
      const x = Math.floor(event.clientX - rect.left);
      const y = Math.floor(event.clientY - rect.top);
      const tileX = Math.floor(x / 32);
      const tileY = Math.floor(y / 32);
      const tileId = tileX + tileY * 64;

      if (tiles[tileId] && tiles[tileId].length > 0) {
        const localX = x % 32;
        const localY = y % 32;
        setSelectedPixel({ tileId, tileX, tileY, localX, localY });
      }
    },
    [tiles],
  );

  const handleDrop = useCallback(
    (event: React.DragEvent<HTMLDivElement>) => {
      event.preventDefault();
      handleFileChange(event.dataTransfer.files);
    },
    [handleFileChange],
  );

  const handleDragOver = useCallback((event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
  }, []);

  const detailsContent = useMemo(() => {
    if (!selectedPixel) return null;

    const { tileId, tileX, tileY, localX, localY } = selectedPixel;
    const localIndex = localX + localY * 32;
    const tileDatas = tiles[tileId];

    if (!tileDatas || tileDatas.length === 0) {
      return <Typography>No data for this tile.</Typography>;
    }

    return (
      <Box sx={{ maxHeight: "70vh", overflow: "auto", p: 2 }}>
        <Typography variant="h6" gutterBottom>
          Tile Details
        </Typography>
        <Typography variant="body2">
          <strong>Tile ID:</strong> {tileId}
        </Typography>
        <Typography variant="body2">
          <strong>Local X:</strong> {localX}
        </Typography>
        <Typography variant="body2">
          <strong>Local Y:</strong> {localY}
        </Typography>
        <Box sx={{ borderTop: 1, borderColor: "divider", mt: 2, pt: 2 }}>
          {tileDatas.map((tile, idx) => {
            const pixelWrites = tile.pixels[localX + localY * 32] || [];
            const globalOps = tile.global_ops || [];
            const events = [
              ...pixelWrites.map((w) => ({ type: "write" as const, seq: w.seq, data: w })),
              ...globalOps.map((g) => ({ type: "global" as const, seq: g.seq, data: g })),
            ].sort((a, b) => a.seq - b.seq);

            return (
              <Box key={idx} sx={{ mb: 3 }}>
                <Typography variant="subtitle1" fontWeight={600}>
                  Tile Render #{idx + 1}
                </Typography>
                <Box component="ul" sx={{ fontSize: "0.875rem", pl: 3 }}>
                  <li>
                    <strong>Base:</strong> 0x{tile.base?.toString(16) ?? "?"}
                  </li>
                  <li>
                    <strong>Control:</strong> 0x{tile.control?.toString(16) ?? "?"}
                  </li>
                  <li>
                    <strong>Opaque:</strong> 0x{tile.opaque?.toString(16) ?? "?"}
                  </li>
                  <li>
                    <strong>Opaque Mod:</strong> 0x{tile.opaque_mod?.toString(16) ?? "?"}
                  </li>
                  <li>
                    <strong>Trans:</strong> 0x{tile.trans?.toString(16) ?? "?"}
                  </li>
                  <li>
                    <strong>Trans Mod:</strong> 0x{tile.trans_mod?.toString(16) ?? "?"}
                  </li>
                  <li>
                    <strong>Puncht:</strong> 0x{tile.puncht?.toString(16) ?? "?"}
                  </li>
                  <li>
                    <strong>X:</strong> {tile.x}
                  </li>
                  <li>
                    <strong>Y:</strong> {tile.y}
                  </li>
                </Box>

                <Typography variant="subtitle2" fontWeight={600} sx={{ mt: 2 }}>
                  Pixel/Global Events
                </Typography>
                <Box component="ul" sx={{ fontSize: "0.875rem", pl: 3 }}>
                  {events.map((ev, widx) => {
                    if (ev.type === "write") {
                      const write = ev.data as PixelWrite;
                      let heading = write.type;
                      const tagHex = write.params?.tag !== undefined ? write.params.tag.toString(16) : null;

                      return (
                        <li key={widx}>
                          <strong>
                            {heading} #{write.seq}:
                          </strong>
                          <ul>
                            {Object.entries(write.params || {}).map(([k, v]) => {
                              if (k === "tag" && tagHex) {
                                return (
                                  <li key={k}>
                                    {k}: 0x{v.toString(16)}
                                  </li>
                                );
                              }
                              if (["isp", "tsp", "tcw"].includes(k)) {
                                return (
                                  <li key={k}>
                                    {k}: 0x{v.toString(16).padStart(8, "0")}
                                  </li>
                                );
                              }
                              if (["A", "R", "G", "B"].includes(k)) {
                                return (
                                  <li key={k}>
                                    {k}: {v}
                                  </li>
                                );
                              }
                              if (k === "render_mode") {
                                return (
                                  <li key={k}>
                                    {k}: {RENDER_MODES[v as number]}
                                  </li>
                                );
                              }
                              if (k === "mode") {
                                return (
                                  <li key={k}>
                                    {k}: {DEPTH_MODES[v as number]}
                                  </li>
                                );
                              }
                              return (
                                <li key={k}>
                                  {k}: {typeof v === "number" && !Number.isNaN(v) ? v : v}
                                </li>
                              );
                            })}

                            {write.type === "TSP" && write.IB !== undefined && (
                              <li>
                                Base Color: 0x{write.IB.toString(16).padStart(8, "0")}
                                <Box
                                  component="span"
                                  sx={{
                                    display: "inline-block",
                                    width: 16,
                                    height: 16,
                                    ml: 1,
                                    border: "1px solid #888",
                                    verticalAlign: "middle",
                                    bgcolor: `#${write.IB.toString(16).padStart(8, "0").slice(2, 8)}`,
                                  }}
                                />
                              </li>
                            )}

                            {write.type === "TSP" && write.IO !== undefined && (
                              <li>
                                Offset Color: 0x{write.IO.toString(16).padStart(8, "0")}
                                <Box
                                  component="span"
                                  sx={{
                                    display: "inline-block",
                                    width: 16,
                                    height: 16,
                                    ml: 1,
                                    border: "1px solid #888",
                                    verticalAlign: "middle",
                                    bgcolor: `#${write.IO.toString(16).padStart(8, "0").slice(2, 8)}`,
                                  }}
                                />
                              </li>
                            )}

                            {write.status && <li>Status: {write.status}</li>}
                            {write.writtenDepth !== undefined && <li>Written Depth: {write.writtenDepth}</li>}
                            {write.readStencil !== undefined && <li>Read Stencil: {write.readStencil}</li>}
                          </ul>
                        </li>
                      );
                    }
                    const op = ev.data as GlobalOp;
                    return (
                      <li key={widx}>
                        <strong>Global Op:</strong> {op.type} (seq: {op.seq})
                      </li>
                    );
                  })}
                </Box>
              </Box>
            );
          })}
        </Box>
      </Box>
    );
  }, [selectedPixel, tiles]);

  const hasData = tiles.some((t) => t.length > 0);

  return (
    <Panel title="CORE: PowerVR Log Visualizer">
      {!hasData ? (
        <Box
          sx={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            minHeight: 300,
            p: 4,
            border: "4px dashed",
            borderColor: "divider",
            borderRadius: 2,
            bgcolor: "background.paper",
            m: 2,
            cursor: "pointer",
          }}
          onDrop={handleDrop}
          onDragOver={handleDragOver}
          onClick={() => fileInputRef.current?.click()}
        >
          {loading ? (
            <CircularProgress />
          ) : (
            <>
              <UploadFileIcon sx={{ fontSize: 64, color: "text.secondary", mb: 2 }} />
              <Typography variant="h6" color="text.secondary" gutterBottom>
                Drop your refsw2 log here
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                OR
              </Typography>
              <Button variant="contained" component="label">
                Choose File
                <input
                  ref={fileInputRef}
                  type="file"
                  hidden
                  accept=".log,text/plain"
                  onChange={(e) => handleFileChange(e.target.files)}
                />
              </Button>
            </>
          )}
        </Box>
      ) : (
        <Box sx={{ display: "flex", flexDirection: "column", gap: 2, p: 2 }}>
          <Box sx={{ display: "flex", gap: 2, alignItems: "flex-start" }}>
            <Box>
              <canvas
                ref={canvasRef}
                width={640}
                height={480}
                onClick={handleCanvasClick}
                style={{
                  border: "1px solid #333",
                  display: "block",
                  imageRendering: "pixelated",
                  cursor: "crosshair",
                }}
              />
              {bgtag !== null && (
                <Typography variant="caption" color="text.secondary" sx={{ mt: 1, display: "block" }}>
                  Background Tag: 0x{bgtag.toString(16)}
                </Typography>
              )}
            </Box>
            {selectedPixel && <Box sx={{ flex: 1, minWidth: 400 }}>{detailsContent}</Box>}
          </Box>
        </Box>
      )}
    </Panel>
  );
};
