import { createTheme } from "@mui/material/styles";

export const debuggerTheme = createTheme({
  palette: {
    mode: "dark",
    primary: {
      main: "#4fc3f7",
    },
    secondary: {
      main: "#ce93d8",
    },
    background: {
      default: "#0f111a",
      paper: "#171a27",
    },
    divider: "rgba(255, 255, 255, 0.08)",
  },
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
});
