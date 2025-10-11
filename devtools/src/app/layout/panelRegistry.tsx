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
import ImageIcon from "@mui/icons-material/Image";
import StorageIcon from "@mui/icons-material/Storage";
import CableIcon from "@mui/icons-material/Cable";
import PowerIcon from "@mui/icons-material/Power";
import RouterIcon from "@mui/icons-material/Router";
import NotificationsIcon from "@mui/icons-material/Notifications";
import AccessTimeIcon from "@mui/icons-material/AccessTime";
import SerialIcon from "@mui/icons-material/SettingsInputAntenna";
import TimerIcon from "@mui/icons-material/Timer";
import PauseCircleIcon from "@mui/icons-material/PauseCircle";
import LayersIcon from "@mui/icons-material/Layers";
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
    icon: <BugReportIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_MEMORY]: {
    id: PANEL_IDS.SH4_MEMORY,
    name: "SH4: Memory",
    icon: <StorageIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_BREAKPOINTS]: {
    id: PANEL_IDS.SH4_BREAKPOINTS,
    name: "SH4: Breakpoints",
    icon: <CodeIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_BSC_REGISTERS]: {
    id: PANEL_IDS.SH4_BSC_REGISTERS,
    name: "SH4: BSC Registers",
    icon: <DeveloperBoardIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_CCN_REGISTERS]: {
    id: PANEL_IDS.SH4_CCN_REGISTERS,
    name: "SH4: CCN Registers",
    icon: <DeveloperBoardIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_CPG_REGISTERS]: {
    id: PANEL_IDS.SH4_CPG_REGISTERS,
    name: "SH4: CPG Registers",
    icon: <DeveloperBoardIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_DMAC_REGISTERS]: {
    id: PANEL_IDS.SH4_DMAC_REGISTERS,
    name: "SH4: DMAC Registers",
    icon: <DeveloperBoardIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_INTC_REGISTERS]: {
    id: PANEL_IDS.SH4_INTC_REGISTERS,
    name: "SH4: INTC Registers",
    icon: <DeveloperBoardIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_RTC_REGISTERS]: {
    id: PANEL_IDS.SH4_RTC_REGISTERS,
    name: "SH4: RTC Registers",
    icon: <DeveloperBoardIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_SCI_REGISTERS]: {
    id: PANEL_IDS.SH4_SCI_REGISTERS,
    name: "SH4: SCI Registers",
    icon: <DeveloperBoardIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_SCIF_REGISTERS]: {
    id: PANEL_IDS.SH4_SCIF_REGISTERS,
    name: "SH4: SCIF Registers",
    icon: <DeveloperBoardIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_TMU_REGISTERS]: {
    id: PANEL_IDS.SH4_TMU_REGISTERS,
    name: "SH4: TMU Registers",
    icon: <DeveloperBoardIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_UBC_REGISTERS]: {
    id: PANEL_IDS.SH4_UBC_REGISTERS,
    name: "SH4: UBC Registers",
    icon: <DeveloperBoardIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_SQ_CONTENTS]: {
    id: PANEL_IDS.SH4_SQ_CONTENTS,
    name: "SH4: Store Queues",
    icon: <StorageIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_ICACHE_CONTENTS]: {
    id: PANEL_IDS.SH4_ICACHE_CONTENTS,
    name: "SH4: ICACHE",
    icon: <LayersIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_OCACHE_CONTENTS]: {
    id: PANEL_IDS.SH4_OCACHE_CONTENTS,
    name: "SH4: OCACHE",
    icon: <LayersIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_OCRAM_CONTENTS]: {
    id: PANEL_IDS.SH4_OCRAM_CONTENTS,
    name: "SH4: OC-RAM",
    icon: <StorageIcon fontSize="small" />,
  },
  [PANEL_IDS.SH4_TLB_CONTENTS]: {
    id: PANEL_IDS.SH4_TLB_CONTENTS,
    name: "SH4: TLB",
    icon: <StorageIcon fontSize="small" />,
  },
  [PANEL_IDS.ARM7_DISASSEMBLY]: {
    id: PANEL_IDS.ARM7_DISASSEMBLY,
    name: "ARM7: Disassembly",
    icon: <BugReportIcon fontSize="small" />,
  },
  [PANEL_IDS.ARM7_MEMORY]: {
    id: PANEL_IDS.ARM7_MEMORY,
    name: "ARM7: Memory",
    icon: <StorageIcon fontSize="small" />,
  },
  [PANEL_IDS.ARM7_BREAKPOINTS]: {
    id: PANEL_IDS.ARM7_BREAKPOINTS,
    name: "ARM7: Breakpoints",
    icon: <CodeIcon fontSize="small" />,
  },
  [PANEL_IDS.CLX2_TA]: {
    id: PANEL_IDS.CLX2_TA,
    name: "CLX2: TA",
    icon: <ViewInArIcon fontSize="small" />,
  },
  [PANEL_IDS.CLX2_CORE]: {
    id: PANEL_IDS.CLX2_CORE,
    name: "CLX2: CORE",
    icon: <ImageIcon fontSize="small" />,
  },
  [PANEL_IDS.SGC]: {
    id: PANEL_IDS.SGC,
    name: "SGC",
    icon: <GraphicEqIcon fontSize="small" />,
  },
  [PANEL_IDS.DSP_DISASSEMBLY]: {
    id: PANEL_IDS.DSP_DISASSEMBLY,
    name: "DSP: Disassembly",
    icon: <BugReportIcon fontSize="small" />,
  },
  [PANEL_IDS.DSP_BREAKPOINTS]: {
    id: PANEL_IDS.DSP_BREAKPOINTS,
    name: "DSP: Breakpoints",
    icon: <CodeIcon fontSize="small" />,
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
