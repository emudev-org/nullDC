import { Box } from "@mui/material";
import React, { useRef, useEffect, memo, useImperativeHandle, forwardRef, useCallback } from "react";
import { SgcFrameData } from "../../lib/sgcChannelData";
import { SgcWaveformRenderer } from "./SgcWaveformRenderer";

export interface SgcWaveformCanvasHandle {
  setPanZoom: (scrollOffsetX: number, zoomLevel: number) => void;
  setPositions: (hoverPosition: number | null, playbackPosition: number) => void;
}

interface SgcWaveformCanvasProps {
  channelIndex: number;
  viewMode: 'pre-volpan' | 'post-volpan' | 'input';
  sgcBinaryData: ArrayBuffer;
  renderer: SgcWaveformRenderer;
  onHoverPositionChange: (position: number | null) => void;
  onPlaybackPositionChange: (position: number) => void;
}

export const SgcWaveformCanvas = memo(forwardRef<SgcWaveformCanvasHandle, SgcWaveformCanvasProps>(({
  channelIndex,
  viewMode,
  sgcBinaryData,
  renderer,
  onHoverPositionChange,
  onPlaybackPositionChange,
}, ref) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const waveformDataRef = useRef<number[]>([]);
  const panZoomRef = useRef<{ scrollOffsetX: number; zoomLevel: number }>({ scrollOffsetX: 0, zoomLevel: 1 });
  const [hoverPosition, setHoverPosition] = React.useState<number | null>(null);
  const [playbackPosition, setPlaybackPosition] = React.useState<number>(0);

  // Rendering state
  const renderQueuedRef = useRef<boolean>(false);
  const animationFrameRef = useRef<number | undefined>(undefined);

  // Cached vertex buffers - only recreated when waveform data changes
  const waveformBufferRef = useRef<WebGLBuffer | null>(null);
  const aegBufferRef = useRef<WebGLBuffer | null>(null);
  const fegBufferRef = useRef<WebGLBuffer | null>(null);

  // Position indicator buffers - recreated on every render
  const positionBuffersRef = useRef<WebGLBuffer[]>([]);

  // WebGL rendering function ref
  const renderCanvasRef = useRef<(() => void) | null>(null);

  // Queue a graph rerender using requestAnimationFrame
  const queueGraphRerender = useCallback(() => {
    if (renderQueuedRef.current) return; // Already queued
    renderQueuedRef.current = true;

    animationFrameRef.current = requestAnimationFrame(() => {
      renderQueuedRef.current = false;
      if (renderCanvasRef.current) {
        renderCanvasRef.current();
      }
    });
  }, []);

  // Calculate sample position from mouse X coordinate
  const calculateSamplePosition = useCallback((clientX: number): number => {
    const container = containerRef.current;
    if (!container) return 0;

    const rect = container.getBoundingClientRect();
    const x = clientX - rect.left;
    const canvasWidth = rect.width;
    const { scrollOffsetX, zoomLevel } = panZoomRef.current;

    // Account for scroll offset and zoom
    const normalizedX = (x + scrollOffsetX) / (canvasWidth * zoomLevel);
    const constrainedNormalized = Math.max(0, Math.min(normalizedX, 1));

    // Convert to sample index [0, 1024)
    const sampleIndex = Math.floor(constrainedNormalized * 1024);
    return Math.max(0, Math.min(sampleIndex, 1023));
  }, []);

  // Handle mouse move for hover position
  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const sampleIndex = calculateSamplePosition(e.clientX);
    setHoverPosition(sampleIndex);
    onHoverPositionChange(sampleIndex);
  }, [calculateSamplePosition, onHoverPositionChange]);

  // Handle mouse leave to clear hover position
  const handleMouseLeave = useCallback(() => {
    setHoverPosition(null);
    onHoverPositionChange(null);
  }, [onHoverPositionChange]);

  // Handle click to set playback position
  const handleClick = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const sampleIndex = calculateSamplePosition(e.clientX);
    setPlaybackPosition(sampleIndex);
    onPlaybackPositionChange(sampleIndex);
  }, [calculateSamplePosition, onPlaybackPositionChange]);

  // Extract waveform data from SGC frame data
  useEffect(() => {
    // Extract sample data from binary SGC frame data
    const numFrames = 1024; // Total frames in the data
    const waveform: number[] = [];

    for (let frameIdx = 0; frameIdx < numFrames; frameIdx++) {
      // Create SgcFrameData for each frame and get the specific channel
      const frameData = new SgcFrameData(sgcBinaryData, frameIdx);
      const frameChannelData = frameData.getChannel(channelIndex);

      // Get the appropriate sample based on view mode
      let sample: number;
      if (viewMode === 'input') {
        // Use sample_filtered for filtered/input view
        sample = frameChannelData.sample_filtered;
      } else if (viewMode === 'pre-volpan') {
        // Use sample_post_tl for pre-volpan view
        sample = frameChannelData.sample_post_tl;
      } else {
        // For post-volpan, we'll handle it differently in rendering
        sample = frameChannelData.sample_post_tl;
      }

      // Normalize sample to -1.0 to 1.0 range (assuming int16 range)
      const normalized = sample / 32768.0;
      waveform.push(normalized);
    }

    waveformDataRef.current = waveform;
    queueGraphRerender();
  }, [channelIndex, sgcBinaryData, viewMode, queueGraphRerender]);


  // Create/update cached vertex buffers when waveform data or canvas size changes
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const { width, height } = canvas;
    if (width === 0 || height === 0) return;

    const numFrames = 1024;
    const waveform = waveformDataRef.current;
    if (waveform.length === 0) return;

    // Clean up old buffers
    if (waveformBufferRef.current) {
      renderer.destroyVertexBuffer(waveformBufferRef.current);
      waveformBufferRef.current = null;
    }
    if (aegBufferRef.current) {
      renderer.destroyVertexBuffer(aegBufferRef.current);
      aegBufferRef.current = null;
    }
    if (fegBufferRef.current) {
      renderer.destroyVertexBuffer(fegBufferRef.current);
      fegBufferRef.current = null;
    }

    // Create waveform buffer
    if (viewMode === 'post-volpan') {
      // Post-volpan mode - 3 waveforms (left, right, DSP)
      const topCenterY = height / 6;
      const middleCenterY = height / 2;
      const bottomCenterY = (height * 5) / 6;
      const waveHeight = height * 0.12;

      // Pre-allocate Float32Array: 3 channels * numFrames * 6 vertices/bar * 6 floats/vertex
      const vertices = new Float32Array(numFrames * 3 * 6 * 6);
      let offset = 0;

      for (let i = 0; i < numFrames; i++) {
        const frameData = new SgcFrameData(sgcBinaryData, i);
        const frameChannelData = frameData.getChannel(channelIndex);

        const x = (i / numFrames) * width;
        const nextX = ((i + 1) / numFrames) * width;

        const leftAmp = frameChannelData.sample_left / 32768.0;
        const rightAmp = frameChannelData.sample_right / 32768.0;
        const dspAmp = frameChannelData.sample_dsp / 32768.0;

        // Left channel (blue)
        const leftHeight = Math.abs(leftAmp * waveHeight);
        const leftY = leftAmp >= 0 ? topCenterY - leftHeight : topCenterY;
        vertices.set([
          x, leftY, 0.098, 0.463, 0.824, 0.8,
          nextX, leftY, 0.098, 0.463, 0.824, 0.8,
          x, leftY + leftHeight, 0.098, 0.463, 0.824, 0.8,
          nextX, leftY, 0.098, 0.463, 0.824, 0.8,
          nextX, leftY + leftHeight, 0.098, 0.463, 0.824, 0.8,
          x, leftY + leftHeight, 0.098, 0.463, 0.824, 0.8
        ], offset);
        offset += 36;

        // Right channel (pink)
        const rightHeight = Math.abs(rightAmp * waveHeight);
        const rightY = rightAmp >= 0 ? middleCenterY - rightHeight : middleCenterY;
        vertices.set([
          x, rightY, 0.824, 0.098, 0.463, 0.8,
          nextX, rightY, 0.824, 0.098, 0.463, 0.8,
          x, rightY + rightHeight, 0.824, 0.098, 0.463, 0.8,
          nextX, rightY, 0.824, 0.098, 0.463, 0.8,
          nextX, rightY + rightHeight, 0.824, 0.098, 0.463, 0.8,
          x, rightY + rightHeight, 0.824, 0.098, 0.463, 0.8
        ], offset);
        offset += 36;

        // DSP channel (orange)
        const dspHeight = Math.abs(dspAmp * waveHeight);
        const dspY = dspAmp >= 0 ? bottomCenterY - dspHeight : bottomCenterY;
        vertices.set([
          x, dspY, 1.0, 0.596, 0.0, 0.8,
          nextX, dspY, 1.0, 0.596, 0.0, 0.8,
          x, dspY + dspHeight, 1.0, 0.596, 0.0, 0.8,
          nextX, dspY, 1.0, 0.596, 0.0, 0.8,
          nextX, dspY + dspHeight, 1.0, 0.596, 0.0, 0.8,
          x, dspY + dspHeight, 1.0, 0.596, 0.0, 0.8
        ], offset);
        offset += 36;
      }

      waveformBufferRef.current = renderer.createVertexBuffer(vertices);
    } else {
      // Pre-volpan or Input mode - single centered waveform
      const centerY = height / 2;
      const waveHeight = height * 0.4;
      const [r, g, b, a] = viewMode === 'input' ? [0.612, 0.153, 0.690, 0.8] : [0.098, 0.463, 0.824, 0.8];

      // Pre-allocate Float32Array: numFrames * 6 vertices/bar * 6 floats/vertex
      const vertices = new Float32Array(numFrames * 6 * 6);
      let offset = 0;

      waveform.forEach((amplitude, i) => {
        const x = (i / numFrames) * width;
        const nextX = ((i + 1) / numFrames) * width;
        const barHeight = Math.abs(amplitude * waveHeight);
        const barY = amplitude >= 0 ? centerY - barHeight : centerY;

        vertices.set([
          x, barY, r, g, b, a,
          nextX, barY, r, g, b, a,
          x, barY + barHeight, r, g, b, a,
          nextX, barY, r, g, b, a,
          nextX, barY + barHeight, r, g, b, a,
          x, barY + barHeight, r, g, b, a
        ], offset);
        offset += 36;
      });

      waveformBufferRef.current = renderer.createVertexBuffer(vertices);
    }

    // Create envelope buffers
    const centerY = height / 2;
    const envelopeHeight = height * 0.3;

    // AEG envelope (green)
    const aegVertices = new Float32Array((numFrames - 1) * 6 * 6);
    let aegOffset = 0;

    for (let i = 0; i < numFrames - 1; i++) {
      const frameData1 = new SgcFrameData(sgcBinaryData, i);
      const frameData2 = new SgcFrameData(sgcBinaryData, i + 1);
      const channelData1 = frameData1.getChannel(channelIndex);
      const channelData2 = frameData2.getChannel(channelIndex);

      const x1 = (i / (numFrames - 1)) * width;
      const x2 = ((i + 1) / (numFrames - 1)) * width;
      const y1 = centerY - height * 0.15 - ((channelData1.aeg_value / 0x3FF) * envelopeHeight);
      const y2 = centerY - height * 0.15 - ((channelData2.aeg_value / 0x3FF) * envelopeHeight);

      aegVertices.set([
        x1, y1 - 0.75, 0.298, 0.686, 0.314, 0.7,
        x2, y2 - 0.75, 0.298, 0.686, 0.314, 0.7,
        x1, y1 + 0.75, 0.298, 0.686, 0.314, 0.7,
        x2, y2 - 0.75, 0.298, 0.686, 0.314, 0.7,
        x2, y2 + 0.75, 0.298, 0.686, 0.314, 0.7,
        x1, y1 + 0.75, 0.298, 0.686, 0.314, 0.7
      ], aegOffset);
      aegOffset += 36;
    }
    aegBufferRef.current = renderer.createVertexBuffer(aegVertices);

    // FEG envelope (orange)
    const fegVertices = new Float32Array((numFrames - 1) * 6 * 6);
    let fegOffset = 0;

    for (let i = 0; i < numFrames - 1; i++) {
      const frameData1 = new SgcFrameData(sgcBinaryData, i);
      const frameData2 = new SgcFrameData(sgcBinaryData, i + 1);
      const channelData1 = frameData1.getChannel(channelIndex);
      const channelData2 = frameData2.getChannel(channelIndex);

      const x1 = (i / (numFrames - 1)) * width;
      const x2 = ((i + 1) / (numFrames - 1)) * width;
      const y1 = centerY + height * 0.15 - ((channelData1.feg_value / 0x1FF8) * envelopeHeight);
      const y2 = centerY + height * 0.15 - ((channelData2.feg_value / 0x1FF8) * envelopeHeight);

      fegVertices.set([
        x1, y1 - 0.75, 1.0, 0.596, 0.0, 0.7,
        x2, y2 - 0.75, 1.0, 0.596, 0.0, 0.7,
        x1, y1 + 0.75, 1.0, 0.596, 0.0, 0.7,
        x2, y2 - 0.75, 1.0, 0.596, 0.0, 0.7,
        x2, y2 + 0.75, 1.0, 0.596, 0.0, 0.7,
        x1, y1 + 0.75, 1.0, 0.596, 0.0, 0.7
      ], fegOffset);
      fegOffset += 36;
    }
    fegBufferRef.current = renderer.createVertexBuffer(fegVertices);

    // Trigger a render
    queueGraphRerender();

    // Cleanup on unmount or when buffers are recreated
    return () => {
      if (waveformBufferRef.current) {
        renderer.destroyVertexBuffer(waveformBufferRef.current);
        waveformBufferRef.current = null;
      }
      if (aegBufferRef.current) {
        renderer.destroyVertexBuffer(aegBufferRef.current);
        aegBufferRef.current = null;
      }
      if (fegBufferRef.current) {
        renderer.destroyVertexBuffer(fegBufferRef.current);
        fegBufferRef.current = null;
      }
    };
  }, [renderer, sgcBinaryData, channelIndex, viewMode, queueGraphRerender]);

  // Render using cached buffers - only updates when pan/zoom or positions change
  useEffect(() => {
    const renderCanvas = () => {
      const canvas = canvasRef.current;
      if (!canvas) return;

      const { width, height } = canvas;
      if (width === 0 || height === 0) return;

      const numFrames = 1024;

      // Get current pan/zoom values
      const { scrollOffsetX, zoomLevel } = panZoomRef.current;

      // Clear offscreen canvas
      renderer.clear();

      // Render waveform (cached)
      if (waveformBufferRef.current) {
        const vertexCount = viewMode === 'post-volpan' ? numFrames * 3 * 6 : numFrames * 6;
        renderer.render(waveformBufferRef.current, vertexCount, scrollOffsetX, zoomLevel);
      }

      // Render AEG envelope (cached)
      if (aegBufferRef.current) {
        renderer.render(aegBufferRef.current, (numFrames - 1) * 6, scrollOffsetX, zoomLevel);
      }

      // Render FEG envelope (cached)
      if (fegBufferRef.current) {
        renderer.render(fegBufferRef.current, (numFrames - 1) * 6, scrollOffsetX, zoomLevel);
      }

      // Clean up old position buffers
      positionBuffersRef.current.forEach(buffer => renderer.destroyVertexBuffer(buffer));
      positionBuffersRef.current = [];

      // Render position indicators (not cached, recreated each frame)
      if (hoverPosition !== null) {
        const hoverX = (hoverPosition / (numFrames - 1)) * width;
        const hoverVertices = new Float32Array([
          hoverX - 1, 0, 1, 1, 1, 0.4,
          hoverX + 1, 0, 1, 1, 1, 0.4,
          hoverX - 1, height, 1, 1, 1, 0.4,
          hoverX + 1, 0, 1, 1, 1, 0.4,
          hoverX + 1, height, 1, 1, 1, 0.4,
          hoverX - 1, height, 1, 1, 1, 0.4
        ]);
        const hoverBuffer = renderer.createVertexBuffer(hoverVertices);
        positionBuffersRef.current.push(hoverBuffer);
        renderer.render(hoverBuffer, 6, 0, 1); // No pan/zoom for indicators
      }

      const playbackX = (playbackPosition / (numFrames - 1)) * width;
      const playbackVertices = new Float32Array([
        playbackX - 1, 0, 1, 0.596, 0, 0.9,
        playbackX + 1, 0, 1, 0.596, 0, 0.9,
        playbackX - 1, height, 1, 0.596, 0, 0.9,
        playbackX + 1, 0, 1, 0.596, 0, 0.9,
        playbackX + 1, height, 1, 0.596, 0, 0.9,
        playbackX - 1, height, 1, 0.596, 0, 0.9
      ]);
      const playbackBuffer = renderer.createVertexBuffer(playbackVertices);
      positionBuffersRef.current.push(playbackBuffer);
      renderer.render(playbackBuffer, 6, 0, 1); // No pan/zoom for indicators

      // Copy offscreen canvas to display canvas
      renderer.copyToCanvas(canvas);
    };

    // Assign render function to ref
    renderCanvasRef.current = renderCanvas;

    // Trigger initial render
    renderCanvas();
  }, [renderer, viewMode, hoverPosition, playbackPosition]);


  // Expose setPanZoom and setPositions methods to parent
  useImperativeHandle(ref, () => ({
    setPanZoom: (scrollOffsetX: number, zoomLevel: number) => {
      panZoomRef.current = { scrollOffsetX, zoomLevel };
      queueGraphRerender();
    },
    setPositions: (hover: number | null, playback: number) => {
      setHoverPosition(hover);
      setPlaybackPosition(playback);
    }
  }), [queueGraphRerender]);

  // Update canvas size on container resize using ResizeObserver
  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const updateCanvasSize = () => {
      const rect = container.getBoundingClientRect();
      canvas.width = rect.width;
      canvas.height = rect.height;
      queueGraphRerender();
    };

    // Initial size
    updateCanvasSize();

    // Use ResizeObserver to detect container size changes
    const resizeObserver = new ResizeObserver(() => {
      updateCanvasSize();
    });

    resizeObserver.observe(container);

    return () => {
      resizeObserver.disconnect();
    };
  }, [queueGraphRerender]);

  return (
    <Box
      ref={containerRef}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
      onClick={handleClick}
      sx={{
        flex: 1,
        position: 'relative',
        minHeight: 0,
        overflow: 'hidden',
        cursor: 'crosshair',
      }}
    >
      <canvas
        ref={canvasRef}
        style={{
          width: '100%',
          height: '100%',
          display: 'block',
        }}
      />
    </Box>
  );
}));

SgcWaveformCanvas.displayName = 'SgcWaveformCanvas';
