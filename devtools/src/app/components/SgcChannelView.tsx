import { Box, IconButton, Typography } from "@mui/material";
import VolumeUpIcon from '@mui/icons-material/VolumeUp';
import RadioButtonUncheckedIcon from '@mui/icons-material/RadioButtonUnchecked';
import GraphicEqIcon from '@mui/icons-material/GraphicEq';
import TuneIcon from '@mui/icons-material/Tune';
import InputIcon from '@mui/icons-material/Input';
import { useRef, useEffect, memo, useImperativeHandle, forwardRef, useState, useCallback } from "react";
import { HideOnHoverTooltip } from "./HideOnHoverTooltip";
import { SgcFrameData } from "../../lib/sgcChannelData";

// Channel state type: 0 = normal, 1 = muted, 2 = soloed
export type ChannelState = 0 | 1 | 2;

export interface SgcChannelViewHandle {
  setPanZoom: (scrollOffsetX: number, zoomLevel: number) => void;
  setPositions: (hoverPosition: number | null, playbackPosition: number) => void;
}

interface SgcChannelViewProps {
  channelIndex: number;
  channelState: ChannelState;
  onMuteToggle: (index: number) => void;
  onSoloToggle: (index: number) => void;
  sgcBinaryData: ArrayBuffer;
}

