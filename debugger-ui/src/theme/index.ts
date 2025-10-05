import type { PaletteMode } from "@mui/material";
import { createTheme } from "@mui/material/styles";

const commonTheme = {
  typography: {
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    h6: {
      letterSpacing: 1.6,
    },
    body2: {
      fontSize: 13,
    },
  },
  components: {
    MuiAppBar: {
      styleOverrides: {
        root: {
          backgroundImage: "none",
        },
      },
    },
    MuiPaper: {
      styleOverrides: {
        root: {
          backgroundImage: "none",
        },
      },
    },
    MuiTabs: {
      styleOverrides: {
        flexContainer: {
          gap: 8,
        },
      },
    },
    MuiTab: {
      styleOverrides: {
        root: {
          textTransform: "none",
          minHeight: 32,
        },
      },
    },
  },
} as const;

const darkPalette = {
  mode: "dark" as PaletteMode,
  primary: { main: "#4fc3f7" },
  secondary: { main: "#ce93d8" },
  background: { default: "#0f111a", paper: "#171a27" },
  divider: "rgba(255, 255, 255, 0.08)",
};

const lightPalette = {
  mode: "light" as PaletteMode,
  primary: { main: "#1976d2" },
  secondary: { main: "#7b1fa2" },
  background: { default: "#f5f7ff", paper: "#ffffff" },
  divider: "rgba(0, 0, 0, 0.12)",
};

export const createDebuggerTheme = (mode: PaletteMode) =>
  createTheme({
    palette: mode === "dark" ? darkPalette : lightPalette,
    ...commonTheme,
  });
