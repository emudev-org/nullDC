import { Box, Button, Card, CardActionArea, CardContent, Container, Stack, Typography } from "@mui/material";
import { useNavigate } from "react-router-dom";

const quickActions = [
  {
    title: "Debug SH4 Code",
    description: "Disassemble, set breakpoints, and step through SH4 instructions with live pipeline data.",
    to: "/sh4-disassembly",
  },
  {
    title: "Debug ARM Code",
    description: "Inspect ARM7 execution state and control breakpoints for the AICA co-processor.",
    to: "/arm7-disassembly",
  },
  {
    title: "Analyze TA Logs",
    description: "Review tile accelerator events and frame lists to diagnose rendering issues.",
    to: "/ta",
  },
  {
    title: "Analyze CORE Logs",
    description: "Inspect PowerVR CORE metrics and state changes across frames.",
    to: "/core",
  },
  {
    title: "Debug AICA & DSP",
    description: "Monitor AICA channels, DSP state, and waveforms to diagnose audio paths.",
    to: "/aica",
  },
  {
    title: "Simulate SH4 Pipeline",
    description: "Experiment with scheduling patterns and visualize pipeline hazards.",
    to: "/sh4-sim",
  },
  {
    title: "DSP Playground",
    description: "Author and preview DSP effects with real-time waveform inspection.",
    to: "/dsp-playground",
  },
  {
    title: "About",
    description: "View emulator version and build information retrieved from the connected emulator.",
    to: "/about",
  },
];

export const HomePane = () => {
  const navigate = useNavigate();

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
              Welcome to the nullDC Debugger
            </Typography>
            <Typography variant="h6" sx={{ color: "text.secondary", maxWidth: 720 }}>
              Dive into Dreamcast hardware debugging with curated entry points for CPU, GPU, and audio analysis.
              Choose a workspace to get started.
            </Typography>
            <Stack direction={{ xs: "column", sm: "row" }} spacing={2} sx={{ mt: 2 }}>
              <Button variant="contained" size="large" onClick={() => navigate("/events")}>View Event Log</Button>
              <Button variant="outlined" size="large" onClick={() => navigate("/device-tree")}>Open Device Tree</Button>
            </Stack>
          </Stack>

          <Box
            sx={{
              width: "100%",
              display: "grid",
              gap: 3,
              gridTemplateColumns: {
                xs: "1fr",
                sm: "repeat(2, minmax(0, 1fr))",
                md: "repeat(3, minmax(0, 1fr))",
              },
            }}
          >
            {quickActions.map((action) => (
              <Card elevation={4} sx={{ borderRadius: 3, height: "100%" }} key={action.title}>
                <CardActionArea
                  onClick={() => navigate(action.to)}
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
      </Container>
    </Box>
  );
};
