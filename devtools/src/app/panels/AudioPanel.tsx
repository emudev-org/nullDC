import { Panel } from "../layout/Panel";
import { Box, Stack, IconButton, Tooltip, ToggleButton, ToggleButtonGroup } from "@mui/material";
import { SgcChannelView, type SgcChannelViewHandle } from "../components/SgcChannelView";
import { SgcWaveformRenderer } from "../components/SgcWaveformRenderer";
import { useState, useCallback, useMemo, useRef, useEffect } from "react";
import FiberManualRecordIcon from '@mui/icons-material/FiberManualRecord';
import { useSessionStore } from "../../state/sessionStore";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import { SgcFrameData } from "../../lib/sgcChannelData";

export const AudioPanel = () => {
  const [showFilter, setShowFilter] = useState<'all' | 'active'>('active');
  const [hoverPosition, setHoverPosition] = useState<number | null>(null); // Sample index [0, 1024)
  const [playbackPosition, setPlaybackPosition] = useState<number>(0); // Sample index [0, 1024)
  const [zoomLevel, setZoomLevel] = useState<number>(1); // 1 = 100%, 2 = 200%, etc.
  const [scrollOffsetX, setScrollOffsetX] = useState<number>(0); // Horizontal scroll offset when zoomed
  const [fullscreenChannel, setFullscreenChannel] = useState<number | null>(null); // Channel in fullscreen mode

  const [isDragging, setIsDragging] = useState<boolean>(false);
  const [sgcBinaryData, setSgcBinaryData] = useState<ArrayBuffer | null>(null);
  const [renderer, setRenderer] = useState<SgcWaveformRenderer | null>(null);
  const [webglError, setWebglError] = useState<string | null>(null);
  const [activeChannels, setActiveChannels] = useState<Set<number>>(new Set());
  const [noDataAvailable, setNoDataAvailable] = useState<boolean>(false);

  const client = useSessionStore((state) => state.client);
  const initialized = useDebuggerDataStore((state) => state.initialized);
  const channelListRef = useRef<HTMLDivElement>(null);
  const dragStartXRef = useRef<number>(0);
  const dragStartScrollRef = useRef<number>(0);
  const channelRefsRef = useRef<Map<number, SgcChannelViewHandle>>(new Map());

  // Fetch SGC frame data when debugger is initialized
  useEffect(() => {
    const fetchData = async () => {
      if (!client || !initialized) return;
      try {
        const arrayBuffer = await client.fetchSgcFrameData();
        setSgcBinaryData(arrayBuffer);
        setNoDataAvailable(false);

        // Detect active channels by scanning for non-zero AEG values
        const detectedActiveChannels = new Set<number>();
        for (let channel = 0; channel < 64; channel++) {
          let hasNonZeroAeg = false;
          for (let frame = 0; frame < 1024; frame++) {
            const frameData = new SgcFrameData(arrayBuffer, frame);
            const channelData = frameData.getChannel(channel);
            if (channelData.aeg_value !== 0) {
              hasNonZeroAeg = true;
              break;
            }
          }
          if (hasNonZeroAeg) {
            detectedActiveChannels.add(channel);
          }
        }
        setActiveChannels(detectedActiveChannels);
        console.log(`Detected active channels: ${Array.from(detectedActiveChannels).sort((a, b) => a - b).join(', ')}`);
      } catch (error) {
        console.error("Failed to fetch SGC frame data:", error);
        setNoDataAvailable(true);
        setSgcBinaryData(null);
      }
    };
    fetchData();
  }, [client, initialized]);

  // Initialize WebGL renderer
  useEffect(() => {
    try {
      const newRenderer = new SgcWaveformRenderer();
      setRenderer(newRenderer);
      setWebglError(null);

      // Cleanup on unmount
      return () => {
        newRenderer.destroy();
      };
    } catch (error) {
      console.error("Failed to initialize WebGL renderer:", error);
      setWebglError(error instanceof Error ? error.message : "Failed to initialize WebGL");
    }
  }, []);

  const handleFullscreenToggle = useCallback((channelIndex: number) => {
    setFullscreenChannel((prev) => (prev === channelIndex ? null : channelIndex));
  }, []);

  const handleRecord = useCallback(async () => {
    if (!client) return;

    try {
      // Request recording from server
      await client.recordSgcFrames();

      // Fetch the new data
      const arrayBuffer = await client.fetchSgcFrameData();
      setSgcBinaryData(arrayBuffer);
      setNoDataAvailable(false);

      // Detect active channels by scanning for non-zero AEG values
      const BYTES_PER_FRAME = 8192;
      const numFrames = arrayBuffer.byteLength / BYTES_PER_FRAME;
      const detectedActiveChannels = new Set<number>();
      for (let channel = 0; channel < 64; channel++) {
        let hasNonZeroAeg = false;
        for (let frame = 0; frame < numFrames; frame++) {
          const frameData = new SgcFrameData(arrayBuffer, frame);
          const channelData = frameData.getChannel(channel);
          if (channelData.aeg_value !== 0) {
            hasNonZeroAeg = true;
            break;
          }
        }
        if (hasNonZeroAeg) {
          detectedActiveChannels.add(channel);
        }
      }
      setActiveChannels(detectedActiveChannels);
      console.log(`Detected active channels: ${Array.from(detectedActiveChannels).sort((a, b) => a - b).join(', ')}`);
    } catch (error) {
      console.error("Failed to record SGC frame data:", error);
    }
  }, [client]);

  const handleFilterChange = useCallback(
    (_event: React.MouseEvent<HTMLElement>, newFilter: 'all' | 'active' | null) => {
      if (newFilter !== null) {
        setShowFilter(newFilter);
      }
    },
    []
  );

  // Get canvas width from the container (accounting for padding)
  const getCanvasWidth = useCallback(() => {
    const container = channelListRef.current;
    if (!container) return 0;
    return container.clientWidth - 16; // Account for padding (p: 1 = 8px on each side)
  }, []);

  // Get max scroll offset based on zoom
  const getMaxScrollOffset = useCallback(() => {
    const canvasWidth = getCanvasWidth();
    const zoomedWidth = canvasWidth * zoomLevel;
    return Math.max(0, zoomedWidth - canvasWidth);
  }, [getCanvasWidth, zoomLevel]);

  // Handle mouse down for drag start
  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (zoomLevel <= 1) return; // Only enable dragging when zoomed in

    setIsDragging(true);
    dragStartXRef.current = e.clientX;
    dragStartScrollRef.current = scrollOffsetX;
    e.preventDefault();
  }, [zoomLevel, scrollOffsetX]);

  // Handle mouse move for dragging
  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    const container = channelListRef.current;
    if (!container) return;

    // Handle dragging
    if (isDragging) {
      const deltaX = dragStartXRef.current - e.clientX;
      const newScrollOffset = dragStartScrollRef.current + deltaX;
      const maxScroll = getMaxScrollOffset();
      const constrainedScroll = Math.max(0, Math.min(newScrollOffset, maxScroll));
      setScrollOffsetX(constrainedScroll);
    }
  }, [getMaxScrollOffset, isDragging]);

  // Handle mouse up to end drag
  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  // Handle mouse leave to end drag
  const handleMouseLeave = useCallback(() => {
    setIsDragging(false);
  }, []);

  // Callback when a channel updates hover position
  const handleHoverPositionChange = useCallback((position: number | null) => {
    setHoverPosition(position);
  }, []);

  // Callback when a channel updates playback position
  const handlePlaybackPositionChange = useCallback((position: number) => {
    setPlaybackPosition(position);
  }, []);

  // Handle wheel event for shift+scroll zoom
  const handleWheel = useCallback((e: React.WheelEvent<HTMLDivElement>) => {
    if (!e.shiftKey) return;

    e.preventDefault();

    setZoomLevel((prevZoom) => {
      const delta = e.deltaY > 0 ? -0.1 : 0.1; // Scroll down = zoom out, scroll up = zoom in
      const newZoom = Math.max(1.0, Math.min(5, prevZoom + delta)); // Clamp between 1.0x and 5x (minimum = fit)
      return newZoom;
    });
  }, []);

  // Filter channels based on showFilter setting and fullscreen mode
  const visibleChannels = useMemo(() => {
    // If fullscreen mode is active, only show that channel
    if (fullscreenChannel !== null) {
      return [fullscreenChannel];
    }

    if (showFilter === 'active') {
      return Array.from({ length: 64 }, (_, i) => i).filter((i) => activeChannels.has(i));
    }
    return Array.from({ length: 64 }, (_, i) => i);
  }, [showFilter, activeChannels, fullscreenChannel]);

  // Create stable ref callback
  const setChannelRef = useCallback((index: number) => {
    return (handle: SgcChannelViewHandle | null) => {
      if (handle) {
        channelRefsRef.current.set(index, handle);
      } else {
        channelRefsRef.current.delete(index);
      }
    };
  }, []);

  // Update all channels when pan/zoom changes
  useEffect(() => {
    channelRefsRef.current.forEach((channelHandle) => {
      channelHandle.setPanZoom(scrollOffsetX, zoomLevel);
    });
  }, [scrollOffsetX, zoomLevel]);

  // Update all channels when positions change
  useEffect(() => {
    channelRefsRef.current.forEach((channelHandle) => {
      channelHandle.setPositions(hoverPosition, playbackPosition);
    });
  }, [hoverPosition, playbackPosition]);

  return (
    <Panel>
      <Box
        sx={{
          height: '100%',
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        {/* Action Bar */}
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            gap: 1,
            p: 1,
            borderBottom: '1px solid',
            borderColor: 'divider',
            flexShrink: 0,
          }}
        >
          {/* Record Control */}
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
            <Tooltip title="Run the emulator and record 1024 frames">
              <IconButton
                size="small"
                onClick={handleRecord}
                sx={{
                  width: 32,
                  height: 32,
                  color: 'error.main',
                }}
              >
                <FiberManualRecordIcon />
              </IconButton>
            </Tooltip>
          </Box>

          {/* Spacer */}
          <Box sx={{ flex: 1 }} />

          {/* Channel Filter */}
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <ToggleButtonGroup
              value={showFilter}
              exclusive
              onChange={handleFilterChange}
              size="small"
              sx={{
                height: 28,
              }}
            >
              <ToggleButton value="all" sx={{ px: 1.5, fontSize: '0.75rem' }}>
                All (64)
              </ToggleButton>
              <ToggleButton value="active" sx={{ px: 1.5, fontSize: '0.75rem' }}>
                Active ({activeChannels.size})
              </ToggleButton>
            </ToggleButtonGroup>
          </Box>
        </Box>

        {/* Channel List */}
        <Box
          ref={channelListRef}
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseLeave}
          onWheel={handleWheel}
          sx={{
            flex: 1,
            overflowY: fullscreenChannel !== null ? 'hidden' : 'scroll',
            overflowX: 'hidden',
            minHeight: 0,
            position: 'relative',
            cursor: isDragging ? 'grabbing' : (zoomLevel > 1 ? 'grab' : 'default'),
          }}
        >
          {webglError ? (
            <Box
              sx={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                color: 'error.main',
                fontSize: '1.2rem',
                flexDirection: 'column',
                gap: 1,
              }}
            >
              <div>WebGL Initialization Failed</div>
              <div style={{ fontSize: '0.9rem', color: 'text.secondary' }}>{webglError}</div>
            </Box>
          ) : noDataAvailable ? (
            <Box
              sx={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                color: 'text.secondary',
                fontSize: '1.2rem',
              }}
            >
              Record some data to display here
            </Box>
          ) : sgcBinaryData && renderer ? (
            <Stack
              direction="column"
              spacing={0.5}
              sx={{
                p: fullscreenChannel !== null ? 0 : 1,
                height: fullscreenChannel !== null ? '100%' : 'auto',
              }}
            >
              {visibleChannels.map((i) => (
                <SgcChannelView
                  key={i}
                  ref={setChannelRef(i)}
                  channelIndex={i}
                  isFullscreen={fullscreenChannel === i}
                  onFullscreenToggle={handleFullscreenToggle}
                  onHoverPositionChange={handleHoverPositionChange}
                  onPlaybackPositionChange={handlePlaybackPositionChange}
                  sgcBinaryData={sgcBinaryData}
                  renderer={renderer}
                />
              ))}
            </Stack>
          ) : (
            <Box
              sx={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                height: '100%',
                color: 'text.secondary',
                fontSize: '1.2rem',
              }}
            >
              Loading...
            </Box>
          )}
        </Box>
      </Box>
    </Panel>
  );
};
