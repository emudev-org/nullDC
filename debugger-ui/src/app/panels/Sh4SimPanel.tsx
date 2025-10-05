import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
import PrintIcon from "@mui/icons-material/Print";
import { deflate, inflate } from "pako";
import { alpha, lighten, useTheme } from "@mui/material/styles";
import type { Theme } from "@mui/material/styles";
import { Panel } from "../layout/Panel";
import { SH4_SIM_DEFAULT_SOURCE } from "../sh4Sim/defaultSource";
import { getAssembleError, simulate, SH4_MNEMONICS, SH4_REGISTERS } from "../sh4Sim/sim";
import type { SimBlock, SimCell, SimulateResult } from "../sh4Sim/sim";
import Editor from "@monaco-editor/react";
// Using "any" here because @monaco-editor/react doesn't export a Monaco type compatible with monaco-editor versions.

const BASE_SHARE_URL = "https://sh4-sim.dreamcast.wiki";

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
  const editorRef = useRef<any | null>(null);
  const editorContainerRef = useRef<HTMLDivElement | null>(null);
  const [hoverState, setHoverState] = useState<{
    blockId: string | null;
    rowIndex: number | null;
    cycle: number | null;
    highlightKeys: string[];
  }>({ blockId: null, rowIndex: null, cycle: null, highlightKeys: [] });

  const highlightKeySet = useMemo(() => createHighlightSet(hoverState.highlightKeys), [hoverState.highlightKeys]);
  const effectiveRowIndex = hideCrosshairs ? null : hoverState.rowIndex;
  const effectiveCycle = hideCrosshairs ? null : hoverState.cycle;

  const simulation = useMemo<SimulateResult>(() => {
    try {
      return simulate(source);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Simulation failed.";
      return { blocks: [] as SimBlock[], error: message };
    }
  }, [source]);

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
        setHoverState((prev) => {
          if (
            prev.blockId === nextState.blockId &&
            prev.rowIndex === nextState.rowIndex &&
            prev.cycle === nextState.cycle &&
            prev.highlightKeys.length === nextState.highlightKeys.length &&
            prev.highlightKeys.every((key, idx) => key === nextState.highlightKeys[idx])
          ) {
            return prev;
          }
          return nextState;
        });
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
    [hideCrosshairs],
  );

  const handleMouseLeave = useCallback(() => {
    if (typeof window !== "undefined" && hoverRafRef.current !== null) {
      window.cancelAnimationFrame(hoverRafRef.current);
      hoverRafRef.current = null;
    }
    setHoverState({ blockId: null, rowIndex: null, cycle: null, highlightKeys: [] });
  }, []);

  const renderBlockHeader = (block: SimBlock, index: number) => {
    const blockId = block.id;
    const shareTarget = blockId;
    const copied = copiedTarget === shareTarget;
    return (
      <Stack direction="row" alignItems="center" spacing={1} sx={{ mt: index === 0 ? 0 : 2 }}>
        <Typography variant="h6" id={blockId} sx={{ fontFamily: "monospace" }}>
          {block.title ?? `Fragment ${index + 1}`}
        </Typography>
        <Tooltip title={copied ? "Copied!" : "Copy link"} open={copied} arrow disableHoverListener>
          <span>
            <IconButton size="small" onClick={() => handleCopyShare(shareTarget)} disabled={!shareToken}>
              <ContentCopyIcon fontSize="inherit" />
            </IconButton>
          </span>
        </Tooltip>
        <Typography variant="caption" color="text.secondary">
          {formatCycleLabel(block)}
        </Typography>
      </Stack>
    );
  };

  const renderBlockSubtitle = (block: SimBlock) => {
    if (!block.subtitle) {
      return null;
    }
    const shareTarget = `${block.id}-subtitle`;
    const copied = copiedTarget === shareTarget;
    return (
      <Stack direction="row" alignItems="center" spacing={1} sx={{ ml: 1 }}>
        <Typography variant="subtitle1" id={shareTarget} sx={{ fontFamily: "monospace" }}>
          {block.subtitle}
        </Typography>
        <Tooltip title={copied ? "Copied!" : "Copy link"} open={copied} arrow disableHoverListener>
          <span>
            <IconButton size="small" onClick={() => handleCopyShare(shareTarget)} disabled={!shareToken}>
              <ContentCopyIcon fontSize="inherit" />
            </IconButton>
          </span>
        </Tooltip>
      </Stack>
    );
  };

  const renderCell = (blockId: string, cell: SimCell, rowIndex: number) => {
    const isInstructionColumn = isStickyColumn(cell);
    const cellKey = getCellKey(cell);
    const baseBackground = getBaseRowColor(rowIndex, theme);
    let backgroundColor = baseBackground;
    let color = "inherit";

    const isActiveBlock = hoverState.blockId === blockId;

    if (cell.full) {
      const colors = getFullColors(plainFormat, theme);
      backgroundColor = colors.background;
      color = colors.color;
    } else if (cell.stall) {
      const colors = getStallColors(plainFormat, theme);
      backgroundColor = colors.background;
      color = colors.color;
    }

    const isRelationHighlighted = isActiveBlock && cell.selfKeys.some((key) => highlightKeySet.has(key));
    if (isRelationHighlighted) {
      const colors = getRelationColors(plainFormat, theme);
      backgroundColor = colors.background;
      color = colors.color;
    }

    const isRowHighlighted = isActiveBlock && effectiveRowIndex !== null && cell.rowIndex === effectiveRowIndex;
    const isCycleHighlighted = isActiveBlock && effectiveCycle !== null && cell.cycle !== null && cell.cycle === effectiveCycle;

    const borders = lessBorders ? "transparent" : alpha(theme.palette.divider, 0.6);
    const highlightBorder = theme.palette.primary.main;

    return (
      <TableCell
        key={cellKey}
        onMouseEnter={() => handleMouseEnter(blockId, cell)}
        onMouseLeave={handleMouseLeave}
        title={cell.explanation ?? undefined}
        sx={{
          position: isInstructionColumn ? "sticky" : "static",
          left: isInstructionColumn ? 0 : "auto",
          zIndex: isInstructionColumn ? 2 : 1,
          backgroundColor,
          color,
          border: "1px solid",
          borderColor: isRelationHighlighted || isRowHighlighted || isCycleHighlighted ? highlightBorder : borders,
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
  };

  const renderBlock = (block: SimBlock, index: number) => {
    return (
      <Box key={block.id} sx={{ display: "flex", flexDirection: "column", gap: 1, minWidth: 0 }}>
        {renderBlockHeader(block, index)}
        {renderBlockSubtitle(block)}
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
            <TableBody>
              {block.table.rows.map((row, rowIdx) => (
                <TableRow key={row.rowKey} sx={{ backgroundColor: getBaseRowColor(rowIdx, theme) }}>
                  {row.cells.map((cell) => renderCell(block.id, cell, rowIdx))}
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      </Box>
    );
  };

  const copiedMain = copiedTarget === "main";
  const assembleError = simulation.error ?? getAssembleError();

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const handleEditorWillMount = useCallback((monaco: any) => {
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

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const handleEditorDidMount = useCallback((editor: any) => {
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
  }, []);

  return (
    <Panel
      title="SH4: Sim"
      action={
        <Stack direction="row" spacing={1}>
          <Tooltip title={copiedMain ? "Copied!" : "Copy share link"} open={copiedMain} arrow disableHoverListener>
            <span>
              <Button
                variant="outlined"
                size="small"
                startIcon={<ContentCopyIcon fontSize="small" />}
                onClick={() => handleCopyShare(null)}
                disabled={!shareToken}
              >
                Share
              </Button>
            </span>
          </Tooltip>
          <Button variant="outlined" size="small" startIcon={<PrintIcon fontSize="small" />} onClick={() => window.print()}>
            Print
          </Button>
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
        <Stack direction="row" spacing={2} alignItems="center">
          <Tooltip title={copiedMain ? "Copied!" : "Copy share link"} open={copiedMain} arrow disableHoverListener>
            <span>
              <Button
                variant="outlined"
                size="small"
                startIcon={<ContentCopyIcon fontSize="small" />}
                onClick={() => handleCopyShare(null)}
                disabled={!shareToken}
              >
                Share
              </Button>
            </span>
          </Tooltip>
          <Button variant="outlined" size="small" startIcon={<PrintIcon fontSize="small" />} onClick={() => window.print()}>
            Print
          </Button>
        </Stack>
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
                {simulation.blocks.map((block, index) => renderBlock(block, index))}
              </Box>
            </Box>
          )}
        </Box>
      </Box>
    </Panel>
  );
};
