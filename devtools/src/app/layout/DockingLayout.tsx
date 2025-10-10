import { useEffect, useRef, useCallback, useMemo, type ReactElement, type FunctionComponent } from "react";
import {
  DockviewReact,
  type DockviewReadyEvent,
  type IDockviewPanelProps,
  type DockviewApi,
} from "dockview";
import { Box } from "@mui/material";
import { useThemeMode } from "../../theme/ThemeModeProvider";
import "dockview/dist/styles/dockview.css";

export type PanelDefinition = {
  id: string;
  title: string;
  component: ReactElement;
};

type DockingLayoutProps = {
  panels: PanelDefinition[];
  onReady?: (api: DockviewApi) => void;
};

const DOCKING_LAYOUT_STORAGE_KEY = "nulldc-debugger-docking-layout";

// Store panel components in a Map
const panelComponentsMap = new Map<string, ReactElement>();

export const DockingLayout = ({ panels, onReady }: DockingLayoutProps) => {
  const apiRef = useRef<DockviewApi | null>(null);
  const { mode } = useThemeMode();
  const isDarkMode = mode === "dark";

  // Update the panel components map whenever panels change
  useEffect(() => {
    panelComponentsMap.clear();
    panels.forEach((panel) => {
      panelComponentsMap.set(panel.id, panel.component);
    });
  }, [panels]);

  // Create dynamic component types for each panel
  const components = useMemo(() => {
    const comps: Record<string, FunctionComponent<IDockviewPanelProps>> = {};

    panels.forEach((panel) => {
      comps[panel.id] = () => (
        <Box sx={{ height: "100%", width: "100%", overflow: "hidden", display: "flex", flexDirection: "column" }}>
          {panel.component}
        </Box>
      );
    });

    return comps;
  }, [panels]);

  const onDockviewReady = useCallback(
    (event: DockviewReadyEvent) => {
      apiRef.current = event.api;

      // Try to load saved layout
      const savedLayout = loadLayout();

      if (savedLayout) {
        try {
          event.api.fromJSON(savedLayout);
        } catch (error) {
          console.warn("Failed to restore docking layout, using default", error);
          createDefaultLayout(event.api, panels);
        }
      } else {
        createDefaultLayout(event.api, panels);
      }

      // Save layout on changes
      const disposable = event.api.onDidLayoutChange(() => {
        saveLayout(event.api.toJSON());
      });

      // Cleanup
      return () => {
        disposable.dispose();
      };
    },
    [panels]
  );

  useEffect(() => {
    if (apiRef.current && onReady) {
      onReady(apiRef.current);
    }
  }, [onReady]);

  return (
    <Box
      sx={{
        height: "100%",
        width: "100%",
        "& .dockview-theme": {
          "--dv-background-color": isDarkMode ? "#1e1e1e" : "#ffffff",
          "--dv-activegroup-visiblepanel-tab-background-color": isDarkMode ? "#2d2d30" : "#f3f3f3",
          "--dv-activegroup-hiddenpanel-tab-background-color": isDarkMode ? "#252526" : "#ececec",
          "--dv-inactivegroup-visiblepanel-tab-background-color": isDarkMode ? "#2d2d30" : "#f3f3f3",
          "--dv-inactivegroup-hiddenpanel-tab-background-color": isDarkMode ? "#1e1e1e" : "#e8e8e8",
          "--dv-tabs-and-actions-container-font-size": "13px",
          "--dv-tabs-and-actions-container-height": "35px",
          "--dv-drag-over-background-color": isDarkMode ? "rgba(83, 89, 93, 0.5)" : "rgba(0, 0, 0, 0.1)",
          "--dv-separator-border": isDarkMode ? "#3e3e42" : "#e0e0e0",
          "--dv-paneview-header-border-color": isDarkMode ? "#3e3e42" : "#e0e0e0",
          "--dv-tabs-and-actions-container-font-color": isDarkMode ? "#cccccc" : "#333333",
          "--dv-activegroup-visiblepanel-tab-color": isDarkMode ? "#ffffff" : "#000000",
        },
      }}
    >
      <DockviewReact
        components={components}
        onReady={onDockviewReady}
        className="dockview-theme"
      />
    </Box>
  );
};

// Helper functions
function createDefaultLayout(api: DockviewApi, panels: PanelDefinition[]) {
  // Create a single panel group with all panels as tabs
  if (panels.length === 0) return;

  // Add first panel to create the initial group
  const firstPanel = api.addPanel({
    id: panels[0].id,
    component: panels[0].id,
    title: panels[0].title,
  });

  // Add remaining panels to the same group
  for (let i = 1; i < panels.length; i++) {
    api.addPanel({
      id: panels[i].id,
      component: panels[i].id,
      title: panels[i].title,
      position: {
        referencePanel: firstPanel,
      },
    });
  }
}

function saveLayout(layout: any) {
  if (typeof window === "undefined") return;

  try {
    window.localStorage.setItem(DOCKING_LAYOUT_STORAGE_KEY, JSON.stringify(layout));
  } catch (error) {
    console.warn("Failed to save docking layout", error);
  }
}

function loadLayout(): any | null {
  if (typeof window === "undefined") return null;

  try {
    const raw = window.localStorage.getItem(DOCKING_LAYOUT_STORAGE_KEY);
    return raw ? JSON.parse(raw) : null;
  } catch (error) {
    console.warn("Failed to load docking layout", error);
    return null;
  }
}

export function clearDockingLayout() {
  if (typeof window === "undefined") return;

  try {
    window.localStorage.removeItem(DOCKING_LAYOUT_STORAGE_KEY);
  } catch (error) {
    console.warn("Failed to clear docking layout", error);
  }
}
