import { Panel } from "../layout/Panel";
import { Box, Stack, IconButton, Tooltip, ToggleButton, ToggleButtonGroup } from "@mui/material";
import { SgcChannelView, type ChannelState, type SgcChannelViewHandle } from "../components/SgcChannelView";
import { useState, useCallback, useMemo, useRef, useEffect } from "react";
import FastRewindIcon from '@mui/icons-material/FastRewind';
import PlayArrowIcon from '@mui/icons-material/PlayArrow';
import StopIcon from '@mui/icons-material/Stop';

// Mock data generator for SGC channel data
const generateMockChannelData = (channelIndex: number) => {
  // Use channel index as seed for consistent but varied data
  const seed = channelIndex * 7919; // Prime number for better distribution

  // Simple seeded random function
  const seededRandom = (min: number, max: number, offset: number = 0) => {
    const x = Math.sin(seed + offset) * 10000;
    const normalized = x - Math.floor(x);
    return Math.floor(normalized * (max - min + 1)) + min;
  };

  const formats = ['PCM8', 'PCM16', 'ADPCM', 'ADPCM-L'] as const;

  return {
    sampleBase: seededRandom(0, 0x7FFFFF, 1),
    format: formats[seededRandom(0, 3, 2)],
    loopStart: seededRandom(0, 30000, 3),
    loopEnd: seededRandom(30000, 65535, 4),
    looped: seededRandom(0, 1, 5) === 1,
    playhead: seededRandom(0, 65535, 6),
    octave: seededRandom(-8, 7, 7),
    fns: seededRandom(0, 1023, 8),
    volume: seededRandom(0, 15, 9),
    panL: seededRandom(0, 15, 10),
    panR: seededRandom(0, 15, 11),
    dspVolume: seededRandom(0, 15, 12),
    dspChannel: seededRandom(0, 15, 13),
    // AEG (Amplitude Envelope Generator)
    aegAttack: seededRandom(0, 31, 14),
    aegDecay1: seededRandom(0, 31, 15),
    aegDecay2: seededRandom(0, 31, 16),
    aegRelease: seededRandom(0, 31, 17),
    aegDecayLevel: seededRandom(0, 31, 18),
    // FEG (Filter Envelope Generator)
    fegAttack: seededRandom(0, 31, 19),
    fegDecay1: seededRandom(0, 31, 20),
    fegDecay2: seededRandom(0, 31, 21),
    fegRelease: seededRandom(0, 31, 22),
    fegDecayLevel: seededRandom(0, 31, 23),
  };
};

export const AudioPanel = () => {
  // Channel states: 0 = normal, 1 = muted, 2 = soloed
  const [channelStates, setChannelStates] = useState<ChannelState[]>(Array(64).fill(0));
  const [isPlaying, setIsPlaying] = useState(false);
  const [showFilter, setShowFilter] = useState<'all' | 'active'>('all');
  const [hoverPlayheadX, setHoverPlayheadX] = useState<number | null>(null);
  const [stickyPlayheadX, setStickyPlayheadX] = useState<number>(0); // Position in normalized space (0-1)
  const [zoomLevel, setZoomLevel] = useState<number>(1); // 1 = 100%, 2 = 200%, etc.
  const [scrollOffsetX, setScrollOffsetX] = useState<number>(0); // Horizontal scroll offset when zoomed

  const [isDragging, setIsDragging] = useState<boolean>(false);

  const channelListRef = useRef<HTMLDivElement>(null);
  const animationFrameRef = useRef<number | undefined>(undefined);
  const dragStartXRef = useRef<number>(0);
  const dragStartScrollRef = useRef<number>(0);
  const channelRefsRef = useRef<Map<number, SgcChannelViewHandle>>(new Map());

  // Generate mock data for all 64 channels (memoized)
  const channelData = useMemo(() => {
    return Array.from({ length: 64 }, (_, i) => generateMockChannelData(i));
  }, []);

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
    setStickyPlayheadX(0); // Set to normalized position 0
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

  // Handle mouse move for both hover playhead and dragging
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

    // Handle hover playhead
    const rect = container.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const canvasWidth = getCanvasWidth();

    // Account for scroll offset and zoom
    const normalizedX = (x + scrollOffsetX - 8) / (canvasWidth * zoomLevel);
    const constrainedNormalized = Math.max(0, Math.min(normalizedX, 1));

    // Convert back to screen space for display
    const displayX = constrainedNormalized * canvasWidth * zoomLevel - scrollOffsetX + 8;
    setHoverPlayheadX(displayX);
  }, [getCanvasWidth, getMaxScrollOffset, zoomLevel, scrollOffsetX, isDragging]);

  // Handle mouse up to end drag
  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  // Handle mouse leave to hide hover playhead and end drag
  const handleMouseLeave = useCallback(() => {
    setHoverPlayheadX(null);
    setIsDragging(false);
  }, []);

  // Handle click to set sticky playhead
  const handleClick = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (isDragging) return; // Don't set playhead if we were dragging

    const container = channelListRef.current;
    if (!container) return;

    const rect = container.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const canvasWidth = getCanvasWidth();

    // Convert to normalized position (0-1)
    const normalizedX = (x + scrollOffsetX - 8) / (canvasWidth * zoomLevel);
    const constrainedNormalized = Math.max(0, Math.min(normalizedX, 1));
    setStickyPlayheadX(constrainedNormalized);
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

  // Update all channels when playhead positions change
  useEffect(() => {
    const container = channelListRef.current;
    if (!container) return;

    const canvasWidth = getCanvasWidth();

    // Convert hover playhead from screen space to normalized (0-1)
    let hoverNormalized: number | null = null;
    if (hoverPlayheadX !== null) {
      hoverNormalized = (hoverPlayheadX + scrollOffsetX - 8) / (canvasWidth * zoomLevel);
      hoverNormalized = Math.max(0, Math.min(hoverNormalized, 1));
    }

    // stickyPlayheadX is already normalized (0-1)
    const stickyNormalized = stickyPlayheadX;

    channelRefsRef.current.forEach((channelHandle) => {
      channelHandle.setPlayheads(hoverNormalized, stickyNormalized);
    });
  }, [hoverPlayheadX, stickyPlayheadX, scrollOffsetX, zoomLevel, getCanvasWidth]);

  // Animate sticky playhead when playing (in normalized space 0-1)
  useEffect(() => {
    if (!isPlaying) {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      return;
    }

    const startTime = Date.now();
    const startPos = stickyPlayheadX;
    const speed = 0.1; // 10% per second (normalized speed)

    const animate = () => {
      const elapsed = (Date.now() - startTime) / 1000;
      const newPos = startPos + elapsed * speed;

      // Loop within normalized bounds (0-1)
      const loopedPos = newPos % 1.0;

      setStickyPlayheadX(loopedPos);
      animationFrameRef.current = requestAnimationFrame(animate);
    };

    animationFrameRef.current = requestAnimationFrame(animate);

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, [isPlaying, stickyPlayheadX]);

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
                All ({channelData.length})
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
                data={channelData[i]}
              />
            ))}
          </Stack>
        </Box>
      </Box>
    </Panel>
  );
};
