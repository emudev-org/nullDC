import { type ReactNode } from "react";
import { AppBar, Box, Container, Stack, Typography } from "@mui/material";
import { TopNav } from "./TopNav";
import { AboutDialog } from "./AboutDialog";
import { useAboutModal } from "./useAboutModal";

type AnchorHeadingProps = {
  id: string;
  level: 1 | 2 | 3;
  title: string;
  children?: ReactNode;
};

const AnchorHeading = ({ id, level, title, children }: AnchorHeadingProps) => {
  const HeadingTag = `h${level}` as const;

  return (
    <Stack id={id} spacing={1} sx={{ position: "relative" }}>
      <Typography component={HeadingTag} variant={level === 1 ? "h4" : level === 2 ? "h5" : "h6"} sx={{ fontWeight: 700 }}>
        {title}
      </Typography>
      {children}
    </Stack>
  );
};

const documentationSections = [
  {
    id: "overview",
    title: "Overview",
    level: 1 as const,
    content: [
      "The nullDC debugger UI provides a modern, web-based front end for the emulator's introspection APIs.",
      "It combines Dreamcast-specific tooling with collaborative workflows inspired by the original vision for the debugger.",
    ],
  },
  {
    id: "getting-started",
    title: "Getting Started",
    level: 1 as const,
    content: [
      "Use the mock debugger during development to explore the UI before wiring it to a live emulator.",
      "Keep one terminal running the UI (`npm run dev`) and another running the mock server (`npm run dev:mock`) so you always see live updates.",
    ],
    subsections: [
      {
        id: "launching-ui",
        title: "Launching the UI",
        level: 2 as const,
        items: [
          "Run `npm install` once, then `npm run dev` to start the Vite dev server on http://localhost:5173.",
          "Run `npm run dev:mock` to serve the UI from Express and expose the JSON-RPC/WebSocket endpoint at `/ws`.",
          "For a standalone mock without Vite middleware, use `npm run mock:start` and open the logged URL.",
        ],
      },
      {
        id: "connecting-emulator",
        title: "Connecting to the Emulator",
        level: 2 as const,
        items: [
          "The UI auto-connects to the host that served it. Adjust `VITE_WS_PATH` or related env vars when deploying.",
          "Use the header status icon to confirm connection state and manually reconnect if needed.",
          "When working with real hardware, ensure the emulator exposes the same JSON-RPC schema as the mock server.",
        ],
      },
    ],
  },
  {
    id: "core-workspaces",
    title: "Core Workspaces",
    level: 1 as const,
    content: [
      "Start with the event log to get a heartbeat of the system, then drill into disassembly, memory, or audio tools as needed.",
      "Most panels support hover linking and explainers—trust the tooltips and inspector panels to understand dependencies.",
    ],
    subsections: [
      {
        id: "events-log",
        title: "Events & Frame Log",
        level: 2 as const,
        items: [
          "Stream live notifications from SH4, TA, CORE, and audio subsystems.",
          "Use highlight crosshairs and hover relations to trace data hazards across cycles.",
          "Print or share the log from the panel actions when collaborating or filing regression notes.",
        ],
      },
      {
        id: "disassembly-tools",
        title: "Disassembly & Callstacks",
        level: 2 as const,
        items: [
          "Inspect SH4, ARM7, and DSP pipelines with synchronized breakpoints.",
          "Toggle column overlays to visualize stalls, locks, and fully utilized stages.",
          "Callstack panels expose recent frames for each core so you can follow frame-to-frame transitions.",
        ],
      },
      {
        id: "memory-and-device-tree",
        title: "Memory & Device Tree",
        level: 2 as const,
        items: [
          "Device Tree panel surfaces every Dreamcast subsystem with register snapshots.",
          "Watches sync to the backend via JSON-RPC so you can annotate critical paths.",
          "Memory viewers support multiple targets (SH4, ARM7) and honor UTLB lookups when available.",
        ],
      },
      {
        id: "aica-and-dsp",
        title: "AICA & DSP",
        level: 2 as const,
        items: [
          "Audio panel visualizes oscillator channels, DSP accumulators, and waveform streams.",
          "DSP Playground is a standalone tool accessible from the home page for authoring effect chains with live previews.",
          "Breakpoint filters highlight AICA- and DSP-related events to understand audio frame scheduling.",
        ],
      },
    ],
  },
  {
    id: "sharing-and-simulation",
    title: "Sharing & Simulation",
    level: 1 as const,
    content: [],
    subsections: [
      {
        id: "source-sharing",
        title: "Source Sharing",
        level: 2 as const,
        items: [
          "Share SH4 pipeline snippets via the integrated Monaco editor; links embed the compressed source.",
          "Printable layouts allow exporting instruction flows for design reviews.",
        ],
      },
      {
        id: "sh4-simulator",
        title: "SH4 Simulator",
        level: 2 as const,
        items: [
          "SH4 Simulator is a standalone tool accessible from the home page.",
          "Visualize execution hazards by feeding assembly directly into the simulator.",
          "Hovering over cells reveals stall explanations and dependency highlights.",
          "Horizontal scroll container keeps the editor static while exploring wide tables.",
        ],
      },
    ],
  },
  {
    id: "api-and-extensibility",
    title: "API & Extensibility",
    level: 1 as const,
    content: [
      "All panels consume the JSON-RPC schema defined in debuggerSchema.ts, mirroring the emulator services.",
      "The mock server included in the repo emulates register streams, watches, breakpoints, and waveform data so the UI can be developed offline.",
      "Version metadata in the About dialog surfaces both emulator and UI git revisions for reproducibility.",
    ],
  },
];

const renderSectionContent = (section: typeof documentationSections[number]) => {
  return (
    <Stack spacing={section.subsections ? 3 : 2}>
      {section.content?.map((paragraph) => (
        <Typography key={paragraph} variant="body1" color="text.secondary">
          {paragraph}
        </Typography>
      ))}
      {section.subsections?.map((sub) => (
        <AnchorHeading key={sub.id} id={sub.id} level={sub.level} title={sub.title}>
          <Stack spacing={1.25}>
            {sub.items.map((item) => (
              <Stack key={item} direction="row" spacing={1.5} alignItems="flex-start">
                <Typography variant="body2" color="primary" sx={{ mt: "3px" }}>
                  •
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  {item}
                </Typography>
              </Stack>
            ))}
          </Stack>
        </AnchorHeading>
      ))}
    </Stack>
  );
};

export const DocsPage = () => {
  const { open: aboutOpen, show: showAbout, hide: hideAbout } = useAboutModal();

  return (
    <Box sx={{ minHeight: "100vh", display: "flex", flexDirection: "column" }}>
      <AppBar position="static" elevation={1} color="default">
        <TopNav
          onAboutClick={showAbout}
          currentPage="docs"
        />
      </AppBar>
      <Container maxWidth="lg" component="main" sx={{ py: 6, flex: 1 }}>
        <Stack spacing={4}>
          {documentationSections.map((section) => (
            <AnchorHeading key={section.id} id={section.id} level={section.level} title={section.title}>
              {renderSectionContent(section)}
            </AnchorHeading>
          ))}
        </Stack>
      </Container>
      <Box component="footer" sx={{ borderTop: "1px solid", borderColor: "divider", py: 2 }}>
        <Container maxWidth="lg">
          <Typography variant="caption" color="text.secondary">
            Documentation reflects mock server capabilities and real hardware integration described in IDEA.md.
          </Typography>
        </Container>
      </Box>
      <AboutDialog open={aboutOpen} onClose={hideAbout} />
    </Box>
  );
};
