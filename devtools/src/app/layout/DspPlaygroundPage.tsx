import { AppBar, Box } from "@mui/material";
import { TopNav } from "./TopNav";
import { AboutDialog } from "./AboutDialog";
import { useAboutModal } from "./useAboutModal";
import { useNavigate } from "react-router-dom";
import { DspPlaygroundPanel } from "../panels/DspPlaygroundPanel";

export const DspPlaygroundPage = () => {
  const navigate = useNavigate();
  const { open: aboutOpen, show: showAbout, hide: hideAbout } = useAboutModal();

  return (
    <Box sx={{ minHeight: "100vh", display: "flex", flexDirection: "column" }}>
      <AppBar position="static" elevation={1} color="default">
        <TopNav
          onHomeClick={() => navigate("/")}
          onDocsClick={() => navigate("/docs")}
          onAboutClick={showAbout}
          active="workspace"
          currentPage="dsp-playground"
        />
      </AppBar>
      <Box sx={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
        <DspPlaygroundPanel />
      </Box>
      <AboutDialog open={aboutOpen} onClose={hideAbout} />
    </Box>
  );
};
