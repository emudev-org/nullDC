import { Panel } from "../layout/Panel";
import { IconButton, List, ListItem, ListItemText, Tooltip, Typography } from "@mui/material";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";
import AddIcon from "@mui/icons-material/Add";

const placeholder = [
  { path: "dc.sh4.cpu.pc", value: "0x8C0000A0" },
  { path: "dc.sh4.dmac.dmaor", value: "0x0000" },
  { path: "dc.aica.dsp.acc", value: "0x1F" },
];

export const WatchPanel = () => {
  return (
    <Panel
      title="Watch"
      action={
        <Tooltip title="Add watch">
          <IconButton size="small" color="primary">
            <AddIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      }
    >
      <List dense disablePadding>
        {placeholder.map((entry) => (
          <ListItem
            key={entry.path}
            secondaryAction={
              <IconButton edge="end" size="small" aria-label={`Remove ${entry.path}`}>
                <DeleteOutlineIcon fontSize="small" />
              </IconButton>
            }
          >
            <ListItemText
              primary={
                <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                  {entry.path}
                </Typography>
              }
              secondary={
                <Typography variant="caption" color="text.secondary">
                  {entry.value}
                </Typography>
              }
            />
          </ListItem>
        ))}
      </List>
    </Panel>
  );
};
