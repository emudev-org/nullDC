import { Tooltip, type TooltipProps } from "@mui/material";
import { useState } from "react";

/**
 * A tooltip component that hides when the mouse enters it.
 * Inherits all props from MUI Tooltip.
 */
export const HideOnHoverTooltip = (props: TooltipProps) => {
  const [open, setOpen] = useState(false);

  const handleOpen = () => {
    setOpen(true);
  };

  const handleClose = () => {
    setOpen(false);
  };

  return (
    <Tooltip
      {...props}
      open={open}
      onOpen={handleOpen}
      onClose={handleClose}
      componentsProps={{
        ...props.componentsProps,
        tooltip: {
          ...props.componentsProps?.tooltip,
          onMouseEnter: handleClose,
        },
      }}
    />
  );
};
