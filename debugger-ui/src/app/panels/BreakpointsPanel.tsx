import { Panel } from "../layout/Panel";
import { Chip, IconButton, List, ListItem, ListItemText, Switch, Tooltip, Typography } from "@mui/material";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

export const BreakpointsPanel = () => {
  const breakpoints = useDebuggerDataStore((state) => state.breakpoints);

  return (
    <Panel title="Breakpoints">
      {breakpoints.length === 0 ? (
        <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
          No breakpoints defined.
        </Typography>
      ) : (
        <List dense disablePadding>
          {breakpoints.map((bp) => (
            <ListItem
              key={bp.id}
              secondaryAction={
                <Tooltip title="Editing breakpoints will be supported soon">
                  <IconButton edge="end" size="small" disabled>
                    <DeleteOutlineIcon fontSize="small" />
                  </IconButton>
                </Tooltip>
              }
            >
              <ListItemText
                primary={
                  <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                    {bp.location}
                  </Typography>
                }
                secondary={
                  <Typography variant="caption" color="text.secondary" sx={{ display: "flex", gap: 1, alignItems: "center" }}>
                    <Chip label={bp.kind} size="small" color="primary" variant="outlined" />
                    <Switch size="small" checked={bp.enabled} disabled />
                    <span>Hits: {bp.hitCount}</span>
                  </Typography>
                }
              />
            </ListItem>
          ))}
        </List>
      )}
    </Panel>
  );
};
