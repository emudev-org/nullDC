import { Box, IconButton, Tooltip, Typography } from "@mui/material";
import VolumeUpIcon from '@mui/icons-material/VolumeUp';
import RadioButtonUncheckedIcon from '@mui/icons-material/RadioButtonUnchecked';
import CallSplitIcon from '@mui/icons-material/CallSplit';
import { useRef, useEffect, memo, useImperativeHandle, forwardRef, useState } from "react";

// Channel state type: 0 = normal, 1 = muted, 2 = soloed
export type ChannelState = 0 | 1 | 2;

export interface SgcChannelViewHandle {
  setPanZoom: (scrollOffsetX: number, zoomLevel: number) => void;
}

interface SgcChannelData {
  sampleBase: number;      // 23-bit memory address
  format: 'PCM8' | 'PCM16' | 'ADPCM' | 'ADPCM-L';
  loopStart: number;       // 0-65535
  loopEnd: number;         // 0-65535
  looped: boolean;
  playhead: number;        // 0-65535
  octave: number;          // +7 to -8
  fns: number;             // 0-1023
  volume: number;          // 0-15
  panL: number;            // 0-15
  panR: number;            // 0-15
  dspVolume: number;       // 0x0-0xF
  dspChannel: number;      // 0-15
  // Envelope parameters
  aegAttack: number;       // 0-31
  aegDecay1: number;       // 0-31
  aegDecay2: number;       // 0-31
  aegRelease: number;      // 0-31
  aegDecayLevel: number;   // 0-31
  fegAttack: number;       // 0-31
  fegDecay1: number;       // 0-31
  fegDecay2: number;       // 0-31
  fegRelease: number;      // 0-31
  fegDecayLevel: number;   // 0-31
}

interface SgcChannelViewProps {
  channelIndex: number;
  channelState: ChannelState;
  onMuteToggle: (index: number) => void;
  onSoloToggle: (index: number) => void;
  data: SgcChannelData;
}

