import { useCallback, useEffect, useRef, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Stack,
  Typography,
} from "@mui/material";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import { deflate, inflate } from "pako";
import { Panel } from "../layout/Panel";
import { DspAssemblyEditor } from "../components/DspAssemblyEditor";
import { WaveformPlotter } from "../components/WaveformPlotter";
import type { WaveformPlotterRef } from "../components/WaveformPlotter";
import { aicaDsp } from "../../lib/aicaDsp";
import { assembleSource, writeRegisters, decodeInst, disassembleDesc } from "../dsp/dspUtils";
import { DSP_DEFAULT_SOURCE } from "../dsp/defaultSource";

const DSP_SOURCE_STORAGE_KEY = "nulldc-debugger-dsp-source";
const DSP_SHARE_URL = "https://skmp.gitlab.io/aica-dsp-playground/";

const encodeSource = (value: string) => {
  const bytes = deflate(new TextEncoder().encode(value));
  let binary = "";
  bytes.forEach((byte: number) => {
    binary += String.fromCharCode(byte);
  });
  return btoa(binary);
};

const decodeSource = (value: string) => {
  const binary = atob(value);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder().decode(inflate(bytes));
};

const resolveInitialSource = () => {
  if (typeof window === "undefined") {
    return DSP_DEFAULT_SOURCE;
  }
  const params = new URLSearchParams(window.location.search);
  if (params.has("source")) {
    try {
      return decodeSource(params.get("source") ?? "");
    } catch (error) {
      console.error("Failed to decode source parameter", error);
      return DSP_DEFAULT_SOURCE;
    }
  }
  try {
    const storedSource = window.localStorage.getItem(DSP_SOURCE_STORAGE_KEY);
    if (storedSource && storedSource !== "") {
      return storedSource;
    }
  } catch (error) {
    console.warn("Failed to read DSP source from storage", error);
  }
  return DSP_DEFAULT_SOURCE;
};

const NOTE_FREQUENCIES: Record<string, number> = {
  C3: 130.81,
  D3: 146.83,
  E3: 164.81,
  F3: 174.61,
  G3: 196.0,
  A3: 220.0,
  B3: 246.94,
  C4: 261.63,
  D4: 293.66,
  E4: 329.63,
  F4: 349.23,
  G4: 392.0,
  A4: 440.0,
  B4: 493.88,
  C5: 523.25,
  D5: 587.33,
  E5: 659.25,
  F5: 698.46,
};

const KEY_TO_NOTE: Record<string, string> = {
  z: "C3",
  x: "D3",
  c: "E3",
  v: "F3",
  b: "G3",
  n: "A3",
  m: "B3",
  ",": "C4",
  ".": "D4",
  "/": "E4",
  a: "C4",
  s: "D4",
  d: "E4",
  f: "F4",
  g: "G4",
  h: "A4",
  j: "B4",
  k: "C5",
  l: "D5",
  ";": "E5",
  "'": "F5",
};

