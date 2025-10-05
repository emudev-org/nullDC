import { useMemo } from "react";
import { Panel } from "../layout/Panel";
import { Box } from "@mui/material";

const generateWave = (length: number) =>
  Array.from({ length }, (_, index) => Math.sin((index / length) * Math.PI * 4));

export const AudioPanel = () => {
  const samples = useMemo(() => generateWave(64), []);

  return (
    <Panel title="AICA Waveform">
      <Box
        component="svg"
        viewBox="0 0 100 40"
        preserveAspectRatio="none"
        sx={{ width: "100%", height: "100%" }}
      >
        <polyline
          fill="none"
          strokeWidth={1}
          stroke="var(--mui-palette-primary-light, #81d4fa)"
          points={samples
            .map((sample, index) => {
              const x = (index / (samples.length - 1)) * 100;
              const y = 20 - sample * 18;
              return `${x},${y}`;
            })
            .join(" ")}
        />
      </Box>
    </Panel>
  );
};
