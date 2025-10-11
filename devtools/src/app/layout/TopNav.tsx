import { useMemo, type ReactNode } from "react";
import { Toolbar, Stack, Divider, Button, Box, Autocomplete, TextField } from "@mui/material";
import { useNavigate } from "react-router-dom";
import { createNavigationItems } from "./navigationItems";

interface TopNavProps {
  onAboutClick: () => void;
  onResetLayout?: () => void;
  rightSection?: ReactNode;
  centerSection?: ReactNode;
  currentPage?: string;
}

export const TopNav = ({
  onAboutClick,
  onResetLayout,
  rightSection,
  centerSection,
  currentPage,
}: TopNavProps) => {
  const navigate = useNavigate();

  // Navigation items are now defined in navigationItems.ts (not configurable)
  const navigationItems = useMemo(() => createNavigationItems(navigate), [navigate]);

  // Find the current page in navigation items
  const currentValue = currentPage
    ? navigationItems.find(item => item.id === currentPage) || null
    : null;

  return (
    <Toolbar sx={{ gap: 2, position: "relative" }}>
      <Stack direction="row" spacing={1.5} alignItems="center" sx={{ flexShrink: 0 }}>
        <Autocomplete
          options={navigationItems}
          groupBy={(option) => option.category}
          getOptionLabel={(option) => option.label}
          value={currentValue}
          onChange={(_, value) => {
            if (value) {
              value.onClick();
            }
          }}
          sx={{ width: 300 }}
          size="small"
          renderInput={(params) => (
            <TextField
              {...params}
              placeholder="Navigate to..."
              variant="outlined"
            />
          )}
          disableClearable={false}
          blurOnSelect
          autoHighlight
          autoSelect
          selectOnFocus
          clearOnEscape
        />
        {onResetLayout && (
          <>
            <Divider orientation="vertical" flexItem />
            <Button variant="text" color="primary" onClick={onResetLayout}>
              Reset layout
            </Button>
          </>
        )}
        <Divider orientation="vertical" flexItem />
        <Button variant="text" color="primary" onClick={onAboutClick}>
          About
        </Button>
      </Stack>
      {centerSection && (
        <Box sx={{ position: "absolute", left: "50%", transform: "translateX(-50%)" }}>
          {centerSection}
        </Box>
      )}
      <Box sx={{ flexGrow: 1 }} />
      {rightSection}
    </Toolbar>
  );
};
