import { Box } from "@mui/material";
import { useRef, memo, useImperativeHandle, forwardRef, useState } from "react";
import { SgcFrameData } from "../../lib/sgcChannelData";
import { SgcWaveformRenderer } from "./SgcWaveformRenderer";
import { SgcChannelHeader } from "./SgcChannelHeader";
import { SgcChannelSidebar } from "./SgcChannelSidebar";
import { SgcWaveformCanvas } from "./SgcWaveformCanvas";
import type { SgcWaveformCanvasHandle } from "./SgcWaveformCanvas";

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
  onHoverPositionChange: (position: number | null) => void;
  onPlaybackPositionChange: (position: number) => void;
  sgcBinaryData: ArrayBuffer;
  renderer: SgcWaveformRenderer;
}

export const SgcChannelView = memo(forwardRef<SgcChannelViewHandle, SgcChannelViewProps>(({
  channelIndex,
  channelState,
  onMuteToggle,
  onSoloToggle,
  onHoverPositionChange,
  onPlaybackPositionChange,
  sgcBinaryData,
  renderer,
}, ref) => {
  const canvasRef = useRef<SgcWaveformCanvasHandle>(null);
  const [viewMode, setViewMode] = useState<'pre-volpan' | 'post-volpan' | 'input'>('pre-volpan');
  const [hoverPosition, setHoverPosition] = useState<number | null>(null);
  const [playbackPosition, setPlaybackPosition] = useState<number>(0);

  // Get channel data from the active sample (hover position if available, otherwise playback position)
  const activeSampleIndex = hoverPosition ?? playbackPosition;
  const channelData = new SgcFrameData(sgcBinaryData, activeSampleIndex).getChannel(channelIndex);

  // Expose setPanZoom and setPositions methods to parent
  useImperativeHandle(ref, () => ({
    setPanZoom: (scrollOffsetX: number, zoomLevel: number) => {
      canvasRef.current?.setPanZoom(scrollOffsetX, zoomLevel);
    },
    setPositions: (hover: number | null, playback: number) => {
      setHoverPosition(hover);
      setPlaybackPosition(playback);
      canvasRef.current?.setPositions(hover, playback);
    }
  }), []);


  // Compute visual state based on channel state
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
      <SgcChannelHeader channelIndex={channelIndex} channelData={channelData} />

      {/* Bottom section - Action buttons and waveform */}
      <Box sx={{ display: 'flex', flexDirection: 'row', height: '8em', gap: 0.5 }}>
        {/* Left - Action buttons */}
        <SgcChannelSidebar
          channelIndex={channelIndex}
          channelState={channelState}
          viewMode={viewMode}
          onViewModeChange={setViewMode}
          onMuteToggle={onMuteToggle}
          onSoloToggle={onSoloToggle}
        />

        {/* Right - Waveform canvas */}
        <SgcWaveformCanvas
          ref={canvasRef}
          channelIndex={channelIndex}
          viewMode={viewMode}
          sgcBinaryData={sgcBinaryData}
          renderer={renderer}
          onHoverPositionChange={(position) => {
            setHoverPosition(position);
            onHoverPositionChange(position);
          }}
          onPlaybackPositionChange={(position) => {
            setPlaybackPosition(position);
            onPlaybackPositionChange(position);
          }}
        />
      </Box>
    </Box>
  );
}));

SgcChannelView.displayName = 'SgcChannelView';