export const SgcChannelView = memo(forwardRef<SgcChannelViewHandle, SgcChannelViewProps>(({
  channelIndex,
  channelState,
  onMuteToggle,
  onSoloToggle,
  sgcBinaryData,
}, ref) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const waveformDataRef = useRef<number[]>([]);
  const panZoomRef = useRef<{ scrollOffsetX: number; zoomLevel: number }>({ scrollOffsetX: 0, zoomLevel: 1 });
  const [viewMode, setViewMode] = useState<'pre-volpan' | 'post-volpan' | 'input'>('pre-volpan');
  const [hoverPosition, setHoverPosition] = useState<number | null>(null);
  const [playbackPosition, setPlaybackPosition] = useState<number>(0);
  const [webglAvailable, setWebglAvailable] = useState<boolean>(true);

  // WebGL refs
  const glRef = useRef<WebGLRenderingContext | null>(null);
  const programRef = useRef<WebGLProgram | null>(null);
  const renderQueuedRef = useRef<boolean>(false);
  const animationFrameRef = useRef<number | undefined>(undefined);

  // Get channel data from the active sample (hover position if available, otherwise playback position)
  const activeSampleIndex = hoverPosition ?? playbackPosition;
  const channelData = new SgcFrameData(sgcBinaryData, activeSampleIndex).getChannel(channelIndex);

  // Helper function to get format string from PCMS value
  const getFormat = (pcms: number): string => {
    switch (pcms) {
      case 0: return 'PCM16';
      case 1: return 'PCM8';
      case 2: return 'ADPCM';
      case 3: return 'ADPCM-L';
      default: return 'PCM16';
    }
  };

  // Helper function to convert OCT to signed octave
  const getOctave = (oct: number): number => {
    // OCT is 4-bit, treat as signed: 0-7 = +0 to +7, 8-15 = -8 to -1
    return oct > 7 ? oct - 16 : oct;
  };

  const getSampleStep = (oct: number, fns: number, plfo: number): number => {
    if (oct > 7) {
      return (1024 + fns + plfo) >> (16-oct);
    } else {
      return (1024 + fns + plfo) << oct;
    }
  };

  const getSampleRate = (oct: number, fns: number, plfo: number): number => {
    const step = getSampleStep(oct, fns, plfo);
    return (44100 * (step/1024)) | 0;
  };

  const getRightPan = (DIPAN: number): number => {
    if (DIPAN & 0x10) {
      return DIPAN & 0xF;
    } else {
      return 0xF;
    }
  };

  const getLeftPan = (DIPAN: number): number => {
    if (DIPAN & 0x10) {
      return 0xF;
    } else {
      return DIPAN & 0xF;
    }
  };

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

  // WebGL shader sources
  const vertexShaderSource = `
    attribute vec2 a_position;
    attribute vec4 a_color;
    uniform vec2 u_resolution;
    uniform vec2 u_transform; // x: scrollOffsetX, y: zoomLevel
    varying vec4 v_color;

    void main() {
      // Apply pan and zoom
      vec2 transformed = vec2(
        (a_position.x * u_transform.y) - u_transform.x,
        a_position.y
      );

      // Convert from pixel coordinates to clip space (-1 to 1)
      vec2 clipSpace = (transformed / u_resolution) * 2.0 - 1.0;
      clipSpace.y = -clipSpace.y; // Flip Y axis

      gl_Position = vec4(clipSpace, 0.0, 1.0);
      v_color = a_color;
    }
  `;

  const fragmentShaderSource = `
    precision mediump float;
    varying vec4 v_color;

    void main() {
      gl_FragColor = v_color;
    }
  `;

  // WebGL rendering - setup the render function
  useEffect(() => {
    const renderCanvas = () => {
      const canvas = canvasRef.current;
      if (!canvas) {
        console.log('SgcChannelView: No canvas');
        return;
      }

      const gl = glRef.current;
      if (!gl) {
        console.log('SgcChannelView: No WebGL context');
        return;
      }

      const { width, height } = canvas;
      if (width === 0 || height === 0) {
        console.log('SgcChannelView: Canvas has zero size', { width, height });
        return;
      }

      const { scrollOffsetX, zoomLevel } = panZoomRef.current;

      // Set viewport
      gl.viewport(0, 0, width, height);

      // Clear canvas
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);

    const program = programRef.current;
    if (!program) {
      console.log('SgcChannelView: No program');
      return;
    }

    gl.useProgram(program);

    // Set uniforms
    const u_resolution = gl.getUniformLocation(program, 'u_resolution');
    const u_transform = gl.getUniformLocation(program, 'u_transform');
    gl.uniform2f(u_resolution, width, height);
    gl.uniform2f(u_transform, scrollOffsetX, zoomLevel);

    console.log('SgcChannelView: Rendering', { width, height, scrollOffsetX, zoomLevel, viewMode });

    const numFrames = 1024;
    const waveform = waveformDataRef.current;

    if (waveform.length === 0) {
      console.log('SgcChannelView: No waveform data');
      return;
    }

    // Helper to create and upload buffer data
    const createBuffer = (data: Float32Array) => {
      const buffer = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
      gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
      return buffer;
    };

    // Helper to draw triangles
    const drawTriangles = (buffer: WebGLBuffer | null, count: number) => {
      if (!buffer) return;

      const a_position = gl.getAttribLocation(program, 'a_position');
      const a_color = gl.getAttribLocation(program, 'a_color');

      gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
      gl.enableVertexAttribArray(a_position);
      gl.vertexAttribPointer(a_position, 2, gl.FLOAT, false, 24, 0);
      gl.enableVertexAttribArray(a_color);
      gl.vertexAttribPointer(a_color, 4, gl.FLOAT, false, 24, 8);

      gl.drawArrays(gl.TRIANGLES, 0, count);
    };

    // Draw waveforms
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

      const buffer = createBuffer(vertices);
      drawTriangles(buffer, vertices.length / 6);
      gl.deleteBuffer(buffer);
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

      const buffer = createBuffer(vertices);
      drawTriangles(buffer, vertices.length / 6);
      gl.deleteBuffer(buffer);
    }

    // Draw envelope curves
    const centerY = height / 2;
    const envelopeHeight = height * 0.3;

    // AEG envelope (green)
    // Pre-allocate Float32Array: (numFrames - 1) line segments * 6 vertices/segment * 6 floats/vertex
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

      // Line as thin quad
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
    let buffer = createBuffer(aegVertices);
    drawTriangles(buffer, aegVertices.length / 6);
    gl.deleteBuffer(buffer);

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
    buffer = createBuffer(fegVertices);
    drawTriangles(buffer, fegVertices.length / 6);
    gl.deleteBuffer(buffer);

    // Draw position indicators (not affected by pan/zoom - disable transform)
    gl.uniform2f(u_transform, 0, 1);

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
      buffer = createBuffer(hoverVertices);
      drawTriangles(buffer, 6);
      gl.deleteBuffer(buffer);
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
    buffer = createBuffer(playbackVertices);
    drawTriangles(buffer, 6);
    gl.deleteBuffer(buffer);
    };

    // Assign render function to ref
    renderCanvasRef.current = renderCanvas;

    // Trigger initial render
    renderCanvas();
  }, [sgcBinaryData, channelIndex, viewMode, hoverPosition, playbackPosition]);

  // WebGL initialization
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    // Try to get WebGL context
    const glContext = canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
    if (!glContext) {
      setWebglAvailable(false);
      return;
    }

    const gl = glContext as WebGLRenderingContext;
    glRef.current = gl;

    // Compile shaders
    const compileShader = (type: number, source: string) => {
      const shader = gl.createShader(type);
      if (!shader) return null;

      gl.shaderSource(shader, source);
      gl.compileShader(shader);

      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        console.error('Shader compile error:', gl.getShaderInfoLog(shader));
        gl.deleteShader(shader);
        return null;
      }

      return shader;
    };

    const vertexShader = compileShader(gl.VERTEX_SHADER, vertexShaderSource);
    const fragmentShader = compileShader(gl.FRAGMENT_SHADER, fragmentShaderSource);

    if (!vertexShader || !fragmentShader) {
      setWebglAvailable(false);
      return;
    }

    // Link program
    const program = gl.createProgram();
    if (!program) {
      setWebglAvailable(false);
      return;
    }

    gl.attachShader(program, vertexShader);
    gl.attachShader(program, fragmentShader);
    gl.linkProgram(program);

    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.error('Program link error:', gl.getProgramInfoLog(program));
      setWebglAvailable(false);
      return;
    }

    programRef.current = program;

    // Enable blending for transparency
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

    setWebglAvailable(true);

    // Cleanup
    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      if (program) {
        gl.deleteProgram(program);
      }
      if (vertexShader) {
        gl.deleteShader(vertexShader);
      }
      if (fragmentShader) {
        gl.deleteShader(fragmentShader);
      }
    };
  }, [vertexShaderSource, fragmentShaderSource]);

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


  // Compute button states declaratively
  const isMuted = channelState === 1;
  const isSoloed = channelState === 2;

  const channelLabel = `${channelIndex.toString().padStart(2, '0')}`;
  const isMutedOrNotSoloed = channelState === 1;

  return (
    <Box
      sx={{
        border: '1px solid',
        borderColor: 'divider',
        borderRadius: 1,
        p: 0.5,
        display: 'flex',
        flexDirection: 'column',
        filter: isMutedOrNotSoloed ? 'grayscale(100%)' : 'none',
        opacity: isMutedOrNotSoloed ? 0.5 : 1,
        transition: 'filter 0.2s, opacity 0.2s',
      }}
    >
      {/* Top bar - Channel number and info */}
      <Box sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 0.5,
        mb: 0.5,
        flexWrap: 'wrap',
        minHeight: '20px',
      }}>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ fontFamily: 'monospace', fontWeight: 'bold', fontSize: '0.75rem', minWidth: '16px', textAlign: 'center' }}
        >
          {channelLabel}
        </Typography>

        {/* Channel state indicators */}
        <HideOnHoverTooltip title="Start Address in Audio Ram">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            {channelData.SA.toString(16).toUpperCase().padStart(6, '0')}
          </Typography>
        </HideOnHoverTooltip>

        <HideOnHoverTooltip title="Channel Format">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            {getFormat(channelData.PCMS).padStart(7, '\u00A0')}
          </Typography>
        </HideOnHoverTooltip>
        |
        <HideOnHoverTooltip title="Looped Indicator">
          <Box
            sx={{
              width: 8,
              height: 8,
              borderRadius: '50%',
              bgcolor: channelData.LPCTL ? 'warning.main' : 'transparent',
              border: channelData.LPCTL ? 'none' : '1px solid',
              borderColor: 'text.secondary',
            }}
          />
        </HideOnHoverTooltip>

        <HideOnHoverTooltip title="Play Position">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            HEAD: {channelData.ca_current.toString().padStart(5, '\u00A0')}:{channelData.ca_fraction.toString().padStart(4, '\u00A0')}
          </Typography>
        </HideOnHoverTooltip>

        <HideOnHoverTooltip title="Loop Parameters">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
          [{channelData.LSA.toString().padStart(5, '\u00A0')}-{channelData.LEA.toString().padStart(5, '\u00A0')}]
          </Typography>
        </HideOnHoverTooltip>
        |
        <HideOnHoverTooltip title="Current Sample: Filtered (Prev, Next)">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            S: {channelData.sample_filtered.toString().padStart(6, '\u00A0')} ({channelData.sample_previous.toString().padStart(6, '\u00A0')}, {channelData.sample_current.toString().padStart(6, '\u00A0')})
          </Typography>
        </HideOnHoverTooltip>
        |
        <HideOnHoverTooltip title={`${getSampleRate(channelData.OCT, channelData.FNS, channelData.plfo_value)} hz / ${getSampleStep(channelData.OCT, channelData.FNS, channelData.plfo_value)}`}>
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
           PITCH: {getOctave(channelData.OCT) >= 0 ? '+' : ''}{getOctave(channelData.OCT)}/{channelData.FNS.toString().padEnd(4, '\u00A0')}
          </Typography>
        </HideOnHoverTooltip>
          |
        <HideOnHoverTooltip title="Volume(TL) Send Level(DISDL) PAN(DIPAN)">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            VOL: TL{channelData.TL.toString().padStart(3, '0')} S{channelData.DISDL.toString(16).toUpperCase()} L{getLeftPan(channelData.DIPAN).toString(16).toUpperCase()}/R{getRightPan(channelData.DIPAN).toString(16).toUpperCase()}
          </Typography>
        </HideOnHoverTooltip>
          |
        <HideOnHoverTooltip title="DSP Channel / Volume">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            DSP: {channelData.ISEL.toString().padStart(2, '0')}/{channelData.DISDL.toString(16).toUpperCase()}
          </Typography>
        </HideOnHoverTooltip>
        |
        <HideOnHoverTooltip title="Amplitude & Filter Envelope">
          <Typography
              variant="caption"
              color="text.secondary"
              sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
            >
            ENV:
          </Typography>
        </HideOnHoverTooltip>
        <HideOnHoverTooltip title="AEG">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            {channelData.aeg_value.toString(16).toUpperCase().padStart(3, '0')}
          </Typography>
        </HideOnHoverTooltip>

        <HideOnHoverTooltip title="FEG">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            {channelData.feg_value.toString(16).toUpperCase().padStart(4, '0')}
          </Typography>
        </HideOnHoverTooltip>
      </Box>

      {/* Bottom section - Action buttons and waveform */}
      <Box sx={{ display: 'flex', flexDirection: 'row', height: '8em', gap: 0.5 }}>
        {/* Left - Action buttons */}
        <Box sx={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 0.25,
          flexShrink: 0,
        }}>
          {/* View mode toggle buttons */}
          <HideOnHoverTooltip title="Pre-VolPan" placement="right">
            <IconButton
              size="small"
              onClick={() => setViewMode('pre-volpan')}
              sx={{
                width: 20,
                height: 20,
                minWidth: 20,
                minHeight: 20,
                p: 0,
                color: viewMode === 'pre-volpan' ? 'primary.main' : 'inherit',
                bgcolor: viewMode === 'pre-volpan' ? 'action.selected' : 'transparent',
              }}
            >
              <GraphicEqIcon sx={{ fontSize: 14 }} />
            </IconButton>
          </HideOnHoverTooltip>

          <HideOnHoverTooltip title="Post-VolPan (L/R/DSP)" placement="right">
            <IconButton
              size="small"
              onClick={() => setViewMode('post-volpan')}
              sx={{
                width: 20,
                height: 20,
                minWidth: 20,
                minHeight: 20,
                p: 0,
                color: viewMode === 'post-volpan' ? 'primary.main' : 'inherit',
                bgcolor: viewMode === 'post-volpan' ? 'action.selected' : 'transparent',
              }}
            >
              <TuneIcon sx={{ fontSize: 14 }} />
            </IconButton>
          </HideOnHoverTooltip>

          <HideOnHoverTooltip title="Input Waveform" placement="right">
            <IconButton
              size="small"
              onClick={() => setViewMode('input')}
              sx={{
                width: 20,
                height: 20,
                minWidth: 20,
                minHeight: 20,
                p: 0,
                mb: 0.5,
                color: viewMode === 'input' ? 'primary.main' : 'inherit',
                bgcolor: viewMode === 'input' ? 'action.selected' : 'transparent',
              }}
            >
              <InputIcon sx={{ fontSize: 14 }} />
            </IconButton>
          </HideOnHoverTooltip>

          <HideOnHoverTooltip title="Mute/Unmute" placement="right">
            <IconButton
              size="small"
              onClick={() => onMuteToggle(channelIndex)}
              sx={{
                width: 16,
                height: 16,
                minWidth: 16,
                minHeight: 16,
                p: 0.25,
                color: isMuted ? 'error.main' : 'inherit',
              }}
            >
              <VolumeUpIcon sx={{ fontSize: 12 }} />
            </IconButton>
          </HideOnHoverTooltip>

          <HideOnHoverTooltip title="Solo" placement="right">
            <IconButton
              size="small"
              onClick={() => onSoloToggle(channelIndex)}
              sx={{
                width: 16,
                height: 16,
                minWidth: 16,
                minHeight: 16,
                p: 0.25,
                color: isSoloed ? 'warning.main' : 'inherit',
              }}
            >
              <RadioButtonUncheckedIcon sx={{ fontSize: 12 }} />
            </IconButton>
          </HideOnHoverTooltip>
        </Box>

        {/* Right - Waveform canvas */}
        <Box ref={containerRef} sx={{ flex: 1, position: 'relative', minHeight: 0, overflow: 'hidden' }}>
          {!webglAvailable ? (
            <Box
              sx={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                color: 'error.main',
                fontSize: '0.9rem',
              }}
            >
              WebGL is not available
            </Box>
          ) : (
            <canvas
              ref={canvasRef}
              style={{
                width: '100%',
                height: '100%',
                display: 'block',
              }}
            />
          )}
        </Box>
      </Box>
    </Box>
  );
}));

SgcChannelView.displayName = 'SgcChannelView';
