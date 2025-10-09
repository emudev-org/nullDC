import { memo, useCallback, useEffect, useRef, useImperativeHandle, forwardRef } from "react";
import { Box, Typography } from "@mui/material";
import Editor from "@monaco-editor/react";
import type { Monaco } from "@monaco-editor/react";
import type { editor } from "monaco-editor";
import { useThemeMode } from "../../theme/ThemeModeProvider";
import { getLastAssemblyMacros } from "../dsp/dspUtils";

export interface CompileError {
  line: number;
  message: string;
}

export interface DspAssemblyEditorProps {
  value: string;
  onChange: (value: string) => void;
  error?: string | null;
  height?: number | string;
  readOnly?: boolean;
  onEditorReady?: () => void;
}

export interface DspAssemblyEditorRef {
  layout: () => void;
  setError: (error: string | null) => void;
  setErrors: (errors: CompileError[]) => void;
  setStatus: (status: 'assembling' | 'assembled' | 'error', errorCount?: number) => void;
}

const registerDspLanguage = (monaco: Monaco) => {
  // Register language
  monaco.languages.register({ id: "aica-dsp-asm" });

  // Define language tokens
  monaco.languages.setMonarchTokensProvider("aica-dsp-asm", {
    tokenizer: {
      root: [
        // Comments
        [/#.*$/, "comment"],

        // Register types
        [/\b(COEF|MADRS|MEMS_L|MEMS_H|MPRO)\b/, "keyword"],

        // Instruction fields
        [
          /\b(TRA|TWT|TWA|XSEL|YSEL|IRA|IWT|IWA|TABLE|MWT|MRD|EWT|EWA|ADRL|FRCL|SHIFT|YRL|NEGB|ZERO|BSEL|NOFL|MASA|ADREB|NXADR)\b/,
          "type",
        ],

        // Numbers
        [/\b\d+\b/, "number"],
        [/\b0x[0-9a-fA-F]+\b/, "number"],

        // Operators
        [/[=:\[\]]/, "operator"],
      ],
    },
  });

  // Define theme colors
  monaco.editor.defineTheme("aica-dsp-asm-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "comment", foreground: "6A9955" },
      { token: "keyword", foreground: "569CD6", fontStyle: "bold" },
      { token: "type", foreground: "4EC9B0" },
      { token: "number", foreground: "B5CEA8" },
      { token: "operator", foreground: "D4D4D4" },
    ],
    colors: {},
  });

  monaco.editor.defineTheme("aica-dsp-asm-light", {
    base: "vs",
    inherit: true,
    rules: [
      { token: "comment", foreground: "008000" },
      { token: "keyword", foreground: "0000FF", fontStyle: "bold" },
      { token: "type", foreground: "267F99" },
      { token: "number", foreground: "098658" },
      { token: "operator", foreground: "000000" },
    ],
    colors: {},
  });
};

export const DspAssemblyEditor = memo(
  forwardRef<DspAssemblyEditorRef, DspAssemblyEditorProps>(
    ({ value, onChange, error, height = 250, readOnly = false, onEditorReady }, ref) => {
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
          // Try to parse line number from error message (e.g., "Error at line 5:")
          const lineMatch = errorMessage.match(/line (\d+)/i);
          const line = lineMatch ? parseInt(lineMatch[1], 10) : 1;

          monacoRef.current.editor.setModelMarkers(model, "dsp-assembly", [
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
          // Clear markers
          monacoRef.current.editor.setModelMarkers(model, "dsp-assembly", []);
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
          monacoRef.current.editor.setModelMarkers(model, "dsp-assembly", markers);
        } else {
          monacoRef.current.editor.setModelMarkers(model, "dsp-assembly", []);
        }
      }, []);

      const scrollToFirstError = useCallback(() => {
        if (!editorRef.current || currentErrorsRef.current.length === 0) return;

        const firstError = currentErrorsRef.current[0];
        editorRef.current.revealLineInCenter(firstError.line);
        editorRef.current.setPosition({ lineNumber: firstError.line, column: 1 });
        editorRef.current.focus();
      }, []);

      const updateStatusBar = useCallback((status: 'assembling' | 'assembled' | 'error', errorCount?: number) => {
        if (!statusBarRef.current) return;

        let text = '';
        let color = '';
        let showSpinner = false;

        switch (status) {
          case 'assembling':
            text = 'Assembling...';
            color = '#569cd6';
            showSpinner = true;
            break;
          case 'assembled':
            text = 'Assembled';
            color = '#4ec9b0';
            break;
          case 'error':
            text = errorCount ? `Error (${errorCount} ${errorCount === 1 ? 'issue' : 'issues'})` : 'Error';
            color = '#f48771';
            break;
        }

        statusBarRef.current.innerHTML = showSpinner
          ? `<span style="display: inline-flex; align-items: center; gap: 6px;"><svg width="14" height="14" viewBox="0 0 14 14" style="animation: spin 1s linear infinite;"><circle cx="7" cy="7" r="5" stroke="currentColor" stroke-width="2" fill="none" stroke-dasharray="24" stroke-dashoffset="8"/></svg>${text}</span>`
          : text;
        statusBarRef.current.style.color = color;

        // Build tooltip for errors
        if (status === 'error') {
          if (currentErrorsRef.current.length > 0) {
            const tooltip = currentErrorsRef.current
              .map(err => `Line ${err.line}: ${err.message}`)
              .join('\n');
            statusBarRef.current.setAttribute('title', tooltip);
            // Make clickable and add cursor pointer
            statusBarRef.current.style.cursor = 'pointer';
            statusBarRef.current.onclick = scrollToFirstError;
          } else {
            statusBarRef.current.removeAttribute('title');
            statusBarRef.current.style.cursor = 'default';
            statusBarRef.current.onclick = null;
          }
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
        setStatus: (status: 'assembling' | 'assembled' | 'error', errorCount?: number) => {
          updateStatusBar(status, errorCount);
        },
      }));

      const handleEditorDidMount = useCallback((editor: editor.IStandaloneCodeEditor) => {
        editorRef.current = editor;
        // Trigger initial layout after mount
        setTimeout(() => {
          editor.layout();
          // Notify parent that editor is ready
          onEditorReady?.();
        }, 0);
      }, [onEditorReady]);

      const handleEditorWillMount = useCallback((monaco: Monaco) => {
        monacoRef.current = monaco;
        registerDspLanguage(monaco);

        // Register hover provider for preprocessor macros
        monaco.languages.registerHoverProvider("aica-dsp-asm", {
          provideHover: (model, position) => {
            const word = model.getWordAtPosition(position);
            if (!word) return null;

            const macros = getLastAssemblyMacros();
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

      const handleEditorChange = useCallback(
        (newValue: string | undefined) => {
          onChange(newValue ?? "");
        },
        [onChange]
      );

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
              language="aica-dsp-asm"
              theme={mode === "dark" ? "aica-dsp-asm-dark" : "aica-dsp-asm-light"}
              value={value}
              onChange={handleEditorChange}
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
                readOnly: readOnly,
                readOnlyMessage: { value: "This editor is read-only" },
                // Show error decorations even when read-only
                renderValidationDecorations: "on",
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
  )
);

DspAssemblyEditor.displayName = "DspAssemblyEditor";
