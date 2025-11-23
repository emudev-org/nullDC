import { Box, IconButton } from "@mui/material";
import GraphicEqIcon from '@mui/icons-material/GraphicEq';
import TuneIcon from '@mui/icons-material/Tune';
import InputIcon from '@mui/icons-material/Input';
import FullscreenIcon from '@mui/icons-material/Fullscreen';
import FullscreenExitIcon from '@mui/icons-material/FullscreenExit';
import { HideOnHoverTooltip } from "./HideOnHoverTooltip";

interface SgcChannelSidebarProps {
  channelIndex: number;
  viewMode: 'pre-volpan' | 'post-volpan' | 'input';
  isFullscreen: boolean;
  onViewModeChange: (mode: 'pre-volpan' | 'post-volpan' | 'input') => void;
  onFullscreenToggle: (index: number) => void;
}

export const SgcChannelSidebar = ({
  channelIndex,
  viewMode,
  isFullscreen,
  onViewModeChange,
  onFullscreenToggle,
}: SgcChannelSidebarProps) => {

  return (
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
          onClick={() => onViewModeChange('pre-volpan')}
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
          onClick={() => onViewModeChange('post-volpan')}
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
          onClick={() => onViewModeChange('input')}
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

      <HideOnHoverTooltip title={isFullscreen ? "Exit Fullscreen" : "Fullscreen"} placement="right">
        <IconButton
          size="small"
          onClick={() => onFullscreenToggle(channelIndex)}
          sx={{
            width: 16,
            height: 16,
            minWidth: 16,
            minHeight: 16,
            p: 0.25,
            color: isFullscreen ? 'primary.main' : 'inherit',
          }}
        >
          {isFullscreen ? (
            <FullscreenExitIcon sx={{ fontSize: 12 }} />
          ) : (
            <FullscreenIcon sx={{ fontSize: 12 }} />
          )}
        </IconButton>
      </HideOnHoverTooltip>
    </Box>
  );
};
