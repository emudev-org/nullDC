import { Panel } from "../layout/Panel";
import { Box, Stack, IconButton, Tooltip, ToggleButton, ToggleButtonGroup } from "@mui/material";
import { SgcChannelView, type ChannelState, type SgcChannelViewHandle } from "../components/SgcChannelView";
import { useState, useCallback, useMemo, useRef, useEffect } from "react";
import FastRewindIcon from '@mui/icons-material/FastRewind';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import StopIcon from '@mui/icons-material/Stop';
import { useSessionStore } from "../../state/sessionStore";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";

export const AudioPanel = () => {
  // Channel states: 0 = normal, 1 = muted, 2 = soloed
  const [channelStates, setChannelStates] = useState<ChannelState[]>(Array(64).fill(0));
  const [isPlaying, setIsPlaying] = useState(false);
  const [showFilter, setShowFilter] = useState<'all' | 'active'>('all');
  const [hoverPosition, setHoverPosition] = useState<number | null>(null); // Sample index [0, 1024)
  const [playbackPosition, setPlaybackPosition] = useState<number>(0); // Sample index [0, 1024)
  const [zoomLevel, setZoomLevel] = useState<number>(1); // 1 = 100%, 2 = 200%, etc.
  const [scrollOffsetX, setScrollOffsetX] = useState<number>(0); // Horizontal scroll offset when zoomed

  const [isDragging, setIsDragging] = useState<boolean>(false);
  const [sgcBinaryData, setSgcBinaryData] = useState<ArrayBuffer | null>(null);

  const client = useSessionStore((state) => state.client);
  const initialized = useDebuggerDataStore((state) => state.initialized);
  const channelListRef = useRef<HTMLDivElement>(null);
  const animationFrameRef = useRef<number | undefined>(undefined);
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
      } catch (error) {
        console.error("Failed to fetch SGC frame data:", error);
      }
    };
    fetchData();
  }, [client, initialized]);

  // Mock active channels (for demo purposes, channels 0-7 are "active")
  const activeChannels = useMemo(() => new Set([0, 1, 2, 3, 4, 5, 6, 7]), []);

  const handleMuteToggle = useCallback((channelIndex: number) => {
    setChannelStates((prevStates) => {
      const newStates = [...prevStates];

      // Toggle between normal (0) and muted (1)
      if (newStates[channelIndex] === 1) {
        newStates[channelIndex] = 0; // Unmute
      } else {
        newStates[channelIndex] = 1; // Mute
      }

      return newStates;
    });
  }, []);

  const handleSoloToggle = useCallback((channelIndex: number) => {
    setChannelStates((prevStates) => {
      const newStates = [...prevStates];

      if (newStates[channelIndex] === 2) {
        // If this channel is already soloed, unsolo it (set to normal)
        newStates[channelIndex] = 0;
      } else {
        // Clear all other solos and solo this channel
        for (let i = 0; i < newStates.length; i++) {
          if (newStates[i] === 2) {
            newStates[i] = 0; // Clear other solos
          }
        }
        newStates[channelIndex] = 2; // Solo this channel
      }

      return newStates;
    });
  }, []);

  const handleRewind = useCallback(() => {
    setPlaybackPosition(0); // Set to sample index 0
  }, []);

  const handlePlayStop = useCallback(() => {
    setIsPlaying((prev) => !prev);
  }, []);

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

  // Handle mouse move for both hover position and dragging
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
      return;
    }

    // Handle hover position - convert to sample index [0, 1024)
    const rect = container.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const canvasWidth = getCanvasWidth();

    // Account for scroll offset and zoom
    const normalizedX = (x + scrollOffsetX - 8) / (canvasWidth * zoomLevel);
    const constrainedNormalized = Math.max(0, Math.min(normalizedX, 1));

    // Convert to sample index [0, 1024)
    const sampleIndex = Math.floor(constrainedNormalized * 1024);
    const constrainedIndex = Math.max(0, Math.min(sampleIndex, 1023));
    setHoverPosition(constrainedIndex);
  }, [getCanvasWidth, getMaxScrollOffset, zoomLevel, scrollOffsetX, isDragging]);

  // Handle mouse up to end drag
  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  // Handle mouse leave to hide hover position and end drag
  const handleMouseLeave = useCallback(() => {
    setHoverPosition(null);
    setIsDragging(false);
  }, []);

  // Handle click to set playback position
  const handleClick = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (isDragging) return; // Don't set position if we were dragging

    const container = channelListRef.current;
    if (!container) return;

    const rect = container.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const canvasWidth = getCanvasWidth();

    // Convert to normalized position (0-1)
    const normalizedX = (x + scrollOffsetX - 8) / (canvasWidth * zoomLevel);
    const constrainedNormalized = Math.max(0, Math.min(normalizedX, 1));

    // Convert to sample index [0, 1024)
    const sampleIndex = Math.floor(constrainedNormalized * 1024);
    const constrainedIndex = Math.max(0, Math.min(sampleIndex, 1023));
    setPlaybackPosition(constrainedIndex);
  }, [getCanvasWidth, zoomLevel, scrollOffsetX, isDragging]);

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

  // Filter channels based on showFilter setting
  const visibleChannels = useMemo(() => {
    if (showFilter === 'active') {
      return Array.from({ length: 64 }, (_, i) => i).filter((i) => activeChannels.has(i));
    }
    return Array.from({ length: 64 }, (_, i) => i);
  }, [showFilter, activeChannels]);

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

  // Animate playback position when playing (in sample indices [0, 1024))
  useEffect(() => {
    if (!isPlaying) {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      return;
    }

    const startTime = Date.now();
    const startPos = playbackPosition;
    const speed = 102.4; // 102.4 samples per second (10% per second for 1024 samples)

    const animate = () => {
      const elapsed = (Date.now() - startTime) / 1000;
      const newPos = startPos + elapsed * speed;

      // Loop within sample bounds [0, 1024)
      const loopedPos = Math.floor(newPos) % 1024;

      setPlaybackPosition(loopedPos);
      animationFrameRef.current = requestAnimationFrame(animate);
    };

    animationFrameRef.current = requestAnimationFrame(animate);

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, [isPlaying, playbackPosition]);

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
          {/* Transport Controls */}
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.5 }}>
            <Tooltip title="Rewind">
              <IconButton
                size="small"
                onClick={handleRewind}
                sx={{ width: 32, height: 32 }}
              >
                <FastRewindIcon />
              </IconButton>
            </Tooltip>
            <Tooltip title={isPlaying ? 'Stop' : 'Play'}>
              <IconButton
                size="small"
                onClick={handlePlayStop}
                sx={{
                  width: 32,
                  height: 32,
                  color: isPlaying ? 'error.main' : 'primary.main',
                }}
              >
                {isPlaying ? <StopIcon /> : <PlayArrowIcon />}
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
          onClick={handleClick}
          onWheel={handleWheel}
          sx={{
            flex: 1,
            overflowY: 'scroll',
            overflowX: 'hidden',
            minHeight: 0,
            position: 'relative',
            cursor: isDragging ? 'grabbing' : (zoomLevel > 1 ? 'grab' : 'crosshair'),
          }}
        >
          {sgcBinaryData ? (
            <Stack
              direction="column"
              spacing={0.5}
              sx={{
                p: 1,
              }}
            >
              {visibleChannels.map((i) => (
                <SgcChannelView
                  key={i}
                  ref={setChannelRef(i)}
                  channelIndex={i}
                  channelState={channelStates[i]}
                  onMuteToggle={handleMuteToggle}
                  onSoloToggle={handleSoloToggle}
                  sgcBinaryData={sgcBinaryData}
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
              Loading Data...
            </Box>
          )}
        </Box>
      </Box>
    </Panel>
  );
};
