import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { Box } from "@mui/material";

export interface WaveformPlotterProps {
  width?: number;
  height?: number;
  maxSamples?: number;
  maxAmplitude?: number;
}

export interface WaveformPlotterRef {
  appendSample: (sample: number) => void;
  clear: () => void;
}

export const WaveformPlotter = forwardRef<WaveformPlotterRef, WaveformPlotterProps>(
  ({ width = 800, height = 400, maxSamples = 800, maxAmplitude = 32767 }, ref) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const samplesRef = useRef<number[]>(Array(maxSamples).fill(0));
    const currentXRef = useRef(0);
    const animationFrameRef = useRef<number | null>(null);

    const draw = () => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      const midY = height / 2;

      // Clear canvas
      ctx.clearRect(0, 0, width, height);

      // Draw blue waveform
      ctx.beginPath();
      ctx.strokeStyle = "#2196f3";
      ctx.lineWidth = 2;

      for (let x = 0; x < maxSamples; x++) {
        const y = midY - (samplesRef.current[x] / maxAmplitude) * (height / 2);
        if (x === 0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
      }

      ctx.stroke();
    };

    useImperativeHandle(ref, () => ({
      appendSample: (sample: number) => {
        samplesRef.current[currentXRef.current] = sample;
        currentXRef.current = (currentXRef.current + 1) % maxSamples;
      },
      clear: () => {
        samplesRef.current = Array(maxSamples).fill(0);
        currentXRef.current = 0;
        draw();
      },
    }));

    useEffect(() => {
      const interval = setInterval(() => {
        draw();
      }, 50);

      return () => {
        clearInterval(interval);
        if (animationFrameRef.current !== null) {
          cancelAnimationFrame(animationFrameRef.current);
        }
      };
    }, [height, maxAmplitude, maxSamples, width]);

    return (
      <Box
        sx={{
          border: 1,
          borderColor: "divider",
          borderRadius: 1,
          overflow: "hidden",
        }}
      >
        <canvas ref={canvasRef} width={width} height={height} />
      </Box>
    );
  }
);

WaveformPlotter.displayName = "WaveformPlotter";