export const DspPlaygroundPanel = () => {
  const [source, setSource] = useState(resolveInitialSource);
  const [error, setError] = useState<string | null>(null);
  const [audioPlaying, setAudioPlaying] = useState(false);
  const [wasmInitialized, setWasmInitialized] = useState(false);
  const [wavBuffer, setWavBuffer] = useState<DataView | null>(null);
  const [shareToken, setShareToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const frequency = useRef(0);
  const keysPressed = useRef(new Set<string>());
  const wavIndex = useRef(0);
  const phase = useRef(0);
  const amplitude = 0.5;

  const audioContextRef = useRef<AudioContext | null>(null);
  const scriptNodeRef = useRef<ScriptProcessorNode | null>(null);

  const inputPlotterRef = useRef<WaveformPlotterRef>(null);
  const outputPlotterRefs = useRef<Array<WaveformPlotterRef | null>>(
    Array(16).fill(null)
  );

  // Initialize WASM
  useEffect(() => {
    void aicaDsp.initialize().then(() => {
      setWasmInitialized(true);
    });
  }, []);

  // Save to localStorage
  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      window.localStorage.setItem(DSP_SOURCE_STORAGE_KEY, source);
    } catch (error) {
      console.warn("Failed to save DSP source to storage", error);
    }
  }, [source]);

  // Generate share token from source
  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      const encoded = encodeSource(source);
      setShareToken(encoded);
    } catch (error) {
      console.error("Failed to encode source", error);
      setShareToken(null);
    }
  }, [source]);

  const assembleAndWrite = useCallback(
    (newSource: string) => {
      if (!wasmInitialized) return;

      setError(null);
      try {
        const lines = newSource.split("\n");
        const parsedData = assembleSource(lines);
        writeRegisters(aicaDsp, parsedData);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
      }
    },
    [wasmInitialized]
  );

  useEffect(() => {
    if (wasmInitialized) {
      assembleAndWrite(source);
    }
  }, [source, wasmInitialized, assembleAndWrite]);

  const handleSourceChange = useCallback((newSource: string) => {
    setSource(newSource);
  }, []);

  const handleLoadRegs = useCallback(
    async (event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0];
      if (!file || !wasmInitialized) return;

      try {
        const arrayBuffer = await file.arrayBuffer();
        const dataView = new DataView(arrayBuffer);

        for (let i = 0; i < arrayBuffer.byteLength; i += 4) {
          const value = dataView.getUint32(i, true);
          aicaDsp.writeReg(i, value);
        }

        // Read back and generate source
        const lines: string[] = [];
        lines.push("# AICA-DSP");
        lines.push("");

        lines.push("# COEF");
        for (let i = 0; i < 128; i++) {
          const value = aicaDsp.readReg(0x3000 + i * 4);
          if (value) {
            lines.push(`COEF[${i}] = ${value}`);
          }
        }
        lines.push("");

        lines.push("# MADRS");
        for (let i = 0; i < 64; i++) {
          const value = aicaDsp.readReg(0x3200 + i * 4);
          if (value) {
            lines.push(`MADRS[${i}] = ${value}`);
          }
        }
        lines.push("");

        lines.push("# MEMS");
        for (let i = 0; i < 32; i++) {
          const low = aicaDsp.readReg(0x4400 + i * 8 + 0);
          const high = aicaDsp.readReg(0x4400 + i * 8 + 4);
          if (low || high) {
            lines.push(`MEMS_L[${i}] = ${low}`);
            lines.push(`MEMS_H[${i}] = ${high}`);
          }
        }
        lines.push("");

        lines.push("# MPRO");
        for (let i = 0; i < 128; i++) {
          const dwords = [
            aicaDsp.readReg(0x3000 + 0x400 + i * 4 * 4 + 0),
            aicaDsp.readReg(0x3000 + 0x400 + i * 4 * 4 + 4),
            aicaDsp.readReg(0x3000 + 0x400 + i * 4 * 4 + 8),
            aicaDsp.readReg(0x3000 + 0x400 + i * 4 * 4 + 12),
          ];
          const desc = decodeInst(dwords);
          const disasm = disassembleDesc(desc);
          if (disasm) {
            lines.push(`MPRO[${i}] = ${disasm}`);
          }
        }

        setSource(lines.join("\n"));
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(`Error loading registers: ${message}`);
      }

      // Reset file input
      event.target.value = "";
    },
    [wasmInitialized]
  );

  const handleDownloadRegs = useCallback(() => {
    if (!wasmInitialized) return;

    const buffer = new Uint8Array(0x8000);
    for (let offset = 0; offset < 0x8000; offset += 4) {
      const value = aicaDsp.readReg(offset);
      buffer[offset] = value & 0xff;
      buffer[offset + 1] = (value >> 8) & 0xff;
      buffer[offset + 2] = (value >> 16) & 0xff;
      buffer[offset + 3] = (value >> 24) & 0xff;
    }

    const blob = new Blob([buffer], { type: "application/octet-stream" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "dsp_memory_dump.bin";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [wasmInitialized]);

  const handleLoadWav = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    void file.arrayBuffer().then((buffer) => {
      setWavBuffer(new DataView(buffer));
      wavIndex.current = 0;
    });

    event.target.value = "";
  }, []);

  const handleStartAudio = useCallback(() => {
    if (!wasmInitialized) return;

    const audioContext = new (window.AudioContext || (window as any).webkitAudioContext)({
      sampleRate: 44100,
    });
    const scriptNode = audioContext.createScriptProcessor(1024, 0, 1);

    scriptNode.onaudioprocess = (event) => {
      const outputBuffer = event.outputBuffer.getChannelData(0);
      const sampleRate = 44100;
      const TWO_PI = Math.PI * 2;

      for (let i = 0; i < outputBuffer.length; i++) {
        // Generate sine wave sample
        let sample = amplitude * Math.sin(phase.current);

        if (wavBuffer) {
          const idx = wavIndex.current;
          if (idx < wavBuffer.byteLength - 1) {
            sample = wavBuffer.getInt16(idx, true) / 32768;
            wavIndex.current = (idx + 2) % wavBuffer.byteLength;
          }
        }

        const sampleInt = Math.round(sample * 32767);

        inputPlotterRef.current?.appendSample(sampleInt);

        // Write input to DSP
        for (let j = 0; j < 2; j++) {
          aicaDsp.writeReg(0x3000 + 0x1500 + 0 + j * 8, (sampleInt >> 0) & 0xf);
          aicaDsp.writeReg(0x3000 + 0x1500 + 4 + j * 8, (sampleInt >> 4) & 0xffff);
        }

        aicaDsp.step128();

        // Read DSP outputs
        let audioOut = 0;
        for (let j = 0; j < 16; j++) {
          let fxSampleInt = aicaDsp.readReg(0x3000 + 0x1580 + j * 4);
          fxSampleInt = fxSampleInt & 0xffff;
          if (fxSampleInt & 0x8000) {
            fxSampleInt |= 0xffff0000;
          }
          const fxSample = fxSampleInt / 32767;
          audioOut += fxSample;

          outputPlotterRefs.current[j]?.appendSample(fxSampleInt);
        }

        outputBuffer[i] = audioOut;

        // Update the phase
        phase.current += (TWO_PI * frequency.current) / sampleRate;

        // Keep phase in the range [0, TWO_PI] to avoid overflow
        if (phase.current >= TWO_PI) {
          phase.current -= TWO_PI;
        }
      }
    };

    scriptNode.connect(audioContext.destination);
    audioContextRef.current = audioContext;
    scriptNodeRef.current = scriptNode;
    setAudioPlaying(true);
  }, [wasmInitialized, wavBuffer]);

  const handleStopAudio = useCallback(() => {
    if (scriptNodeRef.current) {
      scriptNodeRef.current.disconnect();
    }
    if (audioContextRef.current) {
      void audioContextRef.current.close();
    }
    audioContextRef.current = null;
    scriptNodeRef.current = null;
    setAudioPlaying(false);
  }, []);

  // Keyboard MIDI handling
  useEffect(() => {
    if (!audioPlaying) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      // Check if typing in editor or input field
      const target = event.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.contentEditable === "true" ||
        target.closest(".monaco-editor")
      ) {
        return;
      }

      const note = KEY_TO_NOTE[event.key.toLowerCase()];
      if (note && !keysPressed.current.has(event.key)) {
        keysPressed.current.add(event.key);
        frequency.current = NOTE_FREQUENCIES[note];
        event.preventDefault(); // Prevent default browser behavior
      }
    };

    const handleKeyUp = (event: KeyboardEvent) => {
      const note = KEY_TO_NOTE[event.key.toLowerCase()];
      if (note && keysPressed.current.has(event.key)) {
        keysPressed.current.delete(event.key);
        if (keysPressed.current.size === 0) {
          frequency.current = 0;
        }
        event.preventDefault();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    document.addEventListener("keyup", handleKeyUp);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      document.removeEventListener("keyup", handleKeyUp);
    };
  }, [audioPlaying]);

  const handleShare = useCallback(async () => {
    if (!shareToken || typeof navigator === "undefined" || !navigator.clipboard) {
      return;
    }
    const url = `${DSP_SHARE_URL}?source=${encodeURIComponent(shareToken)}`;
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error("Failed to copy share URL", error);
    }
  }, [shareToken]);

  if (!wasmInitialized) {
    return (
      <Panel>
        <Box sx={{ p: 3, display: "flex", justifyContent: "center", alignItems: "center" }}>
          <Typography variant="body1" color="text.secondary">
            Loading DSP WASM module...
          </Typography>
        </Box>
      </Panel>
    );
  }

  return (
    <Panel>
      <Box
        sx={{
          p: 2,
          display: "flex",
          flexDirection: "column",
          gap: 2,
          height: "100%",
          overflow: "auto",
        }}
      >
        {/* Top controls */}
        <Stack direction="row" spacing={1} flexWrap="wrap" alignItems="center">
          {!audioPlaying ? (
            <Button variant="contained" size="small" color="success" onClick={handleStartAudio}>
              Start DSP
            </Button>
          ) : (
            <Button variant="contained" size="small" color="error" onClick={handleStopAudio}>
              Stop DSP
            </Button>
          )}
          <Button variant="outlined" size="small" component="label">
            Load Regs
            <input type="file" hidden accept=".bin" onChange={handleLoadRegs} />
          </Button>
          <Button variant="outlined" size="small" onClick={handleDownloadRegs}>
            Download Regs
          </Button>
          <Button variant="outlined" size="small" component="label">
            Load WAV
            <input type="file" hidden accept=".wav" onChange={handleLoadWav} />
          </Button>
          <Box sx={{ flexGrow: 1 }} />
          {shareToken && (
            <Button
              variant="outlined"
              size="small"
              onClick={handleShare}
              startIcon={<ContentCopyIcon fontSize="small" />}
              color={copied ? "success" : "primary"}
            >
              {copied ? "Copied!" : "Share"}
            </Button>
          )}
        </Stack>

        {audioPlaying && (
          <Alert severity="info" sx={{ py: 0.5 }}>
            <Typography variant="caption">
              <strong>Keyboard MIDI Active:</strong> z,x,c,v,b,n,m (lower octave) | a,s,d,f,g,h,j,k,l (higher octave)
            </Typography>
          </Alert>
        )}

        {/* Assembly Editor */}
        <DspAssemblyEditor value={source} onChange={handleSourceChange} error={error} height={250} />

        {/* Error display */}
        {error && (
          <Alert severity="error" onClose={() => setError(null)}>
            {error}
          </Alert>
        )}

        {/* Waveform visualizations */}
        <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
          <Typography variant="subtitle2">Input</Typography>
          <WaveformPlotter ref={inputPlotterRef} width={800} height={150} />

          <Typography variant="subtitle2" sx={{ mt: 2 }}>
            DSP Outputs (Channels 0-15)
          </Typography>
          <Box
            sx={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
              gap: 1,
            }}
          >
            {Array.from({ length: 16 }, (_, i) => (
              <Box key={i}>
                <Typography variant="caption" color="text.secondary">
                  Ch {i}
                </Typography>
                <WaveformPlotter
                  ref={(el) => {
                    outputPlotterRefs.current[i] = el;
                  }}
                  width={180}
                  height={100}
                  maxSamples={180}
                />
              </Box>
            ))}
          </Box>
        </Box>
      </Box>
    </Panel>
  );
};
