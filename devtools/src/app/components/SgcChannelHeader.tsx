import { Box, Typography } from "@mui/material";
import { HideOnHoverTooltip } from "./HideOnHoverTooltip";
import { SgcChannelData } from "../../lib/sgcChannelData";

interface SgcChannelHeaderProps {
  channelIndex: number;
  channelData: SgcChannelData;
}

export const SgcChannelHeader = ({ channelIndex, channelData }: SgcChannelHeaderProps) => {
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

  const channelLabel = `${channelIndex.toString().padStart(2, '0')}`;

  return (
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
  );
};
