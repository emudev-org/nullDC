import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import { Box } from "@mui/material";

export interface WaveformPlotterProps {
  width?: number;
  height?: number;
  maxSamples?: number;
  maxAmplitude?: number;
  fillWidth?: boolean;
}

export interface WaveformPlotterRef {
  appendSample: (sample: number) => void;
  clear: () => void;
}

export const WaveformPlotter = forwardRef<WaveformPlotterRef, WaveformPlotterProps>(
  ({ width = 800, height = 400, maxSamples = 800, maxAmplitude = 32767, fillWidth = false }, ref) => {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const samplesRef = useRef<number[]>(Array(maxSamples).fill(0));
    const currentXRef = useRef(0);
    const animationFrameRef = useRef<number | null>(null);
    const [actualWidth, setActualWidth] = useState(width);

    const draw = () => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      const midY = height / 2;
      const drawWidth = fillWidth ? actualWidth : width;

      // Clear canvas
      ctx.clearRect(0, 0, drawWidth, height);

      // Draw blue waveform
      ctx.beginPath();
      ctx.strokeStyle = "#2196f3";
      ctx.lineWidth = 2;

      const scaleX = fillWidth ? drawWidth / maxSamples : 1;

      for (let x = 0; x < maxSamples; x++) {
        const y = midY - (samplesRef.current[x] / maxAmplitude) * (height / 2);
        const scaledX = x * scaleX;
        if (x === 0) {
          ctx.moveTo(scaledX, y);
        } else {
          ctx.lineTo(scaledX, y);
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
    }, [height, maxAmplitude, maxSamples, width, actualWidth, fillWidth]);

    useEffect(() => {
      if (!fillWidth) return;

      const handleResize = () => {
        if (containerRef.current) {
          const newWidth = containerRef.current.offsetWidth;
          setActualWidth(newWidth);
        }
      };

      handleResize(); // Initial size
      window.addEventListener("resize", handleResize);

      return () => {
        window.removeEventListener("resize", handleResize);
      };
    }, [fillWidth]);

    const canvasWidth = fillWidth ? actualWidth : width;

    return (
      <Box
        ref={containerRef}
        sx={{
          border: 1,
          borderColor: "divider",
          borderRadius: 1,
          overflow: "hidden",
          width: fillWidth ? "100%" : "auto",
        }}
      >
        <canvas ref={canvasRef} width={canvasWidth} height={height} />
      </Box>
    );
  }
);

WaveformPlotter.displayName = "WaveformPlotter";
