import { Box, IconButton, Typography } from "@mui/material";
import VolumeUpIcon from '@mui/icons-material/VolumeUp';
import RadioButtonUncheckedIcon from '@mui/icons-material/RadioButtonUnchecked';
import GraphicEqIcon from '@mui/icons-material/GraphicEq';
import TuneIcon from '@mui/icons-material/Tune';
import InputIcon from '@mui/icons-material/Input';
import { useRef, useEffect, memo, useImperativeHandle, forwardRef, useState } from "react";
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
  }, [channelIndex, sgcBinaryData, viewMode]);

  // Canvas rendering - no useCallback, no dependencies
  const renderCanvas = () => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const { width, height } = canvas;
    const { scrollOffsetX, zoomLevel } = panZoomRef.current;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Save context state
    ctx.save();

    // Apply pan and zoom transformations
    ctx.translate(-scrollOffsetX, 0);
    ctx.scale(zoomLevel, 1);

    // Calculate the zoomed width for drawing
    const zoomedWidth = width;

    // Draw waveform (SoundCloud style - filled from center)
    const waveform = waveformDataRef.current;

    if (viewMode === 'post-volpan') {
      // Post-volpan mode - 3 waveforms (left, right, DSP)
      const topCenterY = height / 6;
      const middleCenterY = height / 2;
      const bottomCenterY = (height * 5) / 6;
      const waveHeight = height * 0.12;

      // For post-volpan, extract separate L/R/DSP samples from SGC data
      const numFrames = waveform.length;

      for (let i = 0; i < numFrames; i++) {
        const x = (i / (numFrames - 1)) * zoomedWidth;
        const barWidth = Math.max(1, zoomedWidth / numFrames);

        // Get actual L/R/DSP samples from SGC data
        const frameData = new SgcFrameData(sgcBinaryData, i);
        const frameChannelData = frameData.getChannel(channelIndex);
        const leftAmp = frameChannelData.sample_left / 32768.0;
        const rightAmp = frameChannelData.sample_right / 32768.0;
        const dspAmp = frameChannelData.sample_dsp / 32768.0;

        // Draw left channel
        ctx.fillStyle = '#1976d2';
        ctx.globalAlpha = 0.8;
        const leftHeight = Math.abs(leftAmp * waveHeight);
        if (leftAmp >= 0) {
          ctx.fillRect(x, topCenterY - leftHeight, barWidth, leftHeight);
        } else {
          ctx.fillRect(x, topCenterY, barWidth, leftHeight);
        }

        // Draw right channel
        ctx.fillStyle = '#d21976';
        ctx.globalAlpha = 0.8;
        const rightHeight = Math.abs(rightAmp * waveHeight);
        if (rightAmp >= 0) {
          ctx.fillRect(x, middleCenterY - rightHeight, barWidth, rightHeight);
        } else {
          ctx.fillRect(x, middleCenterY, barWidth, rightHeight);
        }

        // Draw DSP mix
        ctx.fillStyle = '#ff9800';
        ctx.globalAlpha = 0.8;
        const dspHeight = Math.abs(dspAmp * waveHeight);
        if (dspAmp >= 0) {
          ctx.fillRect(x, bottomCenterY - dspHeight, barWidth, dspHeight);
        } else {
          ctx.fillRect(x, bottomCenterY, barWidth, dspHeight);
        }
      }
    } else {
      // Pre-volpan or Input mode - single centered waveform
      const centerY = height / 2;
      const waveHeight = height * 0.4;

      ctx.fillStyle = viewMode === 'input' ? '#9c27b0' : '#1976d2';
      ctx.globalAlpha = 0.8;

      waveform.forEach((amplitude, i) => {
        const x = (i / (waveform.length - 1)) * zoomedWidth;
        const barWidth = Math.max(1, zoomedWidth / waveform.length);
        const barHeight = Math.abs(amplitude * waveHeight);

        if (amplitude >= 0) {
          ctx.fillRect(x, centerY - barHeight, barWidth, barHeight);
        } else {
          ctx.fillRect(x, centerY, barWidth, barHeight);
        }
      });
    }

    ctx.globalAlpha = 1.0;

    // Draw envelope curves from actual SGC data
    const totalFrames = 1024;
    const centerY = height / 2;
    const envelopeHeight = height * 0.3;

    // Draw AEG envelope (green, slight offset up)
    ctx.strokeStyle = '#4caf50';
    ctx.lineWidth = 1.5;
    ctx.globalAlpha = 0.7;
    ctx.beginPath();

    for (let i = 0; i < totalFrames; i++) {
      const frameData = new SgcFrameData(sgcBinaryData, i);
      const frameChannelData = frameData.getChannel(channelIndex);
      const x = (i / (totalFrames - 1)) * zoomedWidth;
      // Normalize AEG value (0 to 0xFFFFFFFF) to 0.0-1.0
      const aegNormalized = frameChannelData.aeg_value / 0x3FF;
      const y = centerY - height * 0.15 - (aegNormalized * envelopeHeight);

      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    }
    ctx.stroke();

    // Draw FEG envelope (orange, slight offset down)
    ctx.strokeStyle = '#ff9800';
    ctx.lineWidth = 1.5;
    ctx.globalAlpha = 0.7;
    ctx.beginPath();

    for (let i = 0; i < totalFrames; i++) {
      const frameData = new SgcFrameData(sgcBinaryData, i);
      const frameChannelData = frameData.getChannel(channelIndex);
      const x = (i / (totalFrames - 1)) * zoomedWidth;
      // Normalize FEG value (0 to 0xFFFFFFFF) to 0.0-1.0
      const fegNormalized = frameChannelData.feg_value / 0x1FFF;
      const y = centerY + height * 0.15 - (fegNormalized * envelopeHeight);

      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    }
    ctx.stroke();
    ctx.globalAlpha = 1.0;

    // Restore context state
    ctx.restore();

    // Draw position indicators (after restore so they're not affected by pan/zoom transform)
    const numFrames = 1024;

    if (hoverPosition !== null) {
      // Convert sample index to normalized position
      const hoverX = (hoverPosition / (numFrames - 1)) * width;
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.4)';
      ctx.setLineDash([4, 4]);
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(hoverX, 0);
      ctx.lineTo(hoverX, height);
      ctx.stroke();
    }

    // Convert sample index to normalized position
    const playbackX = (playbackPosition / (numFrames - 1)) * width;
    ctx.strokeStyle = 'rgba(255, 152, 0, 0.9)';
    ctx.setLineDash([4, 4]);
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(playbackX, 0);
    ctx.lineTo(playbackX, height);
    ctx.stroke();
    ctx.setLineDash([]);
  };

  // Expose setPanZoom and setPositions methods to parent
  useImperativeHandle(ref, () => ({
    setPanZoom: (scrollOffsetX: number, zoomLevel: number) => {
      panZoomRef.current = { scrollOffsetX, zoomLevel };
      renderCanvas();
    },
    setPositions: (hover: number | null, playback: number) => {
      setHoverPosition(hover);
      setPlaybackPosition(playback);
    }
  }), []);

  // Update canvas size on container resize using ResizeObserver
  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const updateCanvasSize = () => {
      const rect = container.getBoundingClientRect();
      canvas.width = rect.width;
      canvas.height = rect.height;
      renderCanvas();
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
  }, []);

  // Re-render canvas when view mode or positions change
  useEffect(() => {
    renderCanvas();
  }, [viewMode, hoverPosition, playbackPosition]);

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
          <canvas
            ref={canvasRef}
            style={{
              width: '100%',
              height: '100%',
              display: 'block',
            }}
          />
        </Box>
      </Box>
    </Box>
  );
}));

SgcChannelView.displayName = 'SgcChannelView';
