import type { ReactNode } from "react";
import DescriptionIcon from "@mui/icons-material/Description";
import DeveloperBoardIcon from "@mui/icons-material/DeveloperBoard";
import EventIcon from "@mui/icons-material/Event";
import BugReportIcon from "@mui/icons-material/BugReport";
import CodeIcon from "@mui/icons-material/Code";
import MemoryIcon from "@mui/icons-material/Memory";
import AccountTreeIcon from "@mui/icons-material/AccountTree";
import VisibilityIcon from "@mui/icons-material/Visibility";
import FormatListNumberedIcon from "@mui/icons-material/FormatListNumbered";
import GraphicEqIcon from "@mui/icons-material/GraphicEq";
import ViewInArIcon from "@mui/icons-material/ViewInAr";
import SettingsIcon from "@mui/icons-material/Settings";
import ScienceIcon from "@mui/icons-material/Science";
import { PANEL_IDS, type PanelId } from "../../lib/debuggerSchema";

export type PanelRegistryEntry = {
  id: PanelId;
  name: string;
  icon: ReactNode;
};

export const PANEL_REGISTRY: Record<PanelId, PanelRegistryEntry> = {
  [PANEL_IDS.DOCUMENTATION]: {
    id: PANEL_IDS.DOCUMENTATION,
    name: "Documentation",
    icon: <DescriptionIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_SIM]: {
    id: PANEL_IDS.SH4_SIM,
    name: "SH4: Simulator",
    icon: <ScienceIcon fontSize="small" />,
  },
  [PANEL_IDS.EVENTS]: {
    id: PANEL_IDS.EVENTS,
    name: "Events: Log",
    icon: <EventIcon fontSize="small" />,
  },
  [PANEL_IDS.EVENTS_BREAKPOINTS]: {
    id: PANEL_IDS.EVENTS_BREAKPOINTS,
    name: "Events: Breakpoints",
    icon: <BugReportIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_DISASSEMBLY]: {
    id: PANEL_IDS.SH4_DISASSEMBLY,
    name: "SH4: Disassembly",
    icon: <CodeIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_MEMORY]: {
    id: PANEL_IDS.SH4_MEMORY,
    name: "SH4: Memory",
    icon: <MemoryIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_BREAKPOINTS]: {
    id: PANEL_IDS.SH4_BREAKPOINTS,
    name: "SH4: Breakpoints",
    icon: <BugReportIcon fontSize="small" />,
  },
  [PANEL_IDS.ARM7_DISASSEMBLY]: {
    id: PANEL_IDS.ARM7_DISASSEMBLY,
    name: "ARM7: Disassembly",
    icon: <CodeIcon fontSize="small" />,
  },
  [PANEL_IDS.ARM7_MEMORY]: {
    id: PANEL_IDS.ARM7_MEMORY,
    name: "ARM7: Memory",
    icon: <MemoryIcon fontSize="small" />,
  },
  [PANEL_IDS.ARM7_BREAKPOINTS]: {
    id: PANEL_IDS.ARM7_BREAKPOINTS,
    name: "ARM7: Breakpoints",
    icon: <BugReportIcon fontSize="small" />,
  },
  [PANEL_IDS.TA]: {
    id: PANEL_IDS.TA,
    name: "TA",
    icon: <ViewInArIcon fontSize="small" />,
  },
  [PANEL_IDS.CORE]: {
    id: PANEL_IDS.CORE,
    name: "CORE",
    icon: <SettingsIcon fontSize="small" />,
  },
  [PANEL_IDS.AICA]: {
    id: PANEL_IDS.AICA,
    name: "AICA",
    icon: <GraphicEqIcon fontSize="small" />,
  },
  [PANEL_IDS.DSP_DISASSEMBLY]: {
    id: PANEL_IDS.DSP_DISASSEMBLY,
    name: "DSP: Disassembly",
    icon: <CodeIcon fontSize="small" />,
  },
  [PANEL_IDS.DSP_BREAKPOINTS]: {
    id: PANEL_IDS.DSP_BREAKPOINTS,
    name: "DSP: Breakpoints",
    icon: <BugReportIcon fontSize="small" />,
  },
  [PANEL_IDS.DSP_PLAYGROUND]: {
    id: PANEL_IDS.DSP_PLAYGROUND,
    name: "DSP: Playground",
    icon: <DeveloperBoardIcon fontSize="small" />,
  },
  [PANEL_IDS.DEVICE_TREE]: {
    id: PANEL_IDS.DEVICE_TREE,
    name: "Device Tree",
    icon: <AccountTreeIcon fontSize="small" />,
  },
  [PANEL_IDS.WATCHES]: {
    id: PANEL_IDS.WATCHES,
    name: "Watches",
    icon: <VisibilityIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_CALLSTACK]: {
    id: PANEL_IDS.SH4_CALLSTACK,
    name: "SH4: Callstack",
    icon: <FormatListNumberedIcon fontSize="small" />,
  },
  [PANEL_IDS.ARM7_CALLSTACK]: {
    id: PANEL_IDS.ARM7_CALLSTACK,
    name: "ARM7: Callstack",
    icon: <FormatListNumberedIcon fontSize="small" />,
  },
};
