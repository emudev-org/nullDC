import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import {
  Box,
  Button,
  Checkbox,
  FormControlLabel,
  IconButton,
  Stack,
  Tooltip,
  Typography,
  Alert,
} from "@mui/material";
import { Table, TableBody, TableCell, TableContainer, TableRow } from "@mui/material";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import { deflate, inflate } from "pako";
import { alpha, lighten, useTheme } from "@mui/material/styles";
import type { Theme } from "@mui/material/styles";
import { Panel } from "../layout/Panel";
import { SH4_SIM_DEFAULT_SOURCE } from "../sh4Sim/defaultSource";
import { getAssembleError, simulate, SH4_MNEMONICS, SH4_REGISTERS } from "../sh4Sim/sim";
import type { SimBlock, SimCell, SimRow, SimulateResult } from "../sh4Sim/sim";
import Editor from "@monaco-editor/react";
import type { Monaco } from "@monaco-editor/react";
import type { editor as MonacoEditor } from "monaco-editor";
import { sha256FromJson } from "../../lib/sha256";

const BASE_SHARE_URL = "https://sh4-sim.dreamcast.wiki";
const SH4_SIM_SOURCE_STORAGE_KEY = "nulldc-debugger-sh4-sim-source";

const encodeSource = (value: string) => {
  const bytes = deflate(new TextEncoder().encode(value));
  let binary = "";
  bytes.forEach((byte: number) => {
    binary += String.fromCharCode(byte);
  });
  return btoa(binary);
};

const decodeSource = (value: string) => {
  const binary = atob(value);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder().decode(inflate(bytes));
};

const RELATION_CLASS = "sh4-sim__cell--relation";
const ROW_CLASS = "sh4-sim__cell--row";
const CYCLE_CLASS = "sh4-sim__cell--cycle";

const resolveInitialSource = () => {
  if (typeof window === "undefined") {
    return SH4_SIM_DEFAULT_SOURCE;
  }
  const params = new URLSearchParams(window.location.search);
  if (params.has("source")) {
    try {
      return decodeSource(params.get("source") ?? "");
    } catch (error) {
      console.error("Failed to decode source parameter", error);
      return SH4_SIM_DEFAULT_SOURCE;
    }
  }
  try {
    const storedSource = window.localStorage.getItem(SH4_SIM_SOURCE_STORAGE_KEY);
    if (storedSource && storedSource !== "") {
      return storedSource;
    }
  } catch (error) {
    console.warn("Failed to read SH4 sim source from storage", error);
  }
  return SH4_SIM_DEFAULT_SOURCE;
};

const createHighlightSet = (keys: string[]) => new Set(keys);

const getCellKey = (cell: SimCell) => `${cell.rowIndex}-${cell.columnIndex}-${cell.id ?? ""}`;

const isStickyColumn = (cell: SimCell) => cell.columnIndex === 0;

const getBaseRowColor = (rowIndex: number, theme: Theme) =>
  rowIndex % 2 === 0
    ? lighten(theme.palette.background.paper, 0.06)
    : lighten(theme.palette.background.paper, 0.12);

const getRelationColors = (plainFormat: boolean, theme: Theme) =>
  plainFormat
    ? { background: alpha(theme.palette.primary.main, 0.16), color: theme.palette.primary.dark }
    : { background: theme.palette.primary.main, color: theme.palette.primary.contrastText };

const getStallColors = (plainFormat: boolean, theme: Theme) =>
  plainFormat
    ? { background: alpha(theme.palette.error.main, 0.18), color: theme.palette.error.dark }
    : { background: theme.palette.error.main, color: theme.palette.error.contrastText };

const getFullColors = (plainFormat: boolean, theme: Theme) =>
  plainFormat
    ? { background: alpha(theme.palette.success.main, 0.18), color: theme.palette.success.dark }
    : { background: theme.palette.success.main, color: theme.palette.success.contrastText };

const buildShareUrl = (token: string | null, hash?: string | null) => {
  const query = token ? `?source=${encodeURIComponent(token)}` : "";
  const fragment = hash ? `#${hash}` : "";
  return `${BASE_SHARE_URL}${query}${fragment}`;
};

const formatCycleLabel = (block: SimBlock) => `${block.cycleCount} cycles`;

type SimCellWithHash = {
  cell: SimCell;
  cellHash: string;
};

