import { Box, IconButton } from "@mui/material";
import VolumeUpIcon from '@mui/icons-material/VolumeUp';
import RadioButtonUncheckedIcon from '@mui/icons-material/RadioButtonUnchecked';
import GraphicEqIcon from '@mui/icons-material/GraphicEq';
import TuneIcon from '@mui/icons-material/Tune';
import InputIcon from '@mui/icons-material/Input';
import { HideOnHoverTooltip } from "./HideOnHoverTooltip";

// Channel state type: 0 = normal, 1 = muted, 2 = soloed
type ChannelState = 0 | 1 | 2;

interface SgcChannelSidebarProps {
  channelIndex: number;
  channelState: ChannelState;
  viewMode: 'pre-volpan' | 'post-volpan' | 'input';
  onViewModeChange: (mode: 'pre-volpan' | 'post-volpan' | 'input') => void;
  onMuteToggle: (index: number) => void;
  onSoloToggle: (index: number) => void;
}

export const SgcChannelSidebar = ({
  channelIndex,
  channelState,
  viewMode,
  onViewModeChange,
  onMuteToggle,
  onSoloToggle,
}: SgcChannelSidebarProps) => {
  const isMuted = channelState === 1;
  const isSoloed = channelState === 2;

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
  );
};
