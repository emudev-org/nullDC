import type { PropsWithChildren, ReactNode } from "react";
import { Box, Paper, Typography } from "@mui/material";

type PanelProps = PropsWithChildren<{
  title: string;
  action?: ReactNode;
  footer?: ReactNode;
}>;

export const Panel = ({ title, action, footer, children }: PanelProps) => {
  return (
    <Paper
      elevation={0}
      sx={{
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        height: "100%",
      }}
    >
      <Box
        sx={{
          px: 2,
          py: 1,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          borderBottom: "1px solid",
          borderColor: "divider",
        }}
      >
        <Typography variant="subtitle2" component="h2" sx={{ fontWeight: 600 }}>
          {title}
        </Typography>
        {action && <Box sx={{ display: "flex", gap: 1 }}>{action}</Box>}
      </Box>
      <Box sx={{ flex: 1, overflow: "auto" }}>{children}</Box>
      {footer && (
        <Box
          sx={{
            px: 2,
            py: 1,
            borderTop: "1px solid",
            borderColor: "divider",
          }}
        >
          {footer}
        </Box>
      )}
    </Paper>
  );
};