type SimRowWithCells = {
  row: SimRow;
  cells: SimCellWithHash[];
};

type SimBlockRenderEntry = {
  block: SimBlock;
  index: number;
  blockHash: string;
  tableHash: string;
  rows: SimRowWithCells[];
};

type Sh4SimCellProps = {
  blockId: string;
  cellEntry: SimCellWithHash;
  rowBackground: string;
  plainFormat: boolean;
  lessBorders: boolean;
  onMouseEnter: (blockId: string, cell: SimCell) => void;
  onMouseLeave: () => void;
};

const Sh4SimCell = memo<Sh4SimCellProps>(
  ({
    blockId,
    cellEntry,
    rowBackground,
    plainFormat,
    lessBorders,
    onMouseEnter,
    onMouseLeave,
  }) => {
    const theme = useTheme<Theme>();
    const { cell } = cellEntry;
    const isInstructionColumn = isStickyColumn(cell);
    const relationColors = getRelationColors(plainFormat, theme);

    let backgroundColor = rowBackground;
    let color = "inherit";

    if (cell.full) {
      const colors = getFullColors(plainFormat, theme);
      backgroundColor = colors.background;
      color = colors.color;
    } else if (cell.stall) {
      const colors = getStallColors(plainFormat, theme);
      backgroundColor = colors.background;
      color = colors.color;
    }

    const borders = lessBorders ? "transparent" : alpha(theme.palette.divider, 0.6);
    const highlightBorder = theme.palette.primary.main;

    return (
      <TableCell
        onMouseEnter={() => onMouseEnter(blockId, cell)}
        onMouseLeave={onMouseLeave}
        title={cell.explanation ?? undefined}
        data-block-id={blockId}
        data-row-index={cell.rowIndex}
        data-cycle={cell.cycle ?? undefined}
        data-self-keys={cell.selfKeys.join("|") || undefined}
        sx={{
          position: isInstructionColumn ? "sticky" : "static",
          left: isInstructionColumn ? 0 : "auto",
          zIndex: isInstructionColumn ? 2 : 1,
          backgroundColor,
          color,
          border: "1px solid",
          borderColor: borders,
          minWidth: cell.columnIndex === 0 ? 180 : 80,
          maxWidth: 200,
          px: 1.5,
          py: 0.75,
          fontFamily: "monospace",
          fontSize: 13,
          whiteSpace: "nowrap",
          cursor: "pointer",
          textAlign: cell.columnIndex === 0 ? "left" : "center",
          fontWeight: cell.rowIndex === 0 ? 600 : 400,
          outline: cell.lock ? `2px solid ${alpha(theme.palette.warning.dark, 0.75)}` : "none",
          outlineOffset: cell.lock ? -2 : 0,
          opacity: cell.screenHidden ? 0 : 1,
          userSelect: "none",
          transition: "background-color 80ms ease, border-color 80ms ease, color 80ms ease",
          '&:hover': {
            boxShadow: `inset 0 0 0 1px ${alpha(theme.palette.primary.main, 0.45)}`,
          },
          [`&.${RELATION_CLASS}`]: {
            backgroundColor: relationColors.background,
            color: relationColors.color,
            borderColor: highlightBorder,
          },
          [`&.${ROW_CLASS}`]: {
            borderColor: highlightBorder,
          },
          [`&.${CYCLE_CLASS}`]: {
            borderColor: highlightBorder,
          },
        }}
      >
        <Box
          component="span"
          sx={{
            visibility: cell.screenHiddenText ? "hidden" : "visible",
            pointerEvents: "none",
          }}
        >
          {cell.text}
        </Box>
      </TableCell>
    );
  },
  (prev, next) =>
    prev.cellEntry.cellHash === next.cellEntry.cellHash &&
    prev.plainFormat === next.plainFormat &&
    prev.lessBorders === next.lessBorders &&
    prev.blockId === next.blockId &&
    prev.rowBackground === next.rowBackground &&
    prev.onMouseEnter === next.onMouseEnter &&
    prev.onMouseLeave === next.onMouseLeave,
);

type Sh4SimBlockProps = {
  entry: SimBlockRenderEntry;
  plainFormat: boolean;
  lessBorders: boolean;
  shareEnabled: boolean;
  headerCopied: boolean;
  subtitleCopied: boolean;
  onMouseEnter: (blockId: string, cell: SimCell) => void;
  onMouseLeave: () => void;
  registerBlockBodyRef: (blockId: string) => (node: HTMLTableSectionElement | null) => void;
  handleCopyShare: (hash: string | null) => Promise<void> | void;
};

