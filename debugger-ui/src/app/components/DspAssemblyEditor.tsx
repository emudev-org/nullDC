import { memo, useCallback, useEffect, useRef, useImperativeHandle, forwardRef } from "react";
import { Box, Typography } from "@mui/material";
import Editor from "@monaco-editor/react";
import type { Monaco } from "@monaco-editor/react";
import type { editor } from "monaco-editor";
import { useThemeMode } from "../../theme/ThemeModeProvider";

export interface DspAssemblyEditorProps {
  value: string;
  onChange: (value: string) => void;
  error?: string | null;
  height?: number | string;
}

export interface DspAssemblyEditorRef {
  layout: () => void;
  setError: (error: string | null) => void;
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
    ({ value, onChange, error, height = 250 }, ref) => {
      const { mode } = useThemeMode();
      const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
      const monacoRef = useRef<Monaco | null>(null);
      const errorRef = useRef<string | null>(error || null);

      useImperativeHandle(ref, () => ({
        layout: () => {
          editorRef.current?.layout();
        },
        setError: (newError: string | null) => {
          errorRef.current = newError;
          updateMarkers(newError);
        },
      }));

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

      const handleEditorDidMount = useCallback((editor: editor.IStandaloneCodeEditor) => {
        editorRef.current = editor;
        // Trigger initial layout after mount
        setTimeout(() => {
          editor.layout();
        }, 0);
      }, []);

      const handleEditorWillMount = useCallback((monaco: Monaco) => {
        monacoRef.current = monaco;
        registerDspLanguage(monaco);
      }, []);

      const handleEditorChange = useCallback(
        (newValue: string | undefined) => {
          onChange(newValue ?? "");
        },
        [onChange]
      );

      return (
        <Box sx={{ display: "flex", flexDirection: "column", gap: 1, height: "100%" }}>
          <Typography variant="subtitle2" color="text.secondary">
            DSP Assembly
          </Typography>
          <Box
            sx={{
              border: 1,
              borderColor: "divider",
              borderRadius: 1,
              overflow: "hidden",
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
              }}
            />
          </Box>
        </Box>
      );
    }
  )
);

DspAssemblyEditor.displayName = "DspAssemblyEditor";