export const SgcChannelView = memo(forwardRef<SgcChannelViewHandle, SgcChannelViewProps>(({
  channelIndex,
  channelState,
  onMuteToggle,
  onSoloToggle,
  data,
}, ref) => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const waveformDataRef = useRef<number[]>([]);
  const panZoomRef = useRef<{ scrollOffsetX: number; zoomLevel: number }>({ scrollOffsetX: 0, zoomLevel: 1 });
  const [isStereoSplit, setIsStereoSplit] = useState(false);

  // Generate semi-random waveform data (SoundCloud style)
  useEffect(() => {
    const numPoints = 100;
    const waveform: number[] = [];

    // Generate a random but smooth waveform
    const seed = channelIndex * 12345;
    let phase = seed;

    for (let i = 0; i < numPoints; i++) {
      // Combine multiple sine waves for organic look
      const t = i / numPoints;
      phase += 0.1 + Math.sin(seed + i * 0.5) * 0.05;
      const amplitude =
        Math.sin(phase) * 0.5 +
        Math.sin(phase * 2.3 + seed) * 0.3 +
        Math.sin(phase * 4.7 + seed * 2) * 0.2;

      waveform.push(amplitude);
    }

    waveformDataRef.current = waveform;
  }, [channelIndex]);

  // Draw ADSR envelope curve
  const drawEnvelope = (
    ctx: CanvasRenderingContext2D,
    width: number,
    height: number,
    attack: number,
    decay1: number,
    decay2: number,
    release: number,
    decayLevel: number,
    color: string,
    yOffset: number
  ) => {
    const centerY = height / 2 + yOffset;
    const maxHeight = height * 0.3;

    // Normalize envelope parameters (0-31 -> time proportions)
    const totalTime = attack + decay1 + decay2 + release + 10;
    const attackW = (attack / totalTime) * width;
    const decay1W = (decay1 / totalTime) * width;
    const decay2W = (decay2 / totalTime) * width;
    const releaseW = (release / totalTime) * width;

    const sustainLevel = (decayLevel / 31) * maxHeight;

    ctx.strokeStyle = color;
    ctx.lineWidth = 1.5;
    ctx.globalAlpha = 0.7;
    ctx.beginPath();

    let x = 0;

    // Attack phase
    ctx.moveTo(x, centerY);
    x += attackW;
    ctx.lineTo(x, centerY - maxHeight);

    // Decay 1 phase
    x += decay1W;
    ctx.lineTo(x, centerY - sustainLevel);

    // Decay 2 phase (sustain)
    x += decay2W;
    ctx.lineTo(x, centerY - sustainLevel * 0.8);

    // Release phase
    x += releaseW;
    ctx.lineTo(x, centerY);

    ctx.stroke();
    ctx.globalAlpha = 1.0;
  };

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

    if (isStereoSplit) {
      // Split mode - left channel on top half, right channel on bottom half
      const topCenterY = height / 4;
      const bottomCenterY = (height * 3) / 4;
      const waveHeight = height * 0.2;

      // Draw left channel (top half)
      ctx.fillStyle = '#1976d2';
      ctx.globalAlpha = 0.8;

      waveform.forEach((amplitude, i) => {
        const x = (i / (waveform.length - 1)) * zoomedWidth;
        const barWidth = Math.max(1, zoomedWidth / waveform.length);
        const barHeight = Math.abs(amplitude * waveHeight);

        if (amplitude >= 0) {
          ctx.fillRect(x, topCenterY - barHeight, barWidth, barHeight);
        } else {
          ctx.fillRect(x, topCenterY, barWidth, barHeight);
        }
      });

      // Draw right channel (bottom half)
      ctx.fillStyle = '#d21976';
      ctx.globalAlpha = 0.8;

      waveform.forEach((amplitude, i) => {
        const x = (i / (waveform.length - 1)) * zoomedWidth;
        const barWidth = Math.max(1, zoomedWidth / waveform.length);
        const barHeight = Math.abs(amplitude * waveHeight);

        if (amplitude >= 0) {
          ctx.fillRect(x, bottomCenterY - barHeight, barWidth, barHeight);
        } else {
          ctx.fillRect(x, bottomCenterY, barWidth, barHeight);
        }
      });
    } else {
      // Mono mode - centered waveform
      const centerY = height / 2;
      const waveHeight = height * 0.4;

      ctx.fillStyle = '#1976d2';
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

    // Draw AEG envelope (green, slight offset up)
    drawEnvelope(
      ctx,
      zoomedWidth,
      height,
      data.aegAttack,
      data.aegDecay1,
      data.aegDecay2,
      data.aegRelease,
      data.aegDecayLevel,
      '#4caf50',
      -height * 0.15
    );

    // Draw FEG envelope (orange, slight offset down)
    drawEnvelope(
      ctx,
      zoomedWidth,
      height,
      data.fegAttack,
      data.fegDecay1,
      data.fegDecay2,
      data.fegRelease,
      data.fegDecayLevel,
      '#ff9800',
      height * 0.15
    );

    // Restore context state
    ctx.restore();
  };

  // Expose setPanZoom method to parent
  useImperativeHandle(ref, () => ({
    setPanZoom: (scrollOffsetX: number, zoomLevel: number) => {
      panZoomRef.current = { scrollOffsetX, zoomLevel };
      renderCanvas();
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

  // Re-render canvas when stereo split mode changes
  useEffect(() => {
    renderCanvas();
  }, [isStereoSplit]);

  // Compute button states declaratively
  const isMuted = channelState === 1;
  const isSoloed = channelState === 2;

  const channelLabel = `${channelIndex.toString().padStart(2, '0')}`;
  const isMutedOrNotSoloed = channelState === 1;

  return (
    <Box
      ref={containerRef}
      sx={{
        border: '1px solid',
        borderColor: 'divider',
        borderRadius: 1,
        p: 0.5,
        height: '7em',
        display: 'flex',
        flexDirection: 'column',
        filter: isMutedOrNotSoloed ? 'grayscale(100%)' : 'none',
        opacity: isMutedOrNotSoloed ? 0.5 : 1,
        transition: 'filter 0.2s, opacity 0.2s',
      }}
    >
      {/* Action Line */}
      <Box sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 0.5,
        mb: 0.5,
        flexWrap: 'wrap',
        minHeight: '20px',
      }}>
        {/* Channel name and controls */}
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ fontFamily: 'monospace', fontWeight: 'bold' }}
        >
          {channelLabel}
        </Typography>

        {/* Mute/Solo buttons */}
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.25 }}>
          <Tooltip title="Stereo Split">
            <IconButton
              size="small"
              onClick={() => setIsStereoSplit(!isStereoSplit)}
              sx={{
                width: 16,
                height: 16,
                minWidth: 16,
                minHeight: 16,
                p: 0.25,
                color: isStereoSplit ? 'success.main' : 'inherit',
              }}
            >
              <CallSplitIcon sx={{ fontSize: 12 }} />
            </IconButton>
          </Tooltip>
          <Tooltip title="Mute/Unmute">
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
          </Tooltip>
          <Tooltip title="Solo">
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
          </Tooltip>
        </Box>

        {/* Channel state indicators */}
        <Tooltip title="Start Address in Audio Ram">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            {data.sampleBase.toString(16).toUpperCase().padStart(6, '0')}
          </Typography>
        </Tooltip>

        <Tooltip title="Channel Format">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            {data.format.padStart(7, '\u00A0')}
          </Typography>
        </Tooltip>
        |
        <Tooltip title="Play Position">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            Head: {data.playhead.toString().padStart(5, '\u00A0')}
          </Typography>
        </Tooltip>


        <Tooltip title="Loop Parameters">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
          [{data.loopStart.toString().padStart(5, '\u00A0')}-{data.loopEnd.toString().padStart(5, '\u00A0')}]
          </Typography>
        </Tooltip>
        
        <Tooltip title="Looped Indicator">
          <Box
            sx={{
              width: 8,
              height: 8,
              borderRadius: '50%',
              bgcolor: data.looped ? 'warning.main' : 'transparent',
              border: data.looped ? 'none' : '1px solid',
              borderColor: 'text.secondary',
            }}
          />
        </Tooltip>
          |
        <Tooltip title="44100 hz / 0x1000">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
           PITCH: {data.octave >= 0 ? '+' : ''}{data.octave}/{data.fns.toString().padEnd(4, '\u00A0')}
          </Typography>
        </Tooltip>
          |
        <Tooltip title="Volume(TL) Send Level(DISDL) PAN(DIPAN)">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            VOL: TL{data.volume.toString().padStart(3, '0')} S{data.volume.toString(16).toUpperCase()} L{data.panL.toString(16).toUpperCase()}/R{data.panR.toString(16).toUpperCase()}
          </Typography>
        </Tooltip>
          |
        <Tooltip title="DSP Channel / Volume">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            DSP: {data.dspChannel.toString().padStart(2, '0')}/{data.dspVolume.toString(16).toUpperCase()}
          </Typography>
        </Tooltip>
          |
        <Tooltip title="Amplitude & Filter Envelope">
          <Typography
              variant="caption"
              color="text.secondary"
              sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
            >
            ENV: 
          </Typography>
        </Tooltip>
        <Tooltip title="AEG">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            3FF
          </Typography>
        </Tooltip>

        <Tooltip title="FEG">
          <Typography
            variant="caption"
            color="text.secondary"
            sx={{ fontFamily: 'monospace', fontSize: '0.65rem' }}
          >
            1FFF8
          </Typography>
        </Tooltip>
      </Box>

      {/* Canvas */}
      <Box sx={{ flex: 1, position: 'relative', minHeight: 0, overflow: 'hidden' }}>
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
  );
}));

SgcChannelView.displayName = 'SgcChannelView';
