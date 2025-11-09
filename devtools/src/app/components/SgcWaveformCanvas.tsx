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
    // Calculate number of frames from buffer size
    const BYTES_PER_FRAME = 8192; // 64 channels × 128 bytes
    const numFrames = sgcBinaryData.byteLength / BYTES_PER_FRAME;
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


  // Create/update cached vertex buffers when waveform data changes
  useEffect(() => {
    // Calculate number of frames from buffer size
    const BYTES_PER_FRAME = 8192; // 64 channels × 128 bytes
    const numFrames = sgcBinaryData.byteLength / BYTES_PER_FRAME;
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
      // Normalized Y positions: top at 1/6, middle at 1/2, bottom at 5/6
      const topCenterY = 1.0 / 6.0;
      const middleCenterY = 0.5;
      const bottomCenterY = 5.0 / 6.0;
      const waveHeightScale = 0.12; // 12% of canvas height

      // Pre-allocate Float32Array: 3 channels * numFrames * 6 vertices/bar * 6 floats/vertex
      const vertices = new Float32Array(numFrames * 3 * 6 * 6);
      let offset = 0;

      for (let i = 0; i < numFrames; i++) {
        const frameData = new SgcFrameData(sgcBinaryData, i);
        const frameChannelData = frameData.getChannel(channelIndex);

        // X: sample index [0-1024)
        const sampleIdx = i;
        const nextSampleIdx = i + 1.5; // Make bars 1.5 samples wide for fatter appearance

        // Normalize amplitudes from int16 to [-1, 1]
        const leftAmp = frameChannelData.sample_left / 32768.0;
        const rightAmp = frameChannelData.sample_right / 32768.0;
        const dspAmp = frameChannelData.sample_dsp / 32768.0;

        // Left channel (blue) - Y normalized [0, 1]
        const leftYHeight = Math.abs(leftAmp * waveHeightScale);
        const leftY1 = leftAmp >= 0 ? topCenterY - leftYHeight : topCenterY;
        const leftY2 = leftAmp >= 0 ? topCenterY : topCenterY + leftYHeight;
        vertices.set([
          sampleIdx, leftY1, 0.098, 0.463, 0.824, 0.8,
          nextSampleIdx, leftY1, 0.098, 0.463, 0.824, 0.8,
          sampleIdx, leftY2, 0.098, 0.463, 0.824, 0.8,
          nextSampleIdx, leftY1, 0.098, 0.463, 0.824, 0.8,
          nextSampleIdx, leftY2, 0.098, 0.463, 0.824, 0.8,
          sampleIdx, leftY2, 0.098, 0.463, 0.824, 0.8
        ], offset);
        offset += 36;

        // Right channel (pink)
        const rightYHeight = Math.abs(rightAmp * waveHeightScale);
        const rightY1 = rightAmp >= 0 ? middleCenterY - rightYHeight : middleCenterY;
        const rightY2 = rightAmp >= 0 ? middleCenterY : middleCenterY + rightYHeight;
        vertices.set([
          sampleIdx, rightY1, 0.824, 0.098, 0.463, 0.8,
          nextSampleIdx, rightY1, 0.824, 0.098, 0.463, 0.8,
          sampleIdx, rightY2, 0.824, 0.098, 0.463, 0.8,
          nextSampleIdx, rightY1, 0.824, 0.098, 0.463, 0.8,
          nextSampleIdx, rightY2, 0.824, 0.098, 0.463, 0.8,
          sampleIdx, rightY2, 0.824, 0.098, 0.463, 0.8
        ], offset);
        offset += 36;

        // DSP channel (orange)
        const dspYHeight = Math.abs(dspAmp * waveHeightScale);
        const dspY1 = dspAmp >= 0 ? bottomCenterY - dspYHeight : bottomCenterY;
        const dspY2 = dspAmp >= 0 ? bottomCenterY : bottomCenterY + dspYHeight;
        vertices.set([
          sampleIdx, dspY1, 1.0, 0.596, 0.0, 0.8,
          nextSampleIdx, dspY1, 1.0, 0.596, 0.0, 0.8,
          sampleIdx, dspY2, 1.0, 0.596, 0.0, 0.8,
          nextSampleIdx, dspY1, 1.0, 0.596, 0.0, 0.8,
          nextSampleIdx, dspY2, 1.0, 0.596, 0.0, 0.8,
          sampleIdx, dspY2, 1.0, 0.596, 0.0, 0.8
        ], offset);
        offset += 36;
      }

      waveformBufferRef.current = renderer.createVertexBuffer(vertices);
    } else {
      // Pre-volpan or Input mode - single centered waveform
      const [r, g, b, a] = viewMode === 'input' ? [0.612, 0.153, 0.690, 0.8] : [0.098, 0.463, 0.824, 0.8];

      // Pre-allocate Float32Array: numFrames * 6 vertices/bar * 6 floats/vertex
      const vertices = new Float32Array(numFrames * 6 * 6);
      let offset = 0;

      waveform.forEach((amplitude, i) => {
        // X: sample index [0-1024)
        const sampleIdx = i;
        const nextSampleIdx = i + 1.5; // Make bars 1.5 samples wide for fatter appearance

        // Y: normalized [0, 1] where 0.5 is center
        // amplitude is in [-1, 1], scale by 0.4 (40% of height)
        const centerY = 0.5;
        const scaleY = 0.4;
        const y1 = amplitude >= 0 ? centerY - (amplitude * scaleY) : centerY;
        const y2 = amplitude >= 0 ? centerY : centerY - (amplitude * scaleY);

        vertices.set([
          sampleIdx, y1, r, g, b, a,
          nextSampleIdx, y1, r, g, b, a,
          sampleIdx, y2, r, g, b, a,
          nextSampleIdx, y1, r, g, b, a,
          nextSampleIdx, y2, r, g, b, a,
          sampleIdx, y2, r, g, b, a
        ], offset);
        offset += 36;
      });

      waveformBufferRef.current = renderer.createVertexBuffer(vertices);
    }

    // Create envelope buffers
    // Normalized Y: Both AEG and FEG use full height [0.0, 1.0]
    const envelopeHeightScale = 1.0; // Each envelope gets 100% of the canvas height
    const lineThicknessY = 0.003; // Vertical thickness in normalized space (thicker lines)

    // AEG envelope (green) - uses full height [0.0, 1.0]
    const aegVertices = new Float32Array((numFrames - 1) * 6 * 6);
    let aegOffset = 0;

    for (let i = 0; i < numFrames - 1; i++) {
      const frameData1 = new SgcFrameData(sgcBinaryData, i);
      const frameData2 = new SgcFrameData(sgcBinaryData, i + 1);
      const channelData1 = frameData1.getChannel(channelIndex);
      const channelData2 = frameData2.getChannel(channelIndex);

      // X: sample indices (make line wider by extending 1.5 samples)
      const sampleIdx1 = i;
      const sampleIdx2 = i + 1.5;

      // Y: normalized [0, 1], AEG uses full height
      // AEG: 0x3FF (max) should be at top (0.0), 0x000 (min) should be at bottom (1.0)
      const aegTopY = 0.0; // Top of canvas
      const y1 = aegTopY + ((1 - (channelData1.aeg_value / 0x3FF)) * envelopeHeightScale);
      const y2 = aegTopY + ((1 - (channelData2.aeg_value / 0x3FF)) * envelopeHeightScale);

      aegVertices.set([
        sampleIdx1, y1 - lineThicknessY, 0.298, 0.686, 0.314, 0.7,
        sampleIdx2, y2 - lineThicknessY, 0.298, 0.686, 0.314, 0.7,
        sampleIdx1, y1 + lineThicknessY, 0.298, 0.686, 0.314, 0.7,
        sampleIdx2, y2 - lineThicknessY, 0.298, 0.686, 0.314, 0.7,
        sampleIdx2, y2 + lineThicknessY, 0.298, 0.686, 0.314, 0.7,
        sampleIdx1, y1 + lineThicknessY, 0.298, 0.686, 0.314, 0.7
      ], aegOffset);
      aegOffset += 36;
    }
    aegBufferRef.current = renderer.createVertexBuffer(aegVertices);

    // FEG envelope (orange) - positioned below center
    const fegVertices = new Float32Array((numFrames - 1) * 6 * 6);
    let fegOffset = 0;

    for (let i = 0; i < numFrames - 1; i++) {
      const frameData1 = new SgcFrameData(sgcBinaryData, i);
      const frameData2 = new SgcFrameData(sgcBinaryData, i + 1);
      const channelData1 = frameData1.getChannel(channelIndex);
      const channelData2 = frameData2.getChannel(channelIndex);

      // X: sample indices (make line wider by extending 1.5 samples)
      const sampleIdx1 = i;
      const sampleIdx2 = i + 1.5;

      // Y: normalized [0, 1], FEG uses full height
      // FEG: 0x1FFF (max) should be at top (0.0), 0x0000 (min) should be at bottom (1.0)
      const fegTopY = 0.0; // Top of canvas
      const y1 = fegTopY + ((1 - (channelData1.feg_value / 0x1FFF)) * envelopeHeightScale);
      const y2 = fegTopY + ((1 - (channelData2.feg_value / 0x1FFF)) * envelopeHeightScale);

      fegVertices.set([
        sampleIdx1, y1 - lineThicknessY, 1.0, 0.596, 0.0, 0.7,
        sampleIdx2, y2 - lineThicknessY, 1.0, 0.596, 0.0, 0.7,
        sampleIdx1, y1 + lineThicknessY, 1.0, 0.596, 0.0, 0.7,
        sampleIdx2, y2 - lineThicknessY, 1.0, 0.596, 0.0, 0.7,
        sampleIdx2, y2 + lineThicknessY, 1.0, 0.596, 0.0, 0.7,
        sampleIdx1, y1 + lineThicknessY, 1.0, 0.596, 0.0, 0.7
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

      // Calculate number of frames from buffer size
      const BYTES_PER_FRAME = 8192; // 64 channels × 128 bytes
      const numFrames = sgcBinaryData.byteLength / BYTES_PER_FRAME;

      // Get current pan/zoom values
      const { scrollOffsetX, zoomLevel } = panZoomRef.current;

      // Clear offscreen canvas (renderer will auto-resize to match canvas dimensions)
      renderer.clear(width, height);

      // Render waveform (cached)
      if (waveformBufferRef.current) {
        const vertexCount = viewMode === 'post-volpan' ? numFrames * 3 * 6 : numFrames * 6;
        renderer.render(waveformBufferRef.current, vertexCount, scrollOffsetX, zoomLevel, numFrames);
      }

      // Render AEG envelope (cached)
      if (aegBufferRef.current) {
        renderer.render(aegBufferRef.current, (numFrames - 1) * 6, scrollOffsetX, zoomLevel, numFrames);
      }

      // Render FEG envelope (cached)
      if (fegBufferRef.current) {
        renderer.render(fegBufferRef.current, (numFrames - 1) * 6, scrollOffsetX, zoomLevel, numFrames);
      }

      // Clean up old position buffers
      positionBuffersRef.current.forEach(buffer => renderer.destroyVertexBuffer(buffer));
      positionBuffersRef.current = [];

      // Render position indicators (not cached, recreated each frame)
      if (hoverPosition !== null) {
        // Use sample index for X, normalized Y for full height
        const hoverSampleIdx = hoverPosition;
        const lineWidth = 1; // 1 sample wide
        const hoverVertices = new Float32Array([
          hoverSampleIdx - lineWidth, 0, 1, 1, 1, 0.4,
          hoverSampleIdx + lineWidth, 0, 1, 1, 1, 0.4,
          hoverSampleIdx - lineWidth, 1, 1, 1, 1, 0.4,
          hoverSampleIdx + lineWidth, 0, 1, 1, 1, 0.4,
          hoverSampleIdx + lineWidth, 1, 1, 1, 1, 0.4,
          hoverSampleIdx - lineWidth, 1, 1, 1, 1, 0.4
        ]);
        const hoverBuffer = renderer.createVertexBuffer(hoverVertices);
        positionBuffersRef.current.push(hoverBuffer);
        renderer.render(hoverBuffer, 6, 0, 1, numFrames); // No pan/zoom for indicators
      }

      const playbackSampleIdx = playbackPosition;
      const lineWidth = 1; // 1 sample wide
      const playbackVertices = new Float32Array([
        playbackSampleIdx - lineWidth, 0, 1, 0.596, 0, 0.9,
        playbackSampleIdx + lineWidth, 0, 1, 0.596, 0, 0.9,
        playbackSampleIdx - lineWidth, 1, 1, 0.596, 0, 0.9,
        playbackSampleIdx + lineWidth, 0, 1, 0.596, 0, 0.9,
        playbackSampleIdx + lineWidth, 1, 1, 0.596, 0, 0.9,
        playbackSampleIdx - lineWidth, 1, 1, 0.596, 0, 0.9
      ]);
      const playbackBuffer = renderer.createVertexBuffer(playbackVertices);
      positionBuffersRef.current.push(playbackBuffer);
      renderer.render(playbackBuffer, 6, 0, 1, numFrames); // No pan/zoom for indicators

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
      const newWidth = rect.width;
      const newHeight = rect.height;

      if (newWidth > 0 && newHeight > 0) {
        canvas.width = newWidth;
        canvas.height = newHeight;
        // Trigger a render after canvas is resized
        queueGraphRerender();
      }
    };

    // Use ResizeObserver to detect container size changes (fires initially when observed)
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