const Sh4SimBlock = memo<Sh4SimBlockProps>(
  ({
    entry,
    plainFormat,
    lessBorders,
    shareEnabled,
    headerCopied,
    subtitleCopied,
    onMouseEnter,
    onMouseLeave,
    registerBlockBodyRef,
    handleCopyShare,
  }) => {
    const theme = useTheme<Theme>();
    const { block, index, rows } = entry;
    const displayTitle = block.title ?? `Fragment ${index + 1}`;
    const showCycleOnHeader = !block.subtitle;
    const showGeneratedHeader = Boolean(block.title) || !block.subtitle;
    const tableBodyRef = useMemo(() => registerBlockBodyRef(block.id), [registerBlockBodyRef, block.id]);

    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1, minWidth: 0 }}>
        {showGeneratedHeader && (
          <Stack direction="row" alignItems="center" spacing={1} sx={{ mt: index === 0 ? 0 : 2 }}>
            <Typography variant="h6" id={block.id} sx={{ fontFamily: "monospace" }}>
              {displayTitle}
            </Typography>
            <Tooltip title={headerCopied ? "Copied!" : "Copy link"} open={headerCopied} arrow disableHoverListener>
              <span>
                <IconButton size="small" onClick={() => handleCopyShare(block.id)} disabled={!shareEnabled}>
                  <ContentCopyIcon fontSize="inherit" />
                </IconButton>
              </span>
            </Tooltip>
            {showCycleOnHeader && (
              <Typography variant="caption" color="text.secondary">
                {formatCycleLabel(block)}
              </Typography>
            )}
          </Stack>
        )}
        {block.subtitle && block.subtitle !== displayTitle && (
          <Stack direction="row" alignItems="center" spacing={1} sx={{ ml: 1 }}>
            <Typography variant="subtitle1" id={`${block.id}-subtitle`} sx={{ fontFamily: "monospace" }}>
              {block.subtitle}
            </Typography>
            <Tooltip title={subtitleCopied ? "Copied!" : "Copy link"} open={subtitleCopied} arrow disableHoverListener>
              <span>
                <IconButton size="small" onClick={() => handleCopyShare(`${block.id}-subtitle`)} disabled={!shareEnabled}>
                  <ContentCopyIcon fontSize="inherit" />
                </IconButton>
              </span>
            </Tooltip>
            <Typography variant="caption" color="text.secondary">
              {formatCycleLabel(block)}
            </Typography>
          </Stack>
        )}
        <TableContainer
          component={Box}
          sx={{
            width: "100%",
            maxWidth: "100%",
            minWidth: 0,
            overflowX: "auto",
            overflowY: "auto",
            maxHeight: 480,
            border: "1px solid",
            borderColor: "divider",
            borderRadius: 1,
          }}
        >
          <Table
            size="small"
            sx={{
              borderCollapse: "separate",
              borderSpacing: 0,
              minWidth: block.table.columnCount * 80,
              width: "max-content",
            }}
          >
            <TableBody ref={tableBodyRef}>
              {rows.map((rowEntry, rowIdx) => {
                const rowBackground = getBaseRowColor(rowIdx, theme);
                return (
                  <TableRow key={rowEntry.row.rowKey} sx={{ backgroundColor: rowBackground }}>
                    {rowEntry.cells.map((cellEntry) => (
                      <Sh4SimCell
                        key={getCellKey(cellEntry.cell)}
                        blockId={block.id}
                        cellEntry={cellEntry}
                        rowBackground={rowBackground}
                        plainFormat={plainFormat}
                        lessBorders={lessBorders}
                        onMouseEnter={onMouseEnter}
                        onMouseLeave={onMouseLeave}
                      />
                    ))}
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </TableContainer>
      </Box>
    );
  },
  (prev, next) =>
    prev.entry.blockHash === next.entry.blockHash &&
    prev.entry.tableHash === next.entry.tableHash &&
    prev.plainFormat === next.plainFormat &&
    prev.lessBorders === next.lessBorders &&
    prev.shareEnabled === next.shareEnabled &&
    prev.headerCopied === next.headerCopied &&
    prev.subtitleCopied === next.subtitleCopied &&
    prev.onMouseEnter === next.onMouseEnter &&
    prev.onMouseLeave === next.onMouseLeave &&
    prev.registerBlockBodyRef === next.registerBlockBodyRef &&
    prev.handleCopyShare === next.handleCopyShare,
);

export const Sh4SimPanel = () => {
  const theme = useTheme<Theme>();
  const [source, setSource] = useState(resolveInitialSource);
  const [hideCrosshairs, setHideCrosshairs] = useState(false);
  const [plainFormat, setPlainFormat] = useState(false);
  const [lessBorders, setLessBorders] = useState(false);
  const [shareToken, setShareToken] = useState<string | null>(null);
  const [copiedTarget, setCopiedTarget] = useState<string | null>(null);
  const copyTimeoutRef = useRef<number | null>(null);
  const hoverRafRef = useRef<number | null>(null);
  const editorRef = useRef<MonacoEditor.IStandaloneCodeEditor | null>(null);
  const editorContainerRef = useRef<HTMLDivElement | null>(null);
  const hoverStateRef = useRef<{
    blockId: string | null;
    rowIndex: number | null;
    cycle: number | null;
    highlightKeys: string[];
  }>({ blockId: null, rowIndex: null, cycle: null, highlightKeys: [] });
  const hoverTargetsRef = useRef<{
    relation: HTMLElement[];
    row: HTMLElement[];
    cycle: HTMLElement[];
  }>({ relation: [], row: [], cycle: [] });
  const blockBodyRefs = useRef(new Map<string, HTMLTableSectionElement>());

  const [simulation, setSimulation] = useState<SimulateResult>(() => {
    try {
      return simulate(source);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Simulation failed.";
      return { blocks: [] as SimBlock[], error: message };
    }
  });

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    try {
      window.localStorage.setItem(SH4_SIM_SOURCE_STORAGE_KEY, source);
    } catch (error) {
      console.warn("Failed to persist SH4 sim source", error);
    }
  }, [source]);

  useEffect(() => {
    try {
      const result = simulate(source);
      if (result.error) {
        const message = result.error ?? getAssembleError() ?? "Simulation failed.";
        setSimulation((prev) => ({ blocks: prev.blocks, error: message }));
      } else {
        setSimulation({ blocks: result.blocks, error: null });
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Simulation failed.";
      setSimulation((prev) => ({ blocks: prev.blocks, error: message }));
    }
  }, [source]);

  const blockEntries = useMemo<SimBlockRenderEntry[]>(() => {
    return simulation.blocks.map((block, index) => {
      const tableHash = sha256FromJson(block.table);
      const rows = block.table.rows.map((row) => ({
        row,
        cells: row.cells.map((cell) => ({
          cell,
          cellHash: sha256FromJson(cell),
        })),
      }));
      const blockHash = sha256FromJson({
        id: block.id,
        title: block.title,
        subtitle: block.subtitle,
        cycleCount: block.cycleCount,
        tableHash,
      });
      return { block, index, blockHash, tableHash, rows };
    });
  }, [simulation]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    try {
      const encoded = encodeSource(source);
      setShareToken(encoded);
    } catch (error) {
      console.error("Failed to encode source", error);
      setShareToken(null);
    }
  }, [source]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    const hash = window.location.hash.replace(/^#/, "");
    if (!hash) {
      return;
    }
    requestAnimationFrame(() => {
      const element = document.getElementById(hash);
      if (element) {
        element.scrollIntoView({ behavior: "smooth", block: "start" });
      }
    });
  }, [simulation.blocks.length]);

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current !== null) {
        window.clearTimeout(copyTimeoutRef.current);
      }
      if (hoverRafRef.current !== null) {
        window.cancelAnimationFrame(hoverRafRef.current);
      }
    };
  }, []);

  const handleCopyShare = useCallback(
    async (hash: string | null) => {
      if (!shareToken || typeof navigator === "undefined" || !navigator.clipboard) {
        return;
      }
      const target = hash ?? "main";
      const url = buildShareUrl(shareToken, hash);
      try {
        await navigator.clipboard.writeText(url);
        setCopiedTarget(target);
        if (typeof window !== "undefined") {
          window.location.hash = hash ? `#${hash}` : "";
        }
        if (copyTimeoutRef.current !== null) {
          window.clearTimeout(copyTimeoutRef.current);
        }
        copyTimeoutRef.current = window.setTimeout(() => {
          setCopiedTarget(null);
        }, 2000);
      } catch (error) {
        console.error("Failed to copy share URL", error);
      }
    },
    [shareToken],
  );

  const clearHighlights = useCallback(() => {
    const targets = hoverTargetsRef.current;
    targets.relation.forEach((element) => element.classList.remove(RELATION_CLASS));
    targets.row.forEach((element) => element.classList.remove(ROW_CLASS));
    targets.cycle.forEach((element) => element.classList.remove(CYCLE_CLASS));
    hoverTargetsRef.current = { relation: [], row: [], cycle: [] };
  }, []);

  const applyHighlights = useCallback(
    (nextState: { blockId: string | null; rowIndex: number | null; cycle: number | null; highlightKeys: string[] }) => {
      const prevState = hoverStateRef.current;
      const isSame =
        prevState.blockId === nextState.blockId &&
        prevState.rowIndex === nextState.rowIndex &&
        prevState.cycle === nextState.cycle &&
        prevState.highlightKeys.length === nextState.highlightKeys.length &&
        prevState.highlightKeys.every((key, index) => key === nextState.highlightKeys[index]);

      if (isSame) {
        return;
      }

      clearHighlights();

      hoverStateRef.current = nextState;

      if (!nextState.blockId) {
        return;
      }

      const blockBody = blockBodyRefs.current.get(nextState.blockId);
      if (!blockBody) {
        return;
      }

      const nextTargets: { relation: HTMLElement[]; row: HTMLElement[]; cycle: HTMLElement[] } = {
        relation: [],
        row: [],
        cycle: [],
      };

      if (nextState.highlightKeys.length > 0) {
        const highlightSet = createHighlightSet(nextState.highlightKeys);
        const relationCells = blockBody.querySelectorAll<HTMLElement>("[data-self-keys]");
        relationCells.forEach((element) => {
          const keysAttr = element.dataset.selfKeys;
          if (!keysAttr) {
            return;
          }
          const cellKeys = keysAttr.split("|");
          if (cellKeys.some((key) => highlightSet.has(key))) {
            element.classList.add(RELATION_CLASS);
            nextTargets.relation.push(element);
          }
        });
      }

      if (nextState.rowIndex !== null) {
        const rowCells = blockBody.querySelectorAll<HTMLElement>(`[data-row-index="${nextState.rowIndex}"]`);
        rowCells.forEach((element) => {
          element.classList.add(ROW_CLASS);
          nextTargets.row.push(element);
        });
      }

      if (nextState.cycle !== null) {
        const cycleCells = blockBody.querySelectorAll<HTMLElement>(`[data-cycle="${nextState.cycle}"]`);
        cycleCells.forEach((element) => {
          element.classList.add(CYCLE_CLASS);
          nextTargets.cycle.push(element);
        });
      }

      hoverTargetsRef.current = nextTargets;
    },
    [clearHighlights],
  );

  const handleMouseEnter = useCallback(
    (blockId: string, cell: SimCell) => {
      const nextState = {
        blockId,
        rowIndex: hideCrosshairs ? null : cell.rowIndex,
        cycle: hideCrosshairs ? null : cell.cycle,
        highlightKeys: [...cell.selfKeys, ...cell.relevantKeys],
      };

      if (import.meta.env.DEV) {
        console.debug("hover", blockId, cell.rowIndex, cell.columnIndex);
      }

      const flush = () => {
        applyHighlights(nextState);
        hoverRafRef.current = null;
      };

      if (typeof window !== "undefined") {
        if (hoverRafRef.current !== null) {
          window.cancelAnimationFrame(hoverRafRef.current);
        }
        hoverRafRef.current = window.requestAnimationFrame(flush);
      } else {
        flush();
      }
    },
    [applyHighlights, hideCrosshairs],
  );

  const handleMouseLeave = useCallback(() => {
    if (typeof window !== "undefined" && hoverRafRef.current !== null) {
      window.cancelAnimationFrame(hoverRafRef.current);
      hoverRafRef.current = null;
    }
    clearHighlights();
    hoverStateRef.current = { blockId: null, rowIndex: null, cycle: null, highlightKeys: [] };
  }, [clearHighlights]);

  const registerBlockBodyRef = useCallback(
    (blockId: string) =>
      (node: HTMLTableSectionElement | null) => {
        if (node) {
          blockBodyRefs.current.set(blockId, node);
        } else {
          blockBodyRefs.current.delete(blockId);
        }
      },
    [],
  );

  useLayoutEffect(() => {
    const current = hoverStateRef.current;
    if (!current.blockId) {
      return;
    }
    const adjustedState = {
      blockId: current.blockId,
      rowIndex: hideCrosshairs ? null : current.rowIndex,
      cycle: hideCrosshairs ? null : current.cycle,
      highlightKeys: current.highlightKeys,
    };
    applyHighlights(adjustedState);
  }, [applyHighlights, hideCrosshairs, plainFormat, lessBorders]);

  const shareEnabled = Boolean(shareToken);
  const copiedMain = copiedTarget === "main";
  const assembleError = simulation.error ?? getAssembleError();

  const handleEditorWillMount = useCallback((monaco: Monaco) => {
    const languageId = "sh4asm";

    if (!monaco.languages.getLanguages().some((l: { id: string }) => l.id === languageId)) {
      monaco.languages.register({ id: languageId });

      monaco.languages.setLanguageConfiguration(languageId, {
        comments: {
          lineComment: ";",
        },
        brackets: [
          ["(", ")"],
          ["[", "]"],
          ["{", "}"],
        ],
        autoClosingPairs: [
          { open: "(", close: ")" },
          { open: "[", close: "]" },
          { open: "{", close: "}" },
          { open: '"', close: '"' },
          { open: "'", close: "'" },
        ],
      });

      const instructions = SH4_MNEMONICS;

      const registers = SH4_REGISTERS;

      const directives = [
        ".align",".byte",".word",".long",".int",".data",".text",".section",".global",".globl",".ascii",".asciz",".org",".set",".equ",
      ];

      monaco.languages.setMonarchTokensProvider(languageId, {
        ignoreCase: true,
        defaultToken: "",
        tokenPostfix: ".s",
        keywords: instructions,
        registers,
        directives,
        brackets: [
          { open: "[", close: "]", token: "delimiter.square" },
          { open: "(", close: ")", token: "delimiter.parenthesis" },
          { open: "{", close: "}", token: "delimiter.bracket" },
        ],
        tokenizer: {
          root: [
            // Comments
            [/;.*/, "comment"],
            [/#.*/, "comment"],

            // Labels at BOL: label:
            [/^\s*[A-Za-z_.][\w.]*:/, "type.identifier"],

            // Directives starting with dot
            [/\.[A-Za-z_.][\w.]*/, { cases: { "@directives": "keyword.directive", "@default": "keyword" } }],

            // Registers
            [/(?:\b)(r1[0-5]|r[0-9]|pr|sr|gbr|vbr|mach|macl|pc|ssp|usp)(?:\b)/, "variable.predefined"],

            // Numbers
            [/0x[0-9a-fA-F]+/, "number.hex"],
            [/\b\d+\b/, "number"],

            // Strings
            [/"([^"\\]|\\.)*"/, "string"],
            [/'([^'\\]|\\.)*'/, "string"],

            // Punctuation / delimiters
            // Highlight '@' when followed by optional '-' and a GPR (R0..R15)
            [/@(?=-?(?:[Rr](?:1[0-5]|[0-9]))\b)/, "address.at"],
            [/\(|\)|\[|\]|,|:/, "delimiter"],
            [/\+|-|\*/, "operator"],

            // Identifiers / mnemonics
            [/[A-Za-z_.][\w.]*/, {
              cases: {
                "@keywords": "keyword",
                "@registers": "variable.predefined",
                "@default": "identifier",
              }
            }],

            // Whitespace
            [/\s+/, "white"],
          ],
        },
      });

      // Define custom themes that color address-at token red
      monaco.editor.defineTheme("sh4-dark", {
        base: "vs-dark",
        inherit: true,
        rules: [
          { token: "address.at", foreground: "ff3b30" },
        ],
        colors: {},
      });
      monaco.editor.defineTheme("sh4-light", {
        base: "vs",
        inherit: true,
        rules: [
          { token: "address.at", foreground: "d70000" },
        ],
        colors: {},
      });
    }
  }, []);

  const handleEditorDidMount = useCallback(
    (editor: MonacoEditor.IStandaloneCodeEditor, _monaco: Monaco) => {
      editorRef.current = editor;

      const layout = () => {
        const el = editorContainerRef.current;
        if (!el || !editorRef.current) return;
        editorRef.current.layout({ width: el.clientWidth, height: el.clientHeight });
      };

      // Initial layout
      layout();

      const ro = new ResizeObserver(() => layout());
      if (editorContainerRef.current) {
        ro.observe(editorContainerRef.current);
      }
      window.addEventListener("resize", layout);

      editor.onDidDispose(() => {
        window.removeEventListener("resize", layout);
        ro.disconnect();
      });
    },
    [],
  );

  return (
    <Panel
      action={
        <Stack direction="row" spacing={1}>
          <Tooltip title={copiedMain ? "Copied!" : "Copy share link"} open={copiedMain} arrow disableHoverListener>
            <span>
              <Button
                variant="outlined"
                size="small"
                startIcon={<ContentCopyIcon fontSize="small" />}
                onClick={() => handleCopyShare(null)}
                disabled={!shareEnabled}
              >
                Share
              </Button>
            </span>
          </Tooltip>
        </Stack>
      }
    >
      <Box sx={{ display: "flex", flexDirection: "column", gap: 2, p: 2, minHeight: "100%", minWidth: 0 }}>
        <Box
          sx={{
            position: "relative",
            height: "280px",
            width: "100%",
            border: "1px solid",
            borderColor: "divider",
            borderRadius: 1,
            overflow: "hidden",
          }}
          ref={editorContainerRef}
        >
          <Editor
            beforeMount={handleEditorWillMount}
            onMount={handleEditorDidMount}
            language="sh4asm"
            value={source}
            theme={theme.palette.mode === "dark" ? "sh4-dark" : "sh4-light"}
            onChange={(value) => setSource(value ?? "")}
            height="100%"
            options={{
              fontSize: 14,
              fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
              minimap: { enabled: false },
              lineNumbersMinChars: 3,
              automaticLayout: true,
              renderWhitespace: "selection",
              scrollBeyondLastLine: false,
              wordWrap: "on",
            }}
          />
        </Box>
        <Stack direction="row" spacing={2} alignItems="center" flexWrap="wrap">
          <FormControlLabel
            control={<Checkbox checked={hideCrosshairs} onChange={(event) => setHideCrosshairs(event.target.checked)} />}
            label="Hide Crosshairs"
          />
          <FormControlLabel
            control={<Checkbox checked={plainFormat} onChange={(event) => setPlainFormat(event.target.checked)} />}
            label="Plain Format"
          />
          <FormControlLabel
            control={<Checkbox checked={lessBorders} onChange={(event) => setLessBorders(event.target.checked)} />}
            label="Less Borders"
          />
        </Stack>
        {assembleError && (
          <Alert severity="error" sx={{ maxWidth: 480 }}>
            {assembleError}
          </Alert>
        )}
        <Box
          sx={{
            flex: 1,
            minHeight: 0,
            minWidth: 0,
            display: "flex",
            flexDirection: "column",
            gap: 4,
            overflowX: "hidden",
            overflowY: "visible",
          }}
        >
          {simulation.blocks.length === 0 && !assembleError && (
            <Typography variant="body2" color="text.secondary">
              Enter SH4 instructions above to visualize the pipeline.
            </Typography>
          )}
          {simulation.blocks.length > 0 && (
            <Box sx={{ flex: 1, minHeight: 0, minWidth: 0, overflowX: "auto", overflowY: "visible" }}>
              <Box sx={{ display: "flex", flexDirection: "column", gap: 4, width: "100%", minWidth: 0 }}>
                {blockEntries.map((entry) => (
                  <Sh4SimBlock
                    key={entry.block.id}
                    entry={entry}
                    plainFormat={plainFormat}
                    lessBorders={lessBorders}
                    shareEnabled={shareEnabled}
                    headerCopied={copiedTarget === entry.block.id}
                    subtitleCopied={copiedTarget === `${entry.block.id}-subtitle`}
                    onMouseEnter={handleMouseEnter}
                    onMouseLeave={handleMouseLeave}
                    registerBlockBodyRef={registerBlockBodyRef}
                    handleCopyShare={handleCopyShare}
                  />
                ))}
              </Box>
            </Box>
          )}
        </Box>
      </Box>
    </Panel>
  );
};
