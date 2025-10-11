import { AppBar, Box } from "@mui/material";
import { TopNav } from "./TopNav";
import { AboutDialog } from "./AboutDialog";
import { useAboutModal } from "./useAboutModal";
import { Sh4SimPanel } from "../panels/Sh4SimPanel";

export const Sh4SimPage = () => {
  const { open: aboutOpen, show: showAbout, hide: hideAbout } = useAboutModal();

  return (
    <Box sx={{ minHeight: "100vh", display: "flex", flexDirection: "column" }}>
      <AppBar position="static" elevation={1} color="default">
        <TopNav
          onAboutClick={showAbout}
          currentPage="sh4-sim"
        />
      </AppBar>
      <Box sx={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        <Sh4SimPanel />
      </Box>
      <AboutDialog open={aboutOpen} onClose={hideAbout} />
    </Box>
  );
};
