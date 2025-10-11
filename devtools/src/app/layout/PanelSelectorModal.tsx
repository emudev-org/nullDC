import { useState, useMemo, useEffect, useRef } from "react";
import {
  Dialog,
  TextField,
  List,
  ListItem,
  ListItemButton,
  ListItemText,
  Paper,
  Box,
  Typography,
} from "@mui/material";
import type { PanelDefinition } from "./DockingLayout";

interface PanelSelectorModalProps {
  open: boolean;
  onClose: () => void;
  onSelect: (panelId: string) => void;
  availablePanels: PanelDefinition[];
  existingPanelIds: string[];
}

export const PanelSelectorModal = ({
  open,
  onClose,
  onSelect,
  availablePanels,
  existingPanelIds,
}: PanelSelectorModalProps) => {
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  // Filter panels based on search query and exclude already existing panels
  const filteredPanels = useMemo(() => {
    const available = availablePanels.filter(
      (panel) => !existingPanelIds.includes(panel.id)
    );

    if (!searchQuery) {
      return available;
    }

    const query = searchQuery.toLowerCase();
    return available.filter(
      (panel) =>
        panel.title.toLowerCase().includes(query) ||
        panel.id.toLowerCase().includes(query)
    );
  }, [availablePanels, existingPanelIds, searchQuery]);

  // Reset state when modal opens
  useEffect(() => {
    if (open) {
      setSearchQuery("");
      setSelectedIndex(0);
      // Focus input after a short delay to ensure modal is rendered
      setTimeout(() => {
        inputRef.current?.focus();
      }, 100);
    }
  }, [open]);

  // Handle keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!open) return;

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIndex((prev) =>
            Math.min(prev + 1, filteredPanels.length - 1)
          );
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIndex((prev) => Math.max(prev - 1, 0));
          break;
        case "Enter":
          e.preventDefault();
          if (filteredPanels[selectedIndex]) {
            handleSelect(filteredPanels[selectedIndex].id);
          }
          break;
        case "Escape":
          e.preventDefault();
          onClose();
          break;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, filteredPanels, selectedIndex]);

  // Reset selected index when filtered list changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [searchQuery]);

  const handleSelect = (panelId: string) => {
    onSelect(panelId);
    onClose();
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="sm"
      fullWidth
      PaperProps={{
        sx: {
          position: "fixed",
          top: "20%",
          m: 0,
          maxHeight: "60vh",
        },
      }}
      BackdropProps={{
        sx: {
          backgroundColor: "rgba(0, 0, 0, 0.5)",
        },
      }}
    >
      <Box sx={{ p: 2 }}>
        <TextField
          inputRef={inputRef}
          fullWidth
          placeholder="Type to search panels..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          variant="outlined"
          size="small"
          autoFocus
          sx={{ mb: 1 }}
        />
        <Paper
          variant="outlined"
          sx={{
            maxHeight: "400px",
            overflow: "auto",
          }}
        >
          {filteredPanels.length === 0 ? (
            <Box sx={{ p: 3, textAlign: "center" }}>
              <Typography variant="body2" color="text.secondary">
                {existingPanelIds.length === availablePanels.length
                  ? "All panels are already open"
                  : "No panels found"}
              </Typography>
            </Box>
          ) : (
            <List disablePadding>
              {filteredPanels.map((panel, index) => (
                <ListItem key={panel.id} disablePadding>
                  <ListItemButton
                    selected={index === selectedIndex}
                    onClick={() => handleSelect(panel.id)}
                    sx={{
                      "&.Mui-selected": {
                        backgroundColor: "primary.main",
                        color: "primary.contrastText",
                        "&:hover": {
                          backgroundColor: "primary.dark",
                        },
                      },
                    }}
                  >
                    <ListItemText
                      primary={panel.title}
                      secondary={panel.id}
                      secondaryTypographyProps={{
                        sx: {
                          color: index === selectedIndex ? "inherit" : undefined,
                          opacity: index === selectedIndex ? 0.7 : undefined,
                        },
                      }}
                    />
                  </ListItemButton>
                </ListItem>
              ))}
            </List>
          )}
        </Paper>
      </Box>
    </Dialog>
  );
};
