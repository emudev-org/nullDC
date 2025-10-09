import type { ReactNode } from "react";
import { Toolbar, Stack, Typography, Divider, Button, Box } from "@mui/material";

interface TopNavProps {
  onHomeClick: () => void;
  onDocsClick: () => void;
  onAboutClick: () => void;
  onResetLayout?: () => void;
  rightSection?: ReactNode;
  active?: "home" | "docs" | "workspace";
  title?: string;
}

export const TopNav = ({
  onHomeClick,
  onDocsClick,
  onAboutClick,
  onResetLayout,
  rightSection,
  active,
  title = "nullDC Debugger",
}: TopNavProps) => {
  const homeVariant = active === "home" ? "contained" : "text";
  const docsVariant = active === "docs" ? "contained" : "text";

  return (
    <Toolbar sx={{ gap: 2 }}>
      <Stack direction="row" spacing={1.5} alignItems="center" sx={{ flexShrink: 0 }}>
        <Typography variant="h6">{title}</Typography>
        <Divider orientation="vertical" flexItem />
        <Button variant={homeVariant} color="primary" onClick={onHomeClick}>
          Home
        </Button>
        <Button variant={docsVariant} color="primary" onClick={onDocsClick}>
          Docs
        </Button>
        <Button variant="text" color="primary" onClick={onAboutClick}>
          About
        </Button>
        {onResetLayout && (
          <Button variant="text" color="primary" onClick={onResetLayout}>
            Reset layout
          </Button>
        )}
      </Stack>
      <Box sx={{ flexGrow: 1 }} />
      {rightSection}
    </Toolbar>
  );
};
