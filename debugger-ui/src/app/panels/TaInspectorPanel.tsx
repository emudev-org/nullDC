import { Panel } from "../layout/Panel";
import { List, ListItem, ListItemText, Typography } from "@mui/material";

const primitives = [
  { type: "Triangle Strip", count: 32 },
  { type: "Quad", count: 12 },
  { type: "Sprite", count: 8 },
];

export const TaInspectorPanel = () => (
  <Panel title="TA Debugger">
    <List dense disablePadding>
      {primitives.map((primitive) => (
        <ListItem key={primitive.type}>
          <ListItemText
            primary={primitive.type}
            secondary={<Typography variant="caption">{`${primitive.count} primitives`}</Typography>}
          />
        </ListItem>
      ))}
    </List>
  </Panel>
);
