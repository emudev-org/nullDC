import { memo, useCallback } from "react";
import { Box, Typography } from "@mui/material";
import Editor from "@monaco-editor/react";
import type { Monaco } from "@monaco-editor/react";
import { useThemeMode } from "../../theme/ThemeModeProvider";

export interface DspAssemblyEditorProps {
  value: string;
  onChange: (value: string) => void;
  error?: string | null;
  height?: number | string;
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

export const DspAssemblyEditor = memo(({ value, onChange, error, height = 250 }: DspAssemblyEditorProps) => {
  const { mode } = useThemeMode();

  const handleEditorWillMount = useCallback((monaco: Monaco) => {
    registerDspLanguage(monaco);
  }, []);

  const handleEditorChange = useCallback(
    (newValue: string | undefined) => {
      onChange(newValue ?? "");
    },
    [onChange]
  );

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
      <Typography variant="subtitle2" color="text.secondary">
        DSP Assembly
      </Typography>
      <Box
        sx={{
          border: 1,
          borderColor: error ? "error.main" : "divider",
          borderRadius: 1,
          overflow: "hidden",
        }}
      >
        <Editor
          height={height}
          language="aica-dsp-asm"
          theme={mode === "dark" ? "aica-dsp-asm-dark" : "aica-dsp-asm-light"}
          value={value}
          onChange={handleEditorChange}
          beforeMount={handleEditorWillMount}
          options={{
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            fontSize: 13,
            lineNumbers: "on",
            renderWhitespace: "selection",
            tabSize: 2,
            insertSpaces: true,
            automaticLayout: true,
          }}
        />
      </Box>
    </Box>
  );
});

DspAssemblyEditor.displayName = "DspAssemblyEditor";
