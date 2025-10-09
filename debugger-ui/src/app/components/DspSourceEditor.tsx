import { memo, useCallback, useEffect, useRef, useImperativeHandle, forwardRef } from "react";
import { Box } from "@mui/material";
import Editor from "@monaco-editor/react";
import type { Monaco } from "@monaco-editor/react";
import type { editor } from "monaco-editor";
import { useThemeMode } from "../../theme/ThemeModeProvider";
import { getLastPreprocessedMacros } from "../dsp/dspCompiler";

export interface CompileError {
  line: number;
  message: string;
}

export interface DspSourceEditorProps {
  value: string;
  onChange: (value: string) => void;
  error?: string | null;
  height?: number | string;
  onEditorReady?: () => void;
}

export interface DspSourceEditorRef {
  layout: () => void;
  setError: (error: string | null) => void;
  setErrors: (errors: CompileError[]) => void;
  setStatus: (status: 'compiling' | 'compiled' | 'error' | 'assembling-failed', errorCount?: number, onClickCallback?: () => void) => void;
}

const registerDspSourceLanguage = (monaco: Monaco) => {
  // Register language
  monaco.languages.register({ id: "aica-dsp-source" });

  // Define language tokens
  monaco.languages.setMonarchTokensProvider("aica-dsp-source", {
    tokenizer: {
      root: [
        // Comments
        [/\/\/.*$/, "comment"],
        [/#.*$/, "comment"],

        // Hex numbers (before identifiers to catch 0x prefix)
        [/0x[0-9a-fA-F]+/, "number"],
        [/#0x[0-9a-fA-F]+/, "number"],

        // Identifiers (match before decimal numbers to prevent partial matching)
        [/[a-zA-Z_][a-zA-Z0-9_]*/, {
          cases: {
            '@keywords': 'keyword',
            '@types': 'type',
            '@constants': 'constant',
            '@default': 'identifier'
          }
        }],

        // Decimal numbers (standalone only)
        [/\d+/, "number"],
        [/#-?\d+/, "number"],

        // Operators
        [/[=:\[\]+\/\-]/, "operator"],
      ],
    },
    keywords: ['INPUT', 'OUTPUT', 'MAC', 'SMODE', 'ST', 'STF', 'LD', 'LDF', 'MADRS'],
    types: ['mixer', 'mems', 'cdda', 'input', 'temp', 'madrs', 'yreg', 'adrs', 'shifted', 'acc'],
    constants: ['sat', 'sat2', 'trim', 'trim2', 'lo', 'hi'],
  });

  // Define theme colors
  monaco.editor.defineTheme("aica-dsp-source-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "keyword", foreground: "569cd6", fontStyle: "bold" },
      { token: "type", foreground: "4ec9b0" },
      { token: "constant", foreground: "b5cea8" },
      { token: "number", foreground: "b5cea8" },
      { token: "comment", foreground: "6a9955", fontStyle: "italic" },
      { token: "operator", foreground: "d4d4d4" },
    ],
    colors: {},
  });

  monaco.editor.defineTheme("aica-dsp-source-light", {
    base: "vs",
    inherit: true,
    rules: [
      { token: "keyword", foreground: "0000ff", fontStyle: "bold" },
      { token: "type", foreground: "267f99" },
      { token: "constant", foreground: "09885a" },
      { token: "number", foreground: "09885a" },
      { token: "comment", foreground: "008000", fontStyle: "italic" },
      { token: "operator", foreground: "000000" },
    ],
    colors: {},
  });
};

const DspSourceEditorComponent = forwardRef<DspSourceEditorRef, DspSourceEditorProps>(
  ({ value, onChange, error, height = "100%", onEditorReady }, ref) => {
    const { mode } = useThemeMode();
    const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
    const monacoRef = useRef<Monaco | null>(null);
    const errorRef = useRef<string | null>(error || null);
    const statusBarRef = useRef<HTMLDivElement | null>(null);
    const currentErrorsRef = useRef<CompileError[]>([]);

    const updateMarkers = useCallback((errorMessage: string | null) => {
      if (!editorRef.current || !monacoRef.current) return;

      const model = editorRef.current.getModel();
      if (!model) return;

      if (errorMessage) {
        const lineMatch = errorMessage.match(/line (\d+)/i);
        const line = lineMatch ? parseInt(lineMatch[1], 10) : 1;

        monacoRef.current.editor.setModelMarkers(model, "dsp-source", [
          {
            startLineNumber: line,
            startColumn: 1,
            endLineNumber: line,
            endColumn: model.getLineMaxColumn(line),
            message: errorMessage,
            severity: monacoRef.current.MarkerSeverity.Error,
          },
        ]);
      } else {
        monacoRef.current.editor.setModelMarkers(model, "dsp-source", []);
      }
    }, []);

    const updateMultipleMarkers = useCallback((errors: CompileError[]) => {
      if (!editorRef.current || !monacoRef.current) return;

      const model = editorRef.current.getModel();
      if (!model) return;

      // Store errors for hover display
      currentErrorsRef.current = errors;

      if (errors.length > 0) {
        const lineCount = model.getLineCount();
        const markers = errors
          .filter(error => error.line > 0 && error.line <= lineCount)
          .map(error => ({
            startLineNumber: error.line,
            startColumn: 1,
            endLineNumber: error.line,
            endColumn: model.getLineMaxColumn(error.line),
            message: error.message,
            severity: monacoRef.current!.MarkerSeverity.Error,
          }));
        monacoRef.current.editor.setModelMarkers(model, "dsp-source", markers);
      } else {
        monacoRef.current.editor.setModelMarkers(model, "dsp-source", []);
      }
    }, []);

    const scrollToFirstError = useCallback(() => {
      if (!editorRef.current || currentErrorsRef.current.length === 0) return;

      const firstError = currentErrorsRef.current[0];
      editorRef.current.revealLineInCenter(firstError.line);
      editorRef.current.setPosition({ lineNumber: firstError.line, column: 1 });
      editorRef.current.focus();
    }, []);

    const updateStatusBar = useCallback((status: 'compiling' | 'compiled' | 'error' | 'assembling-failed', errorCount?: number, onClickCallback?: () => void) => {
      if (!statusBarRef.current) return;

      let text = '';
      let color = '';
      let showSpinner = false;

      switch (status) {
        case 'compiling':
          text = 'Compiling...';
          color = '#569cd6';
          showSpinner = true;
          break;
        case 'compiled':
          text = 'Compiled';
          color = '#4ec9b0';
          break;
        case 'error':
          text = errorCount ? `Error (${errorCount} ${errorCount === 1 ? 'issue' : 'issues'})` : 'Error';
          color = '#f48771';
          break;
        case 'assembling-failed':
          text = errorCount ? `Assembling failed (${errorCount} ${errorCount === 1 ? 'issue' : 'issues'})` : 'Assembling failed';
          color = '#f48771';
          break;
      }

      statusBarRef.current.innerHTML = showSpinner
        ? `<span style="display: inline-flex; align-items: center; gap: 6px;"><svg width="14" height="14" viewBox="0 0 14 14" style="animation: spin 1s linear infinite;"><circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="2" fill="none" stroke-dasharray="24" stroke-dashoffset="8"/></svg>${text}</span>`
        : text;
      statusBarRef.current.style.color = color;

      // Build tooltip for errors
      if (status === 'error' && currentErrorsRef.current.length > 0) {
        const tooltip = currentErrorsRef.current
          .map(err => `Line ${err.line}: ${err.message}`)
          .join('\n');
        statusBarRef.current.setAttribute('title', tooltip);
        // Make clickable and add cursor pointer
        statusBarRef.current.style.cursor = 'pointer';
        statusBarRef.current.onclick = scrollToFirstError;
      } else if (status === 'assembling-failed' && onClickCallback) {
        const tooltip = 'Click to view assembly errors';
        statusBarRef.current.setAttribute('title', tooltip);
        statusBarRef.current.style.cursor = 'pointer';
        statusBarRef.current.onclick = onClickCallback;
      } else {
        statusBarRef.current.removeAttribute('title');
        statusBarRef.current.style.cursor = 'default';
        statusBarRef.current.onclick = null;
      }
    }, [scrollToFirstError]);

    useImperativeHandle(ref, () => ({
      layout: () => {
        editorRef.current?.layout();
      },
      setError: (newError: string | null) => {
        errorRef.current = newError;
        updateMarkers(newError);
      },
      setErrors: (errors: CompileError[]) => {
        updateMultipleMarkers(errors);
      },
      setStatus: (status: 'compiling' | 'compiled' | 'error' | 'assembling-failed', errorCount?: number, onClickCallback?: () => void) => {
        updateStatusBar(status, errorCount, onClickCallback);
      },
    }), [updateStatusBar, updateMarkers, updateMultipleMarkers]);

    const handleEditorWillMount = useCallback((monaco: Monaco) => {
      monacoRef.current = monaco;
      registerDspSourceLanguage(monaco);

      // Register hover provider for preprocessor macros
      monaco.languages.registerHoverProvider("aica-dsp-source", {
        provideHover: (model, position) => {
          const word = model.getWordAtPosition(position);
          if (!word) return null;

          const macros = getLastPreprocessedMacros();
          const macro = macros.get(word.word);

          if (macro) {
            return {
              range: new monaco.Range(
                position.lineNumber,
                word.startColumn,
                position.lineNumber,
                word.endColumn
              ),
              contents: [
                { value: `**${macro.name}**` },
                { value: `Defined on line ${macro.line}` },
                { value: `\`\`\`\n#define ${macro.name} ${macro.value}\n\`\`\`` }
              ]
            };
          }

          return null;
        }
      });
    }, []);

    const handleEditorDidMount = useCallback(
      (editor: editor.IStandaloneCodeEditor) => {
        editorRef.current = editor;
        setTimeout(() => {
          editor.layout();
          // Notify parent that editor is ready
          onEditorReady?.();
        }, 0);
      },
      [onEditorReady]
    );

    const handleChange = useCallback(
      (value: string | undefined) => {
        if (value !== undefined) {
          onChange(value);
        }
      },
      [onChange]
    );

    useEffect(() => {
      updateMarkers(errorRef.current);
    }, [updateMarkers]);

    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1, height: "100%" }}>
        <style>
          {`
            @keyframes spin {
              from { transform: rotate(0deg); }
              to { transform: rotate(360deg); }
            }
          `}
        </style>
        <Box
          sx={{
            border: 1,
            borderColor: "divider",
            borderRadius: 1,
            overflow: "hidden",
            position: "relative",
            ...(typeof height === "string" && height === "100%"
              ? { flex: 1, minHeight: 0 }
              : { height }),
          }}
        >
          <Editor
            height="100%"
            defaultLanguage="aica-dsp-source"
            language="aica-dsp-source"
            value={value}
            theme={mode === "dark" ? "aica-dsp-source-dark" : "aica-dsp-source-light"}
            onChange={handleChange}
            beforeMount={handleEditorWillMount}
            onMount={handleEditorDidMount}
            options={{
              minimap: { enabled: false },
              scrollBeyondLastLine: false,
              fontSize: 13,
              lineNumbers: "on",
              renderWhitespace: "selection",
              tabSize: 2,
              insertSpaces: true,
              automaticLayout: false,
            }}
          />
          <Box
            ref={statusBarRef}
            sx={{
              position: "absolute",
              bottom: 0,
              right: 0,
              padding: "2px 8px",
              fontSize: "11px",
              fontFamily: "monospace",
              backgroundColor: mode === "dark" ? "rgba(30, 30, 30, 0.9)" : "rgba(255, 255, 255, 0.9)",
              borderTopLeftRadius: "4px",
              zIndex: 10,
              cursor: "default",
            }}
          />
        </Box>
      </Box>
    );
  }
);

DspSourceEditorComponent.displayName = "DspSourceEditor";

export const DspSourceEditor = memo(DspSourceEditorComponent);
