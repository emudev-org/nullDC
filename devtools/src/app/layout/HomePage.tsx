import { useMemo } from "react";
import { Box, Button, Card, CardActionArea, CardContent, Container, Stack, Typography } from "@mui/material";
import { useNavigate } from "react-router-dom";
import { AboutDialog } from "./AboutDialog";
import { useAboutModal } from "./useAboutModal";

export const HomePage = () => {
  const navigate = useNavigate();
  const { open: aboutOpen, show: showAbout, hide: hideAbout } = useAboutModal();

  const debuggerActions = useMemo(
    () => [
      {
        title: "SH4 Debugger",
        description: "Disassemble, set breakpoints, and step through SH4 instructions with live pipeline data.",
        action: () => navigate("/workspace/sh4-debugger"),
      },
      {
        title: "ARM7 Debugger",
        description: "Inspect ARM7 execution state and control breakpoints for the AICA co-processor.",
        action: () => navigate("/workspace/arm7-debugger"),
      },
      {
        title: "DSP Debugger",
        description: "Inspect DSP state, disassembly, and breakpoints for audio processing.",
        action: () => navigate("/workspace/dsp-debugger"),
      },
      {
        title: "Custom Debugger",
        description: "Full debugger workspace with all panels for comprehensive system debugging.",
        action: () => navigate("/workspace/custom-debugger"),
      },
    ],
    [navigate],
  );

  const toolsActions = useMemo(
    () => [
      {
        title: "SH4 Simulator",
        description: "Experiment with scheduling patterns and visualize pipeline hazards.",
        action: () => navigate("/workspace/sh4-sim"),
      },
      {
        title: "CLX2/TA Log Analyzer",
        description: "Review tile accelerator primitives and generated CORE lists to diagnose frame submission issues.",
        action: () => navigate("/workspace/clx2-ta-log-analyzer"),
      },
      {
        title: "CLX2/CORE Log Analyzer",
        description: "Inspect PowerVR CORE primitives, buffers and state changes in a frame to diagnose frame rendering artifacts.",
        action: () => navigate("/workspace/clx2-core-log-analyzer"),
      },
      {
        title: "DSP Playground",
        description: "Author, debug and preview DSP effects with real-time waveform inspection.",
        action: () => navigate("/workspace/dsp-playground"),
      },
    ],
    [navigate],
  );

  const othersActions = useMemo(
    () => [
      {
        title: "Documentation",
        description: "Learn the debugger workflows, mocked APIs, and sharing features in detail.",
        action: () => navigate("/docs"),
      },
      {
        title: "About",
        description: "View debugger version and build information retrieved from the connected emulator.",
        action: showAbout,
      },
    ],
    [navigate, showAbout],
  );

  return (
    <Box
      sx={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        background: (theme) =>
          theme.palette.mode === "dark"
            ? "linear-gradient(135deg, #111827 0%, #1f2937 100%)"
            : "linear-gradient(135deg, #f8fafc 0%, #eef2ff 100%)",
        py: 12,
      }}
    >
      <Container maxWidth="lg">
        <Stack spacing={6} alignItems="center">
          <Stack spacing={2} alignItems="center" textAlign="center">
            <Typography variant="h3" fontWeight={700} sx={{ color: "text.primary" }}>
              Welcome to the nullDC DevTools
            </Typography>
            <Typography variant="h6" sx={{ color: "text.secondary", maxWidth: 720 }}>
              Dive into Dreamcast with curated entry points for CPU, GPU, and audio analysis, as well as sh4 simulator and dsp authoring tools.
              Choose a task to get started.
            </Typography>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2} sx={{ mt: 2 }}>
              <Button variant="contained" size="large" onClick={() => navigate("/workspace/custom-debugger/events")}>View Event Log</Button>
              <Button variant="outlined" size="large" onClick={() => navigate("/workspace/custom-debugger/device-tree")}>Open Device Tree</Button>
            </Stack>
          </Stack>

          <Stack spacing={4} sx={{ width: "100%" }}>
            <Stack spacing={2}>
              <Typography variant="h5" fontWeight={700} sx={{ color: "text.primary", textAlign: "center" }}>
                Debugger
              </Typography>
              <Box
                sx={{
                  display: "grid",
                  gap: 3,
                  gridTemplateColumns: {
                    xs: "1fr",
                    sm: "repeat(2, minmax(0, 1fr))",
                    md: "repeat(3, minmax(0, 1fr))",
                  },
                }}
              >
                {debuggerActions.map((action) => (
                  <Card elevation={4} sx={{ borderRadius: 3, height: "100%" }} key={action.title}>
                    <CardActionArea
                      onClick={action.action}
                      sx={{ height: "100%", display: "flex", alignItems: "stretch" }}
                    >
                      <CardContent sx={{ display: "flex", flexDirection: "column", gap: 1.5 }}>
                        <Typography variant="h6" fontWeight={600}>
                          {action.title}
                        </Typography>
                        <Typography variant="body2" color="text.secondary">
                          {action.description}
                        </Typography>
                        <Box sx={{ flexGrow: 1 }} />
                        <Typography variant="button" sx={{ color: "primary.main" }}>
                          Open →
                        </Typography>
                      </CardContent>
                    </CardActionArea>
                  </Card>
                ))}
              </Box>
            </Stack>

            <Stack spacing={2}>
              <Typography variant="h5" fontWeight={700} sx={{ color: "text.primary", textAlign: "center" }}>
                Tools
              </Typography>
              <Box
                sx={{
                  display: "grid",
                  gap: 3,
                  gridTemplateColumns: {
                    xs: "1fr",
                    sm: "repeat(2, minmax(0, 1fr))",
                    md: "repeat(3, minmax(0, 1fr))",
                  },
                }}
              >
                {toolsActions.map((action) => (
                  <Card elevation={4} sx={{ borderRadius: 3, height: "100%" }} key={action.title}>
                    <CardActionArea
                      onClick={action.action}
                      sx={{ height: "100%", display: "flex", alignItems: "stretch" }}
                    >
                      <CardContent sx={{ display: "flex", flexDirection: "column", gap: 1.5 }}>
                        <Typography variant="h6" fontWeight={600}>
                          {action.title}
                        </Typography>
                        <Typography variant="body2" color="text.secondary">
                          {action.description}
                        </Typography>
                        <Box sx={{ flexGrow: 1 }} />
                        <Typography variant="button" sx={{ color: "primary.main" }}>
                          Open →
                        </Typography>
                      </CardContent>
                    </CardActionArea>
                  </Card>
                ))}
              </Box>
            </Stack>

            <Stack spacing={2}>
              <Typography variant="h5" fontWeight={700} sx={{ color: "text.primary", textAlign: "center" }}>
                Others
              </Typography>
              <Box
                sx={{
                  display: "grid",
                  gap: 3,
                  gridTemplateColumns: {
                    xs: "1fr",
                    sm: "repeat(2, minmax(0, 1fr))",
                    md: "repeat(3, minmax(0, 1fr))",
                  },
                }}
              >
                {othersActions.map((action) => (
                  <Card elevation={4} sx={{ borderRadius: 3, height: "100%" }} key={action.title}>
                    <CardActionArea
                      onClick={action.action}
                      sx={{ height: "100%", display: "flex", alignItems: "stretch" }}
                    >
                      <CardContent sx={{ display: "flex", flexDirection: "column", gap: 1.5 }}>
                        <Typography variant="h6" fontWeight={600}>
                          {action.title}
                        </Typography>
                        <Typography variant="body2" color="text.secondary">
                          {action.description}
                        </Typography>
                        <Box sx={{ flexGrow: 1 }} />
                        <Typography variant="button" sx={{ color: "primary.main" }}>
                          Open →
                        </Typography>
                      </CardContent>
                    </CardActionArea>
                  </Card>
                ))}
              </Box>
            </Stack>
          </Stack>
        </Stack>
      </Container>
      <AboutDialog open={aboutOpen} onClose={hideAbout} />
    </Box>
  );
};
