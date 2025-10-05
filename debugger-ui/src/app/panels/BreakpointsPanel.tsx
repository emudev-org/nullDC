import { Panel } from "../layout/Panel";
import { Chip, IconButton, List, ListItem, ListItemText, Stack, Switch, Tooltip, Typography } from "@mui/material";
import DeleteOutlineIcon from "@mui/icons-material/DeleteOutline";

const breakpoints = [
  {
    id: "bp-1",
    label: "dc.sh4.cpu.pc == 0x8C0000A0",
    kind: "code",
    enabled: true,
  },
  {
    id: "bp-2",
    label: "dc.aica.channel[0].step",
    kind: "event",
    enabled: false,
  },
];

export const BreakpointsPanel = () => (
  <Panel title="Breakpoints">
    <List dense disablePadding>
      {breakpoints.map((bp) => (
        <ListItem
          key={bp.id}
          secondaryAction={
            <Stack direction="row" spacing={1} alignItems="center">
              <Switch size="small" defaultChecked={bp.enabled} />
              <Tooltip title="Remove breakpoint">
                <IconButton edge="end" size="small">
                  <DeleteOutlineIcon fontSize="small" />
                </IconButton>
              </Tooltip>
            </Stack>
          }
        >
          <ListItemText
            primary={
              <Typography variant="body2" sx={{ fontFamily: "monospace" }}>
                {bp.label}
              </Typography>
            }
            secondary={<Chip label={bp.kind} size="small" color="primary" />}
          />
        </ListItem>
      ))}
    </List>
  </Panel>
);
