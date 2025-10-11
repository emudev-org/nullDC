import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  IconButton,
  MenuItem,
  Paper,
  Select,
  Stack,
  Tab,
  Tabs,
  Tooltip,
  Typography,
} from "@mui/material";
import ContentCopyIcon from "@mui/icons-material/ContentCopy";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import PauseIcon from "@mui/icons-material/Pause";
import StopIcon from "@mui/icons-material/Stop";
import FastForwardIcon from "@mui/icons-material/FastForward";
import SkipNextIcon from "@mui/icons-material/SkipNext";
import DeleteIcon from "@mui/icons-material/Delete";
import HearingIcon from "@mui/icons-material/Hearing";
import VolumeUpIcon from "@mui/icons-material/VolumeUp";
import RadioButtonUncheckedIcon from "@mui/icons-material/RadioButtonUnchecked";
import KeyboardArrowDownIcon from "@mui/icons-material/KeyboardArrowDown";
import ArrowForwardIcon from "@mui/icons-material/ArrowForward";
import ZoomInIcon from "@mui/icons-material/ZoomIn";
import FullscreenIcon from "@mui/icons-material/Fullscreen";
import FullscreenExitIcon from "@mui/icons-material/FullscreenExit";
import { deflate, inflate } from "pako";
import { Panel } from "../layout/Panel";
import { DspAssemblyEditor } from "../components/DspAssemblyEditor";
import type { DspAssemblyEditorRef } from "../components/DspAssemblyEditor";
import { DspSourceEditor } from "../components/DspSourceEditor";
import type { DspSourceEditorRef } from "../components/DspSourceEditor";
import { WaveformPlotter } from "../components/WaveformPlotter";
import type { WaveformPlotterRef } from "../components/WaveformPlotter";
import { DisassemblyView, type DisassemblyViewConfig, type DisassemblyViewCallbacks } from "../components/DisassemblyView";
import type { DisassemblyLine } from "../../lib/debuggerSchema";
import { aicaDsp } from "../../lib/aicaDsp";
import { assembleSource, assembleSourceWithPreprocessing, writeRegisters, decodeInst, disassembleDesc } from "../dsp/dspUtils";
import { compileDspSource, CompilationError } from "../dsp/dspCompiler";
import { DSP_DEFAULT_SOURCE } from "../dsp/defaultSource";
import { DEFAULT_DSP_SOURCE } from "../dsp/defaultDspSource";

const DSP_SOURCE_STORAGE_KEY = "nulldc-debugger-dsp-source";
const DSP_ASSEMBLY_STORAGE_KEY = "nulldc-debugger-dsp-assembly";
const DSP_TAB_STORAGE_KEY = "nulldc-debugger-dsp-active-tab";
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
    const storedSource = window.localStorage.getItem(DSP_ASSEMBLY_STORAGE_KEY);
    if (storedSource && storedSource !== "") {
      return storedSource;
    }
  } catch (error) {
    console.warn("Failed to read DSP assembly from storage", error);
  }
  return DSP_DEFAULT_SOURCE;
};

const resolveInitialDspSource = () => {
  if (typeof window === "undefined") {
    return DEFAULT_DSP_SOURCE;
  }
  try {
    const storedSource = window.localStorage.getItem(DSP_SOURCE_STORAGE_KEY);
    if (storedSource && storedSource !== "") {
      return storedSource;
    }
  } catch (error) {
    console.warn("Failed to read DSP source from storage", error);
  }
  return DEFAULT_DSP_SOURCE;
};

const resolveInitialTab = (): "source" | "compiled" | "assembly" => {
  if (typeof window === "undefined") {
    return "source";
  }
  try {
    const storedTab = window.localStorage.getItem(DSP_TAB_STORAGE_KEY);
    if (storedTab === "source" || storedTab === "compiled" || storedTab === "assembly") {
      return storedTab;
    }
  } catch (error) {
    console.warn("Failed to read active tab from storage", error);
  }
  return "source";
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

type InputSource = "none" | "keyboard" | { type: "file"; fileId: string };

interface AudioFile {
  id: string;
  name: string;
  audioBuffer: AudioBuffer | null;
  loading?: boolean;
  error?: string;
}

export const DspPlaygroundPanel = () => {
  const [activeTab, setActiveTab] = useState<"source" | "compiled" | "assembly">(resolveInitialTab());
  const dspSourceRef = useRef(resolveInitialDspSource());
  const compiledAssemblyRef = useRef("");
  const assemblySourceRef = useRef(resolveInitialSource());
  const compiledEditorRef = useRef<DspAssemblyEditorRef | null>(null);
  const [audioPlaying, setAudioPlaying] = useState(false);
  const [audioPaused, setAudioPaused] = useState(false);
  const [currentDspStep, setCurrentDspStep] = useState(0);
  const [currentSample, setCurrentSample] = useState(0);
  const [breakpoints, setBreakpoints] = useState<Map<number, { id: number; enabled: boolean }>>(new Map());
  const breakpointIdCounterRef = useRef(0);
  const breakpointsRef = useRef<Map<number, { id: number; enabled: boolean }>>(breakpoints);
  const [goToPc, setGoToPc] = useState<{ address: number; fromUrl: boolean; highlight?: boolean } | undefined>(undefined);
  const registerElementsRef = useRef<Map<string, HTMLElement>>(new Map());
  const sourceExpandedRef = useRef(true);
  const debuggerExpandedRef = useRef(false);
  const waveformsExpandedRef = useRef(true);
  const audioFilesExpandedRef = useRef(true);
  const sourceContainerRef = useRef<HTMLElement | null>(null);
  const debuggerContainerRef = useRef<HTMLElement | null>(null);
  const waveformsContainerRef = useRef<HTMLElement | null>(null);
  const audioFilesContainerRef = useRef<HTMLElement | null>(null);
  const outputScale10xRef = useRef<Set<number>>(new Set());
  const [wasmInitialized, setWasmInitialized] = useState(false);
  const [binaryVersion, setBinaryVersion] = useState(0);
  const [editorsReady, setEditorsReady] = useState(false);
  const [shareToken, setShareToken] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [audioFiles, setAudioFiles] = useState<AudioFile[]>([]);
  const [playingFileId, setPlayingFileId] = useState<string | null>(null);
  const [loadingChannels, setLoadingChannels] = useState<Set<number>>(new Set());
  const inputSourcesRef = useRef<InputSource[]>(
    (() => {
      const sources: InputSource[] = Array(16).fill("none");
      sources[0] = "keyboard"; // Channel 0 defaults to keyboard
      return sources;
    })()
  );
  const audioFilesRef = useRef<AudioFile[]>(audioFiles);
  const frequency = useRef(0);
  const keysPressed = useRef(new Set<string>());
  const phase = useRef(0);
  const amplitude = 0.5;
  const lastKeyboardSample = useRef(0); // For smoothing keyboard input

  const audioContextRef = useRef<AudioContext | null>(null);
  const scriptNodeRef = useRef<ScriptProcessorNode | null>(null);
  const filePlaybackContextRef = useRef<AudioContext | null>(null);
  const filePlaybackSourceRef = useRef<AudioBufferSourceNode | null>(null);
  const tappedChannelRef = useRef<number | null>(null);
  const tapButtonRefs = useRef<Array<HTMLButtonElement | null>>(Array(16).fill(null));

  // Channel output states: 0 = normal, 1 = muted, 2 = soloed
  type ChannelState = 0 | 1 | 2;
  const channelStatesRef = useRef<ChannelState[]>(Array(16).fill(0));

  const muteButtonRefs = useRef<Array<HTMLButtonElement | null>>(Array(16).fill(null));
  const soloButtonRefs = useRef<Array<HTMLButtonElement | null>>(Array(16).fill(null));
  const zoomButtonRefs = useRef<Array<HTMLButtonElement | null>>(Array(16).fill(null));

  const sourceEditorRef = useRef<DspSourceEditorRef | null>(null);
  const editorRef = useRef<DspAssemblyEditorRef | null>(null);
  const editorContainerRef = useRef<HTMLDivElement | null>(null);
  const editorInnerContainerRef = useRef<HTMLDivElement | null>(null);
  const dspSectionRef = useRef<HTMLDivElement | null>(null);
  const editorFocusModeRef = useRef(false);
  const [editorFocusMode, setEditorFocusMode] = useState(false);
  const editorFocusButtonRef = useRef<HTMLButtonElement | null>(null);
  const inputPlotterRefs = useRef<Array<WaveformPlotterRef | null>>(
    Array(16).fill(null)
  );
  const outputPlotterRefs = useRef<Array<WaveformPlotterRef | null>>(
    Array(16).fill(null)
  );
  const mixPlotterRef = useRef<WaveformPlotterRef | null>(null);
  const inputContainerRefs = useRef<Array<HTMLElement | null>>(Array(16).fill(null));
  const outputContainerRefs = useRef<Array<HTMLElement | null>>(Array(16).fill(null));
  const inputSelectRefs = useRef<Array<HTMLSelectElement | null>>(Array(16).fill(null));

  // Sync breakpoints ref with state
  useEffect(() => {
    breakpointsRef.current = breakpoints;
  }, [breakpoints]);

  // Initialize WASM
  useEffect(() => {
    void aicaDsp.initialize().then(() => {
      setWasmInitialized(true);
    });
  }, []);


  // Save to localStorage (called manually, not reactive)
  const saveToLocalStorage = useCallback(() => {
    if (typeof window === "undefined") return;
    try {
      window.localStorage.setItem(DSP_SOURCE_STORAGE_KEY, dspSourceRef.current);
      window.localStorage.setItem(DSP_ASSEMBLY_STORAGE_KEY, assemblySourceRef.current);
      window.localStorage.setItem(DSP_TAB_STORAGE_KEY, activeTab);
    } catch (error) {
      console.warn("Failed to save DSP source to storage", error);
    }
  }, [activeTab]);

  // Generate share token from source (called manually, not reactive)
  const updateShareToken = useCallback(() => {
    if (typeof window === "undefined") return;
    try {
      // Share based on active content
      const contentToShare = activeTab === "assembly" ? assemblySourceRef.current : compiledAssemblyRef.current;
      const encoded = encodeSource(contentToShare);
      setShareToken(encoded);
    } catch (error) {
      console.error("Failed to encode source", error);
      setShareToken(null);
    }
  }, [activeTab]);

  const assembleAndWrite = useCallback(
    (newSource: string) => {
      if (!wasmInitialized) return;

      editorRef.current?.setStatus('assembling');
      editorRef.current?.setError(null);
      editorRef.current?.setErrors([]);
      try {
        // Use preprocessing version to support #define macros
        const parsedData = assembleSourceWithPreprocessing(newSource);
        writeRegisters(aicaDsp, parsedData);
        setBinaryVersion(v => v + 1);
        editorRef.current?.setStatus('assembled');
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        editorRef.current?.setError(message);
        setTimeout(() => {
          editorRef.current?.setStatus('error', 1);
        }, 0);
      }
    },
    [wasmInitialized]
  );

  const compileAndWrite = useCallback(
    (newDspSource: string) => {
      if (!wasmInitialized) return;

      // Set compiling status
      sourceEditorRef.current?.setStatus('compiling');
      compiledEditorRef.current?.setStatus('assembling');

      // Clear any existing errors
      if (sourceEditorRef.current) {
        sourceEditorRef.current.setErrors([]);
        sourceEditorRef.current.setError(null);
      }
      if (compiledEditorRef.current) {
        compiledEditorRef.current.setErrors([]);
        compiledEditorRef.current.setError(null);
      }

      try {
        // Compile high-level source to assembly
        const assembly = compileDspSource(newDspSource);

        // Store the compiled assembly and update the compiled editor
        compiledAssemblyRef.current = assembly;

        try {
          // Assemble and write to DSP
          const lines = assembly.split("\n");
          const parsedData = assembleSource(lines);
          writeRegisters(aicaDsp, parsedData);
          setBinaryVersion(v => v + 1);

          // Set compiled status
          sourceEditorRef.current?.setStatus('compiled');
          compiledEditorRef.current?.setStatus('assembled');
        } catch (assembleErr) {
          // Assembly failed - parse line number from error
          const message = assembleErr instanceof Error ? assembleErr.message : String(assembleErr);
          const lineMatch = message.match(/line (\d+)/i);
          const line = lineMatch ? parseInt(lineMatch[1], 10) : 1;

          const assemblyErrors = [{ line, message }];

          // Show "assembling failed" in source editor with callback to switch to compiled tab
          const switchToCompiledTab = () => {
            setActiveTab('compiled');
          };

          // Set errors in compiled editor
          compiledEditorRef.current?.setErrors(assemblyErrors);

          setTimeout(() => {
            sourceEditorRef.current?.setStatus('assembling-failed', assemblyErrors.length, switchToCompiledTab);
            compiledEditorRef.current?.setStatus('error', assemblyErrors.length);
          }, 0);
        }
      } catch (err) {
        if (err instanceof CompilationError) {
          // Handle multiple compilation errors
          // Set errors first so they're stored before status updates the tooltip
          sourceEditorRef.current?.setErrors(err.errors);

          // Generate error report for compiled editor
          const errorReport = [
            "# Failed to compile source",
            "",
            ...err.errors.map(e => `# Line ${e.line}: ${e.message}`)
          ].join('\n');
          compiledAssemblyRef.current = errorReport;

          // Then set status which will use the stored errors for the tooltip
          setTimeout(() => {
            sourceEditorRef.current?.setStatus('error', err.errors.length);
            compiledEditorRef.current?.setStatus('error', err.errors.length);
          }, 0);
        } else {
          // Handle other errors as before
          const message = err instanceof Error ? err.message : String(err);
          sourceEditorRef.current?.setError(message);

          // Generate error report for compiled editor
          const errorReport = [
            "# Failed to compile source",
            "",
            `# ${message}`
          ].join('\n');
          compiledAssemblyRef.current = errorReport;

          setTimeout(() => {
            sourceEditorRef.current?.setStatus('error', 1);
            compiledEditorRef.current?.setStatus('error', 1);
          }, 0);
        }
      }
    },
    [wasmInitialized]
  );

  // Initial compilation when WASM initializes and editors are ready
  useEffect(() => {
    if (wasmInitialized && editorsReady) {
      if (activeTab === "source" || activeTab === "compiled") {
        compileAndWrite(dspSourceRef.current);
      } else {
        assembleAndWrite(assemblySourceRef.current);
      }
      updateShareToken();
    }
  }, [wasmInitialized, editorsReady]);

  // Re-compile/re-assemble when switching tabs
  useEffect(() => {
    if (!wasmInitialized) return;

    // Use a small timeout to ensure the new editor is mounted
    const timer = setTimeout(() => {
      if (activeTab === "source" || activeTab === "compiled") {
        compileAndWrite(dspSourceRef.current);
      } else {
        assembleAndWrite(assemblySourceRef.current);
      }
      updateShareToken();
    }, 50);

    return () => clearTimeout(timer);
  }, [activeTab, wasmInitialized, compileAndWrite, assembleAndWrite, updateShareToken]);

  const handleEditorReady = useCallback(() => {
    setEditorsReady(true);
  }, []);

  const handleDspSourceChange = useCallback((newSource: string) => {
    dspSourceRef.current = newSource;
    compileAndWrite(newSource);
    saveToLocalStorage();
    updateShareToken();
  }, [compileAndWrite, saveToLocalStorage, updateShareToken]);

  const handleAssemblySourceChange = useCallback((newSource: string) => {
    assemblySourceRef.current = newSource;
    assembleAndWrite(newSource);
    saveToLocalStorage();
    updateShareToken();
  }, [assembleAndWrite, saveToLocalStorage, updateShareToken]);

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

        const newSource = lines.join("\n");
        assemblySourceRef.current = newSource;
        assembleAndWrite(newSource);
        saveToLocalStorage();
        updateShareToken();
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        editorRef.current?.setError(`Error loading registers: ${message}`);
      }

      // Reset file input
      event.target.value = "";
    },
    [wasmInitialized, assembleAndWrite, saveToLocalStorage, updateShareToken]
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

  const handleStartAudio = useCallback(() => {
    if (!wasmInitialized) return;

    // Reset DSP step counter
    aicaDsp.resetCounters();
    setCurrentDspStep(0);

    const audioContext = new (window.AudioContext || (window as any).webkitAudioContext)({
      sampleRate: 44100,
    });
    const scriptNode = audioContext.createScriptProcessor(1024, 0, 1);

    scriptNode.onaudioprocess = (event) => {
      const outputBuffer = event.outputBuffer.getChannelData(0);
      const sampleRate = 44100;
      const TWO_PI = Math.PI * 2;

      for (let i = 0; i < outputBuffer.length; i++) {
        // Generate sine wave sample only if frequency > 0
        const sample = frequency.current > 0 ? amplitude * Math.sin(phase.current) : 0;

        const rawKeyboardSample = Math.round(sample * 32767);

        // Apply smoothing: smoothed = old * 0.2 + new * 0.8
        const smoothedKeyboardSample = Math.round(lastKeyboardSample.current * 0.97 + rawKeyboardSample * 0.02);
        lastKeyboardSample.current = smoothedKeyboardSample;

        // Write input to DSP channels based on input source
        for (let j = 0; j < 16; j++) {
          let channelSample = 0;

          // Route based on input source (use ref to get current value)
          const source = inputSourcesRef.current[j];
          if (typeof source === "object" && source.type === "file") {
            // File source
            const file = audioFilesRef.current.find(f => f.id === source.fileId);
            if (file && file.audioBuffer) {
              const channelData = file.audioBuffer.getChannelData(0);
              const pos = aicaDsp.getSampleCounter();

              // Read sample if within bounds, otherwise silent
              if (pos < channelData.length) {
                channelSample = Math.round(channelData[pos] * 32767);
              }
            }
          } else {
            // String sources
            switch (source) {
              case "keyboard":
                channelSample = smoothedKeyboardSample;
                break;
              case "none":
              default:
                channelSample = 0;
                break;
            }
          }

          aicaDsp.writeReg(0x3000 + 0x1500 + 0 + j * 8, (channelSample >> 0) & 0xf);
          aicaDsp.writeReg(0x3000 + 0x1500 + 4 + j * 8, (channelSample >> 4) & 0xffff);
          inputPlotterRefs.current[j]?.appendSample(channelSample);
        }

        // Execute 128 DSP steps (one full sample) or until breakpoint hit
        let hitBreakpoint = false;
        const startStep = aicaDsp.getCurrentStep();

        do {
          aicaDsp.doDspStep();
          const currentStep = aicaDsp.getCurrentStep();

          // Check if we hit a breakpoint (but not on the starting step)
          if (currentStep !== startStep) {
            const bp = breakpointsRef.current.get(currentStep);
            if (bp && bp.enabled) {
              hitBreakpoint = true;
              break;
            }
          }
        } while (aicaDsp.getCurrentStep() !== 0);

        // If we hit a breakpoint, pause audio and don't continue processing this buffer
        if (hitBreakpoint) {
          // Pause audio immediately
          if (audioContextRef.current) {
            void audioContextRef.current.suspend();
          }
          queueMicrotask(() => {
            setAudioPaused(true);
            const currentStep = aicaDsp.getCurrentStep();
            setCurrentDspStep(currentStep);
            setGoToPc({ address: currentStep, fromUrl: true, highlight: false });
          });
          // Fill rest of buffer with silence
          for (let k = i; k < outputBuffer.length; k++) {
            outputBuffer[k] = 0;
          }
          return;
        }

        // Read DSP outputs
        let audioOut = 0;
        let mixSampleInt = 0;
        const states = channelStatesRef.current;
        const hasSolo = states.some(s => s === 2);

        for (let j = 0; j < 16; j++) {
          let fxSampleInt = aicaDsp.readReg(0x3000 + 0x1580 + j * 4);
          fxSampleInt = fxSampleInt & 0xffff;
          if (fxSampleInt & 0x8000) {
            fxSampleInt |= 0xffff0000;
          }

          // Apply solo/mute logic based on channel state
          let channelEnabled = true;
          if (hasSolo) {
            // If any channel is soloed, only soloed channels are enabled
            channelEnabled = states[j] === 2;
          } else {
            // Otherwise, only muted channels are disabled
            channelEnabled = states[j] !== 1;
          }

          const fxSample = fxSampleInt / 32767;
          if (channelEnabled) {
            audioOut += fxSample;
            mixSampleInt += fxSampleInt;
          }

          outputPlotterRefs.current[j]?.appendSample(fxSampleInt);
        }

        // Update mix plotter with combined output
        mixPlotterRef.current?.appendSample(mixSampleInt);

        // If a channel is tapped, output only the input of that channel
        const tapped = tappedChannelRef.current;
        if (tapped !== null) {
          const source = inputSourcesRef.current[tapped];
          if (source === "keyboard") {
            outputBuffer[i] = smoothedKeyboardSample / 32767;
          } else if (typeof source === "object" && source.type === "file") {
            const file = audioFilesRef.current.find(f => f.id === source.fileId);
            if (file && file.audioBuffer) {
              const channelData = file.audioBuffer.getChannelData(0);
              const pos = aicaDsp.getSampleCounter();

              // Read sample if within bounds, otherwise silent
              if (pos < channelData.length) {
                outputBuffer[i] = channelData[pos];
              } else {
                outputBuffer[i] = 0;
              }
            } else {
              outputBuffer[i] = 0;
            }
          } else {
            outputBuffer[i] = 0;
          }
        } else {
          outputBuffer[i] = audioOut;
        }

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
  }, [wasmInitialized]);

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
    setAudioPaused(false);
  }, []);

  // Handle audio context suspend/resume when paused
  // Note: The pause/resume button handler now does this synchronously to avoid glitches
  useEffect(() => {
    if (!audioContextRef.current || !audioPlaying) return;

    // Only handle cases where state changed outside of the pause button
    // (e.g., when stopping playback)
    if (audioPaused) {
      void audioContextRef.current.suspend();
    } else {
      void audioContextRef.current.resume();
    }
  }, [audioPlaying, audioPaused]);

  // Keyboard input handling
  useEffect(() => {
    if (!audioPlaying || audioPaused) return;

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
  }, [audioPlaying, audioPaused]);

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

  // Sync audioFiles state to ref for audio callback
  useEffect(() => {
    audioFilesRef.current = audioFiles;
  }, [audioFiles]);

  // Reset goToPc after it's been applied
  useEffect(() => {
    if (goToPc) {
      const timer = setTimeout(() => {
        setGoToPc(undefined);
      }, 100);
      return () => clearTimeout(timer);
    }
  }, [goToPc]);

  // Store last register values to track changes
  const lastRegValuesRef = useRef<Map<string, number>>(new Map());

  // Update register values directly in DOM when paused (avoids re-render)
  useEffect(() => {
    if (!wasmInitialized || !audioPlaying || !audioPaused) return;

    const sampleNum = aicaDsp.getSampleCounter();
    const stepNum = aicaDsp.getCurrentStep();
    const regs = aicaDsp.getDspRegisters();

    // Helper to update register element only if value changed
    const updateRegister = (name: string, value: number, bits: number = 32) => {
      const lastValue = lastRegValuesRef.current.get(name);
      if (lastValue === value) return; // Skip if unchanged

      lastRegValuesRef.current.set(name, value);

      const el = registerElementsRef.current.get(name);
      if (el) {
        const signExtend = (value: number, bits: number): number => {
          const shift = 32 - bits;
          return (value << shift) >> shift;
        };
        const signedValue = signExtend(value, bits);
        const hexValue = (value >>> 0).toString(16).toUpperCase().padStart(Math.ceil(bits / 4), '0');
        el.textContent = signedValue.toString().padStart(11, ' ');
        el.title = `0x${hexValue}`;
      }
    };

    // Update counters
    const sampleEl = registerElementsRef.current.get('Sample');
    if (sampleEl) {
      sampleEl.textContent = sampleNum.toString().padStart(11, ' ');
      sampleEl.title = `0x${sampleNum.toString(16).toUpperCase()}`;
    }
    const stepEl = registerElementsRef.current.get('Step');
    if (stepEl) {
      stepEl.textContent = stepNum.toString().padStart(11, ' ');
      stepEl.title = `0x${stepNum.toString(16).toUpperCase()}`;
    }

    // Update internal registers
    const regNames = ["MDEC_CT", "ACC", "SHIFTED", "X", "Y", "B", "INPUTS", "MEMVAL[0]", "MEMVAL[1]", "MEMVAL[2]", "MEMVAL[3]", "FRC_REG", "Y_REG", "ADRS_REG"];
    regNames.forEach((name, i) => {
      updateRegister(name, regs[i], i === 0 ? 10 : i >= 4 && i <= 7 ? 24 : 32);
    });

    // Update TEMP registers (128 registers, 24-bit)
    for (let i = 0; i < 128; i++) {
      updateRegister(`TEMP[${i}]`, aicaDsp.readReg(0x3000 + 0x1000 + i * 8), 24);
    }

    // Update COEF registers (128 registers, 16-bit)
    for (let i = 0; i < 128; i++) {
      updateRegister(`COEF[${i}]`, aicaDsp.readReg(0x3000 + 0x000 + i * 4), 16);
    }

    // Update MADRS registers (64 registers, 16-bit)
    for (let i = 0; i < 64; i++) {
      updateRegister(`MADRS[${i}]`, aicaDsp.readReg(0x3000 + 0x200 + i * 4), 16);
    }

    // Update MEMS registers (32 registers, 24-bit)
    for (let i = 0; i < 32; i++) {
      updateRegister(`MEMS[${i}]`, aicaDsp.readReg(0x3000 + 0x1400 + i * 8), 24);
    }

    // Update MIXS registers (16 registers, 20-bit)
    for (let i = 0; i < 16; i++) {
      updateRegister(`MIXS[${i}]`, aicaDsp.readReg(0x3000 + 0x1500 + i * 8), 20);
    }

    // Update EFREG registers (16 registers, 32-bit)
    for (let i = 0; i < 16; i++) {
      updateRegister(`EFREG[${i}]`, aicaDsp.readReg(0x3000 + 0x1580 + i * 4), 32);
    }

    // Update EXTS registers (2 registers, 16-bit)
    for (let i = 0; i < 2; i++) {
      updateRegister(`EXTS[${i}]`, aicaDsp.readReg(0x3000 + 0x15C0 + i * 4), 16);
    }
  }, [wasmInitialized, audioPlaying, audioPaused, currentDspStep, currentSample]);

  // Sync inputSources ref to Select elements on audioFiles change
  useEffect(() => {
    inputSelectRefs.current.forEach((selectEl, i) => {
      if (selectEl) {
        const source = inputSourcesRef.current[i];
        const value = typeof source === "object" ? `file:${source.fileId}` : source;
        selectEl.value = value;
      }
    });
  }, [audioFiles]);


  const handleTapToggle = useCallback((channel: number) => {
    const newTapped = tappedChannelRef.current === channel ? null : channel;
    tappedChannelRef.current = newTapped;

    // Update button classes directly without re-render
    tapButtonRefs.current.forEach((btn, idx) => {
      if (btn) {
        if (idx === newTapped) {
          btn.classList.add('tap-active');
        } else {
          btn.classList.remove('tap-active');
        }
      }
    });

    // Apply grayscale to all containers except the tapped one
    inputContainerRefs.current.forEach((container, idx) => {
      if (container) {
        if (newTapped !== null && idx !== newTapped) {
          container.style.filter = 'grayscale(100%)';
          container.style.opacity = '0.5';
        } else {
          container.style.filter = '';
          container.style.opacity = '';
        }
      }
    });

    // Apply grayscale to all outputs when input is tapped
    outputContainerRefs.current.forEach((container) => {
      if (container) {
        if (newTapped !== null) {
          container.style.filter = 'grayscale(100%)';
          container.style.opacity = '0.5';
        } else {
          container.style.filter = '';
          container.style.opacity = '';
        }
      }
    });
  }, []);

  // Helper function to update all output channel visual states
  const updateOutputVisualStates = useCallback(() => {
    const states = channelStatesRef.current;
    const hasSolo = states.some(s => s === 2);

    // Update all mute buttons
    muteButtonRefs.current.forEach((btn, idx) => {
      if (btn) {
        const icon = btn.querySelector('svg');
        const isMuted = states[idx] === 1;

        if (isMuted) {
          btn.classList.add('mute-active');
          if (icon) {
            icon.innerHTML = '<path d="M16.5 12c0-1.77-1.02-3.29-2.5-4.03v2.21l2.45 2.45c.03-.2.05-.41.05-.63zm2.5 0c0 .94-.2 1.82-.54 2.64l1.51 1.51C20.63 14.91 21 13.5 21 12c0-4.28-2.99-7.86-7-8.77v2.06c2.89.86 5 3.54 5 6.71zM4.27 3L3 4.27 7.73 9H3v6h4l5 5v-6.73l4.25 4.25c-.67.52-1.42.93-2.25 1.18v2.06c1.38-.31 2.63-.95 3.69-1.81L19.73 21 21 19.73l-9-9L4.27 3zM12 4L9.91 6.09 12 8.18V4z"></path>';
          }
        } else {
          btn.classList.remove('mute-active');
          if (icon) {
            icon.innerHTML = '<path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77z"></path>';
          }
        }
      }
    });

    // Update all solo buttons
    soloButtonRefs.current.forEach((btn, idx) => {
      if (btn) {
        const icon = btn.querySelector('svg');
        const isSoloed = states[idx] === 2;

        if (isSoloed) {
          btn.classList.add('solo-active');
          if (icon) {
            icon.innerHTML = '<path d="M12 7c-2.76 0-5 2.24-5 5s2.24 5 5 5 5-2.24 5-5-2.24-5-5-5zm0-5C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8z"></path>';
          }
        } else {
          btn.classList.remove('solo-active');
          if (icon) {
            icon.innerHTML = '<path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8z"></path>';
          }
        }
      }
    });

    // Update all output container visual states
    outputContainerRefs.current.forEach((container, idx) => {
      if (container) {
        // Determine if this channel should be grayed out
        let shouldGray = false;

        if (hasSolo) {
          // If any channel is soloed, gray out all except the soloed channel(s)
          shouldGray = states[idx] !== 2;
        } else if (states[idx] === 1) {
          // If no solo but channel is muted, gray it out
          shouldGray = true;
        }

        if (shouldGray) {
          container.style.filter = 'grayscale(100%)';
          container.style.opacity = '0.5';
        } else {
          container.style.filter = '';
          container.style.opacity = '';
        }
      }
    });
  }, []);

  const handleMuteToggle = useCallback((channel: number) => {
    const states = channelStatesRef.current;

    // Toggle between normal (0) and muted (1)
    if (states[channel] === 1) {
      states[channel] = 0; // Unmute
    } else {
      states[channel] = 1; // Mute
    }

    // Recalculate all visual states
    updateOutputVisualStates();
  }, [updateOutputVisualStates]);

  const handleEditorFocusToggle = useCallback(async () => {
    const newFocusMode = !editorFocusModeRef.current;
    editorFocusModeRef.current = newFocusMode;
    setEditorFocusMode(newFocusMode);

    if (newFocusMode) {
      // Enter fullscreen
      try {
        if (editorContainerRef.current) {
          await editorContainerRef.current.requestFullscreen();
        }
      } catch (err) {
        console.error('Failed to enter fullscreen:', err);
      }
    } else {
      // Exit fullscreen
      try {
        if (document.fullscreenElement) {
          await document.exitFullscreen();
        }
      } catch (err) {
        console.error('Failed to exit fullscreen:', err);
      }
    }

    // Toggle CSS on containers
    if (editorContainerRef.current) {
      if (newFocusMode) {
        editorContainerRef.current.style.flex = '1';
        editorContainerRef.current.style.minHeight = '0';
      } else {
        editorContainerRef.current.style.flex = '';
        editorContainerRef.current.style.minHeight = '';
      }
    }

    if (editorInnerContainerRef.current) {
      if (newFocusMode) {
        editorInnerContainerRef.current.style.flex = '1';
        editorInnerContainerRef.current.style.minHeight = '0';
        editorInnerContainerRef.current.style.height = '100%';
      } else {
        editorInnerContainerRef.current.style.flex = '';
        editorInnerContainerRef.current.style.minHeight = '';
        editorInnerContainerRef.current.style.height = '';
      }
    }

    if (dspSectionRef.current) {
      dspSectionRef.current.style.display = newFocusMode ? 'none' : 'flex';
    }

    // Trigger editor layout after DOM updates
    setTimeout(() => {
      if (activeTab === "source") {
        sourceEditorRef.current?.layout();
      } else if (activeTab === "compiled") {
        compiledEditorRef.current?.layout();
      } else {
        editorRef.current?.layout();
      }
    }, 10);
  }, [activeTab]);

  const handleSoloToggle = useCallback((channel: number) => {
    const states = channelStatesRef.current;

    if (states[channel] === 2) {
      // If this channel is already soloed, unsolo it (set to normal)
      states[channel] = 0;
    } else {
      // Clear all other solos and solo this channel
      for (let i = 0; i < states.length; i++) {
        if (states[i] === 2) {
          states[i] = 0; // Clear other solos
        }
      }
      states[channel] = 2; // Solo this channel
    }

    // Recalculate all visual states
    updateOutputVisualStates();
  }, [updateOutputVisualStates]);

  const handleInputSourceChange = useCallback((channel: number, source: InputSource) => {
    inputSourcesRef.current[channel] = source;

    // Update the Select element's value directly without re-render
    const selectElement = inputSelectRefs.current[channel];
    if (selectElement) {
      const value = typeof source === "object" ? `file:${source.fileId}` : source;
      selectElement.value = value;
    }
  }, []);

  const handleAddAudioFileForChannel = useCallback((channel: number, previousSource: InputSource) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.pcm,.wav,.mp3,.ogg,.flac,.aac,.m4a';
    input.multiple = false;
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;

      // Mark channel as loading
      setLoadingChannels((prev) => new Set(prev).add(channel));

      const audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
      const fileId = `${Date.now()}-${Math.random()}`;

      // Add file immediately with loading state
      setAudioFiles((prev) => [...prev, {
        id: fileId,
        name: file.name,
        audioBuffer: null,
        loading: true,
      }]);

      try {
        const arrayBuffer = await file.arrayBuffer();

        if (file.name.toLowerCase().endsWith('.pcm')) {
          const dataView = new DataView(arrayBuffer);
          const sampleCount = arrayBuffer.byteLength / 2;
          const audioBuffer = audioContext.createBuffer(1, sampleCount, 44100);
          const channelData = audioBuffer.getChannelData(0);

          for (let i = 0; i < sampleCount; i++) {
            const sample = dataView.getUint16(i * 2, true);
            channelData[i] = (sample - 32768) / 32768;
          }

          // Update file with decoded buffer
          setAudioFiles((prev) => prev.map((f) =>
            f.id === fileId ? { ...f, audioBuffer, loading: false } : f
          ));

          // Set input source to this file
          handleInputSourceChange(channel, { type: "file", fileId });

          // Mark channel as not loading
          setLoadingChannels((prev) => {
            const next = new Set(prev);
            next.delete(channel);
            return next;
          });
        } else {
          const audioBuffer = await audioContext.decodeAudioData(arrayBuffer);

          // Remove the loading placeholder
          setAudioFiles((prev) => prev.filter((f) => f.id !== fileId));

          // Get first channel
          const baseId = `${Date.now()}-${Math.random()}`;
          const firstChannelId = `${baseId}#0`;

          // Add all channels
          for (let channelIndex = 0; channelIndex < audioBuffer.numberOfChannels; channelIndex++) {
            const singleChannelBuffer = audioContext.createBuffer(
              1,
              audioBuffer.length,
              audioBuffer.sampleRate
            );
            const sourceChannelData = audioBuffer.getChannelData(channelIndex);
            const destChannelData = singleChannelBuffer.getChannelData(0);
            destChannelData.set(sourceChannelData);

            setAudioFiles((prev) => [...prev, {
              id: `${baseId}#${channelIndex}`,
              name: audioBuffer.numberOfChannels > 1 ? `${file.name}#${channelIndex}` : file.name,
              audioBuffer: singleChannelBuffer,
              loading: false,
            }]);
          }

          // Set input source to first channel
          handleInputSourceChange(channel, { type: "file", fileId: firstChannelId });

          // Mark channel as not loading
          setLoadingChannels((prev) => {
            const next = new Set(prev);
            next.delete(channel);
            return next;
          });
        }
      } catch (error) {
        console.error(`Failed to load audio file ${file.name}:`, error);
        const errorMessage = error instanceof Error ? error.message : String(error);

        // Update file with error state
        setAudioFiles((prev) => prev.map((f) =>
          f.id === fileId ? { ...f, loading: false, error: errorMessage } : f
        ));

        // Revert to previous source
        handleInputSourceChange(channel, previousSource);

        // Mark channel as not loading
        setLoadingChannels((prev) => {
          const next = new Set(prev);
          next.delete(channel);
          return next;
        });
      }
    };
    input.click();
  }, [handleInputSourceChange]);

  const handleFileDrop = useCallback(async (event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();

    const files = Array.from(event.dataTransfer.files);
    const audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();

    for (const file of files) {
      const fileId = `${Date.now()}-${Math.random()}`;

      // Add file immediately with loading state
      setAudioFiles((prev) => [...prev, {
        id: fileId,
        name: file.name,
        audioBuffer: null,
        loading: true,
      }]);

      try {
        const arrayBuffer = await file.arrayBuffer();

        // Handle .pcm files (raw 44100 u16 little endian)
        if (file.name.toLowerCase().endsWith('.pcm')) {
          // Create AudioBuffer manually for PCM
          const dataView = new DataView(arrayBuffer);
          const sampleCount = arrayBuffer.byteLength / 2; // u16 = 2 bytes per sample
          const audioBuffer = audioContext.createBuffer(1, sampleCount, 44100);
          const channelData = audioBuffer.getChannelData(0);

          for (let i = 0; i < sampleCount; i++) {
            const sample = dataView.getUint16(i * 2, true); // little endian
            channelData[i] = (sample - 32768) / 32768; // Convert u16 to float [-1, 1]
          }

          // Update file with decoded buffer
          setAudioFiles((prev) => prev.map((f) =>
            f.id === fileId ? { ...f, audioBuffer, loading: false } : f
          ));
        } else {
          // Handle other audio formats (.wav, .mp3, .ogg, etc.)
          // TODO: Currently assuming stereo for non-.pcm files
          const audioBuffer = await audioContext.decodeAudioData(arrayBuffer);

          // Remove the loading placeholder
          setAudioFiles((prev) => prev.filter((f) => f.id !== fileId));

          // Split multi-channel files into separate AudioFile entries
          const baseId = `${Date.now()}-${Math.random()}`;
          for (let channelIndex = 0; channelIndex < audioBuffer.numberOfChannels; channelIndex++) {
            // Create single-channel AudioBuffer
            const singleChannelBuffer = audioContext.createBuffer(
              1,
              audioBuffer.length,
              audioBuffer.sampleRate
            );
            const sourceChannelData = audioBuffer.getChannelData(channelIndex);
            const destChannelData = singleChannelBuffer.getChannelData(0);
            destChannelData.set(sourceChannelData);

            setAudioFiles((prev) => [...prev, {
              id: `${baseId}#${channelIndex}`,
              name: audioBuffer.numberOfChannels > 1 ? `${file.name}#${channelIndex}` : file.name,
              audioBuffer: singleChannelBuffer,
              loading: false,
            }]);
          }
        }
      } catch (error) {
        console.error(`Failed to load audio file ${file.name}:`, error);
        const errorMessage = error instanceof Error ? error.message : String(error);
        // Update file with error state
        setAudioFiles((prev) => prev.map((f) =>
          f.id === fileId ? { ...f, loading: false, error: errorMessage } : f
        ));
      }
    }
  }, []);

  const handleDragOver = useCallback((event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
  }, []);

  const handlePlayFile = useCallback((fileId: string) => {
    // Stop any currently playing file
    if (filePlaybackSourceRef.current) {
      filePlaybackSourceRef.current.stop();
      filePlaybackSourceRef.current.disconnect();
      filePlaybackSourceRef.current = null;
    }
    if (filePlaybackContextRef.current) {
      void filePlaybackContextRef.current.close();
      filePlaybackContextRef.current = null;
    }

    const file = audioFiles.find((f) => f.id === fileId);
    if (!file) return;

    // Create new audio context and play the file
    const context = new (window.AudioContext || (window as any).webkitAudioContext)();
    const source = context.createBufferSource();
    source.buffer = file.audioBuffer;
    source.connect(context.destination);
    source.onended = () => {
      setPlayingFileId(null);
      filePlaybackSourceRef.current = null;
      void context.close();
      filePlaybackContextRef.current = null;
    };
    source.start(0);

    filePlaybackContextRef.current = context;
    filePlaybackSourceRef.current = source;
    setPlayingFileId(fileId);
  }, [audioFiles]);

  const handleStopFile = useCallback(() => {
    if (filePlaybackSourceRef.current) {
      filePlaybackSourceRef.current.stop();
      filePlaybackSourceRef.current.disconnect();
      filePlaybackSourceRef.current = null;
    }
    if (filePlaybackContextRef.current) {
      void filePlaybackContextRef.current.close();
      filePlaybackContextRef.current = null;
    }
    setPlayingFileId(null);
  }, []);

  const handleDeleteFile = useCallback((fileId: string) => {
    setAudioFiles((prev) => prev.filter((f) => f.id !== fileId));

    // Also reset any input sources using this file to "none"
    inputSourcesRef.current = inputSourcesRef.current.map((source: InputSource) =>
      typeof source === "object" && source.fileId === fileId ? "none" : source
    );
  }, []);

  const handleDropZoneClick = useCallback(() => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.pcm,.wav,.mp3,.ogg,.flac,.aac,.m4a';
    input.multiple = true;
    input.onchange = async (e) => {
      const files = Array.from((e.target as HTMLInputElement).files || []);
      const audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();

      for (const file of files) {
        const fileId = `${Date.now()}-${Math.random()}`;

        // Add file immediately with loading state
        setAudioFiles((prev) => [...prev, {
          id: fileId,
          name: file.name,
          audioBuffer: null,
          loading: true,
        }]);

        try {
          const arrayBuffer = await file.arrayBuffer();

          if (file.name.toLowerCase().endsWith('.pcm')) {
            const dataView = new DataView(arrayBuffer);
            const sampleCount = arrayBuffer.byteLength / 2;
            const audioBuffer = audioContext.createBuffer(1, sampleCount, 44100);
            const channelData = audioBuffer.getChannelData(0);

            for (let i = 0; i < sampleCount; i++) {
              const sample = dataView.getUint16(i * 2, true);
              channelData[i] = (sample - 32768) / 32768;
            }

            // Update file with decoded buffer
            setAudioFiles((prev) => prev.map((f) =>
              f.id === fileId ? { ...f, audioBuffer, loading: false } : f
            ));
          } else {
            // TODO: Currently assuming stereo for non-.pcm files
            const audioBuffer = await audioContext.decodeAudioData(arrayBuffer);

            // Remove the loading placeholder
            setAudioFiles((prev) => prev.filter((f) => f.id !== fileId));

            // Split multi-channel files into separate AudioFile entries
            const baseId = `${Date.now()}-${Math.random()}`;
            for (let channelIndex = 0; channelIndex < audioBuffer.numberOfChannels; channelIndex++) {
              const singleChannelBuffer = audioContext.createBuffer(
                1,
                audioBuffer.length,
                audioBuffer.sampleRate
              );
              const sourceChannelData = audioBuffer.getChannelData(channelIndex);
              const destChannelData = singleChannelBuffer.getChannelData(0);
              destChannelData.set(sourceChannelData);

              setAudioFiles((prev) => [...prev, {
                id: `${baseId}#${channelIndex}`,
                name: audioBuffer.numberOfChannels > 1 ? `${file.name}#${channelIndex}` : file.name,
                audioBuffer: singleChannelBuffer,
                loading: false,
              }]);
            }
          }
        } catch (error) {
          console.error(`Failed to load audio file ${file.name}:`, error);
          const errorMessage = error instanceof Error ? error.message : String(error);
          // Update file with error state
          setAudioFiles((prev) => prev.map((f) =>
            f.id === fileId ? { ...f, loading: false, error: errorMessage } : f
          ));
        }
      }
    };
    input.click();
  }, []);

  // Helper function to check if a breakpoint is hit
  const checkBreakpoint = useCallback((step: number): boolean => {
    const bp = breakpoints.get(step);
    return bp !== undefined && bp.enabled;
  }, [breakpoints]);

  // Handler for running to next sample (run until currentStep === 0)
  const handleRunToNextSample = useCallback(() => {
    if (!wasmInitialized || !audioPaused) return;

    // Run until we reach the next sample boundary (step 0) or hit a breakpoint
    const startStep = aicaDsp.getCurrentStep();

    // Always step at least once, then continue until we reach step 0
    do {
      aicaDsp.doDspStep();
      const currentStep = aicaDsp.getCurrentStep();

      // Check if we hit a breakpoint (but not on the starting step)
      if (currentStep !== startStep && checkBreakpoint(currentStep)) {
        break;
      }

      // Stop when we reach step 0 (sample boundary)
      if (currentStep === 0) {
        break;
      }
    } while (true);

    // Update current step and sample for the disassembly view
    const newStep = aicaDsp.getCurrentStep();
    const newSample = aicaDsp.getSampleCounter();
    setCurrentDspStep(newStep);
    setCurrentSample(newSample);

    // Scroll to the current PC without highlighting
    setGoToPc({ address: newStep, fromUrl: true, highlight: false });
  }, [wasmInitialized, audioPaused, checkBreakpoint]);

  // Handler for running 128 samples
  const handleRun128Samples = useCallback(() => {
    if (!wasmInitialized || !audioPaused) return;

    let startStep = aicaDsp.getCurrentStep();
    let hitBreakpoint = false;

    // Run 128 samples (or until breakpoint)
    for (let sampleIdx = 0; sampleIdx < 128; sampleIdx++) {
      // Process inputs for all 16 channels
      for (let j = 0; j < 16; j++) {
        const source = inputSourcesRef.current[j];
        let channelSample = 0;

        if (typeof source === "object" && source.type === "file") {
          // File source
          const file = audioFilesRef.current.find(f => f.id === source.fileId);
          if (file && file.audioBuffer) {
            const channelData = file.audioBuffer.getChannelData(0);
            const pos = aicaDsp.getSampleCounter();

            // Read sample if within bounds, otherwise silent
            if (pos < channelData.length) {
              channelSample = Math.round(channelData[pos] * 32767);
            }
          }
        }
        // Note: keyboard input is not included when paused

        aicaDsp.writeReg(0x3000 + 0x1500 + 0 + j * 8, (channelSample >> 0) & 0xf);
        aicaDsp.writeReg(0x3000 + 0x1500 + 4 + j * 8, (channelSample >> 4) & 0xffff);
        inputPlotterRefs.current[j]?.appendSample(channelSample);
      }

      // Run one full sample (128 DSP steps) or until breakpoint hit
      let completedSample = false;

      do {
        aicaDsp.doDspStep();
        const currentStep = aicaDsp.getCurrentStep();

        // Check if we hit a breakpoint (but not on the starting step)
        if (currentStep !== startStep && checkBreakpoint(currentStep)) {
          hitBreakpoint = true;
          break;
        }

        startStep = -1;

        // Check if we completed a full sample
        if (currentStep === 0) {
          completedSample = true;
          break;
        }
      } while (true);

      // Only read outputs and update plotters if we completed the full sample
      if (completedSample) {

        // Read DSP outputs and update plotters
        let mixSampleInt = 0;
        const states = channelStatesRef.current;
        const hasSolo = states.some(s => s === 2);

        for (let j = 0; j < 16; j++) {
          let fxSampleInt = aicaDsp.readReg(0x3000 + 0x1580 + j * 4);
          fxSampleInt = fxSampleInt & 0xffff;
          if (fxSampleInt & 0x8000) {
            fxSampleInt |= 0xffff0000;
          }

          // Apply solo/mute logic based on channel state
          let channelEnabled = true;
          if (hasSolo) {
            channelEnabled = states[j] === 2;
          } else {
            channelEnabled = states[j] !== 1;
          }

          if (channelEnabled) {
            mixSampleInt += fxSampleInt;
          }

          outputPlotterRefs.current[j]?.appendSample(fxSampleInt);
        }

        // Update mix plotter with combined output
        mixPlotterRef.current?.appendSample(mixSampleInt);
      }

      // Stop if we hit a breakpoint
      if (hitBreakpoint) {
        break;
      }
    }

    // Update current step and sample for the disassembly view
    const newStep = aicaDsp.getCurrentStep();
    const newSample = aicaDsp.getSampleCounter();
    setCurrentDspStep(newStep);
    setCurrentSample(newSample);

    // Scroll to the current PC without highlighting
    setGoToPc({ address: newStep, fromUrl: true, highlight: false });
  }, [wasmInitialized, audioPaused, checkBreakpoint]);

  // DisassemblyView configuration for DSP
  const disassemblyConfig: DisassemblyViewConfig = useMemo(() => ({
    instructionSize: 1,
    maxAddress: 0x7f,
    formatAddressInput: (value: number) => value.toString(),
    formatAddressDisplay: (value: number) => value.toString().padStart(3, '0'),
    parseAddressInput: (input: string) => {
      const trimmed = input.trim();
      if (!trimmed) return undefined;
      const parsed = Number.parseInt(trimmed, 10);
      return Number.isNaN(parsed) ? undefined : parsed;
    },
    gridColumns: '24px 80px 1fr',
    stepLabel: 'Step',
    stepIcon: ArrowForwardIcon,
    showStepInOut: false,
    urlParamName: 'step',
    showBytes: false,
    showMuteSolo: false,
    runPauseIcon: {
      paused: PlayArrowIcon,
      running: PauseIcon,
    },
    runPauseLabel: {
      paused: 'Run',
      running: 'Pause',
    },
    prefixActions: [
      {
        key: 'run128',
        icon: FastForwardIcon,
        label: 'Run 128 samples',
        disabled: !audioPlaying || !audioPaused,
        onClick: handleRun128Samples,
      },
      {
        key: 'runToNextSample',
        icon: SkipNextIcon,
        label: 'Run to next sample',
        disabled: !audioPlaying || !audioPaused,
        onClick: handleRunToNextSample,
      },
    ],
  }), [audioPlaying, audioPaused, handleRunToNextSample, handleRun128Samples]);

  // DisassemblyView callbacks for DSP playground
  const disassemblyCallbacks: DisassemblyViewCallbacks = useMemo(() => ({
    onFetchDisassembly: async (address: number, count: number): Promise<DisassemblyLine[]> => {
      if (!wasmInitialized) {
        return [];
      }

      const lines: DisassemblyLine[] = [];
      for (let i = 0; i < count && address + i <= 0x7f; i++) {
        const step = address + i;
        const dwords = [
          aicaDsp.readReg(0x3000 + 0x400 + step * 4 * 4 + 0),
          aicaDsp.readReg(0x3000 + 0x400 + step * 4 * 4 + 4),
          aicaDsp.readReg(0x3000 + 0x400 + step * 4 * 4 + 8),
          aicaDsp.readReg(0x3000 + 0x400 + step * 4 * 4 + 12),
        ];

        const desc = decodeInst(dwords);
        const disasm = disassembleDesc(desc);

        lines.push({
          address: step,
          bytes: dwords.map(d => d.toString(16).padStart(8, '0')).join(' '),
          disassembly: disasm || '(nop)',
        });
      }

      return lines;
    },
    onStep: async () => {
      if (!wasmInitialized || !audioPaused) return;

      // Process inputs for all 16 channels
      for (let j = 0; j < 16; j++) {
        const source = inputSourcesRef.current[j];
        let channelSample = 0;

        if (typeof source === "object" && source.type === "file") {
          const file = audioFilesRef.current.find(f => f.id === source.fileId);
          if (file && file.audioBuffer) {
            const channelData = file.audioBuffer.getChannelData(0);
            const pos = aicaDsp.getSampleCounter();

            if (pos < channelData.length) {
              channelSample = Math.round(channelData[pos] * 32767);
            }
          }
        }

        aicaDsp.writeReg(0x3000 + 0x1500 + 0 + j * 8, (channelSample >> 0) & 0xf);
        aicaDsp.writeReg(0x3000 + 0x1500 + 4 + j * 8, (channelSample >> 4) & 0xffff);
        inputPlotterRefs.current[j]?.appendSample(channelSample);
      }

      // Execute one DSP step
      aicaDsp.doDspStep();

      // Update current step for the disassembly view
      const newStep = aicaDsp.getCurrentStep();
      setCurrentDspStep(newStep);

      // Scroll to the current PC without highlighting
      setGoToPc({ address: newStep, fromUrl: true, highlight: false });

      // Read DSP outputs and update plotters
      let mixSampleInt = 0;
      const states = channelStatesRef.current;
      const hasSolo = states.some(s => s === 2);

      for (let j = 0; j < 16; j++) {
        let fxSampleInt = aicaDsp.readReg(0x3000 + 0x1580 + j * 4);
        fxSampleInt = fxSampleInt & 0xffff;
        if (fxSampleInt & 0x8000) {
          fxSampleInt |= 0xffff0000;
        }

        let channelEnabled = true;
        if (hasSolo) {
          channelEnabled = states[j] === 2;
        } else {
          channelEnabled = states[j] !== 1;
        }

        if (channelEnabled) {
          mixSampleInt += fxSampleInt;
        }

        outputPlotterRefs.current[j]?.appendSample(fxSampleInt);
      }

      mixPlotterRef.current?.appendSample(mixSampleInt);
    },
    onBreakpointAdd: async (address: number) => {

      let audioWasPaused = false;
      // Pause audio first if playing (to avoid hitting the breakpoint immediately)
      if (audioPlaying && !audioPaused && audioContextRef.current) {
        audioWasPaused = true;
        await void audioContextRef.current.suspend();
        setAudioPaused(true);

        while (aicaDsp.getCurrentStep() != address) {
          aicaDsp.doDspStep();
        }
      }

      const id = breakpointIdCounterRef.current++;
      setBreakpoints(prev => new Map(prev).set(address, { id, enabled: true }));

      // Update display after adding breakpoint
      if (wasmInitialized && audioWasPaused) {
        const newStep = aicaDsp.getCurrentStep();
        setCurrentDspStep(newStep);
      }
    },
    onBreakpointRemove: async (id: number) => {
      setBreakpoints(prev => {
        const next = new Map(prev);
        for (const [addr, bp] of next.entries()) {
          if (bp.id === id) {
            next.delete(addr);
            break;
          }
        }
        return next;
      });
    },
    onBreakpointToggle: async (id: number, enabled: boolean) => {
      setBreakpoints(prev => {
        const next = new Map(prev);
        for (const [addr, bp] of next.entries()) {
          if (bp.id === id) {
            next.set(addr, { ...bp, enabled });
            break;
          }
        }
        return next;
      });
    },
    onMuteToggle: () => {
      // No-op in playground mode
    },
    onSoloToggle: () => {
      // No-op in playground mode
    },
    onRunPauseToggle: () => {
      if (!audioPlaying) return; // Disabled when audio not playing

      if (audioPaused) {
        // Resume - update state first, then resume audio in microtask
        setAudioPaused(false);
        queueMicrotask(() => {
          if (audioContextRef.current) {
            void audioContextRef.current.resume();
          }
        });
      } else {
        // Pause - first suspend audio immediately, then update state in microtask
        if (audioContextRef.current) {
          void audioContextRef.current.suspend();
        }
        queueMicrotask(() => {
          setAudioPaused(true);
          // Scroll to current step when pausing
          if (wasmInitialized) {
            const currentStep = aicaDsp.getCurrentStep();
            setCurrentDspStep(currentStep);
            setGoToPc({ address: currentStep, fromUrl: true, highlight: false });
          }
        });
      }
    },
  }), [wasmInitialized, audioPlaying, audioPaused, breakpoints]);

  if (!wasmInitialized) {
    return (
      <Panel>
        <Box sx={{ p: 3, display: "flex", justifyContent: "center", alignItems: "center" }}>
          <Typography variant="body1" color="text.secondary">
            Loading WASM module...
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
              <strong>Keyboard input active:</strong> z,x,c,v,b,n,m (lower octave) | a,s,d,f,g,h,j,k,l (higher octave)
            </Typography>
          </Alert>
        )}

        {/* Source Section */}
        <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {/* Source Header */}
          <Box sx={{ display: "flex", alignItems: "center", gap: 1, mt: 2 }}>
            <Typography variant="h6">Source</Typography>
            <Tooltip title="Collapse/Expand section">
              <IconButton
                size="small"
                onClick={(e) => {
                  sourceExpandedRef.current = !sourceExpandedRef.current;
                  if (sourceContainerRef.current) {
                    sourceContainerRef.current.style.display = sourceExpandedRef.current ? 'block' : 'none';
                  }
                  const btn = e.currentTarget;
                  btn.style.transform = sourceExpandedRef.current ? 'rotate(180deg)' : 'rotate(0deg)';
                }}
                sx={{
                  transform: 'rotate(180deg)',
                  transition: 'transform 0.2s',
                }}
              >
                <KeyboardArrowDownIcon />
              </IconButton>
            </Tooltip>
          </Box>

          <Box ref={sourceContainerRef}>
            {/* Tabbed Editor */}
            <Box
              ref={editorContainerRef}
              sx={{
                position: 'relative',
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
              }}
            >
                <Tabs
                  value={activeTab}
                  onChange={(_, newValue) => {
                    setActiveTab(newValue);
                    // Don't reset editorsReady - let the useEffect handle re-compilation
                  }}
                  sx={{ width: '100%', borderBottom: 1, borderColor: 'divider' }}
                >
                  <Tab label="Compiler" value="source" />
                  <Tab label="Compiled" value="compiled" />
                  <Tab label="Assembler" value="assembly" />
                </Tabs>

                <Box ref={editorInnerContainerRef} sx={{ width: '100%', height: 250 }}>
                  {activeTab === "source" ? (
                    <DspSourceEditor
                      ref={sourceEditorRef}
                      value={dspSourceRef.current}
                      onChange={handleDspSourceChange}
                      height="100%"
                      onEditorReady={handleEditorReady}
                    />
                  ) : activeTab === "compiled" ? (
                    <DspAssemblyEditor
                      key={activeTab}
                      ref={compiledEditorRef}
                      value={compiledAssemblyRef.current}
                      onChange={() => {}} // Read-only
                      height="100%"
                      readOnly={true}
                      onEditorReady={handleEditorReady}
                    />
                  ) : (
                    <DspAssemblyEditor
                      key={activeTab}
                      ref={editorRef}
                      value={assemblySourceRef.current}
                      onChange={handleAssemblySourceChange}
                      height="100%"
                      onEditorReady={handleEditorReady}
                    />
                  )}
                </Box>
                <Tooltip title="Enter/Exit focus mode">
                  <IconButton
                    ref={editorFocusButtonRef}
                    size="small"
                    onClick={handleEditorFocusToggle}
                    sx={{
                      mt: 1,
                      backgroundColor: 'background.paper',
                      opacity: 0.7,
                      flexShrink: 0,
                      '&:hover': {
                        opacity: 1,
                        backgroundColor: 'background.paper',
                      },
                    }}
                  >
                    {editorFocusMode ? <FullscreenExitIcon /> : <FullscreenIcon />}
                  </IconButton>
                </Tooltip>
              </Box>
          </Box>
        </Box>

        {/* Debugger Section */}
        <Box ref={dspSectionRef} sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {/* Debugger Header */}
          <Box sx={{ display: "flex", alignItems: "center", gap: 1, mt: 2 }}>
            <Typography variant="h6">Debugger</Typography>
            <Tooltip title="Collapse/Expand section">
              <IconButton
                size="small"
                onClick={(e) => {
                  debuggerExpandedRef.current = !debuggerExpandedRef.current;
                  if (debuggerContainerRef.current) {
                    debuggerContainerRef.current.style.display = debuggerExpandedRef.current ? 'block' : 'none';
                  }
                  const btn = e.currentTarget;
                  btn.style.transform = debuggerExpandedRef.current ? 'rotate(180deg)' : 'rotate(0deg)';
                }}
                sx={{
                  transform: 'rotate(0deg)',
                  transition: 'transform 0.2s',
                }}
              >
                <KeyboardArrowDownIcon />
              </IconButton>
            </Tooltip>
          </Box>

          <Box ref={debuggerContainerRef} sx={{ display: 'none' }}>
            {/* Disassembly */}
            <Typography variant="subtitle2">Disassembly</Typography>
              <Box sx={{ height: 600 }}>
            <DisassemblyView
              key={binaryVersion}
              config={disassemblyConfig}
              callbacks={disassemblyCallbacks}
              defaultAddress={0}
              currentPc={audioPlaying && audioPaused ? currentDspStep : undefined}
              breakpointsByAddress={breakpoints}
              initialized={wasmInitialized && audioPlaying}
              executionState={audioPlaying && audioPaused ? "paused" : "running"}
              categoryState={undefined}
              initialUrlAddress={goToPc}
            />
          </Box>

          {/* Registers */}
          <Typography variant="subtitle2" sx={{ mt: 2 }}>Registers</Typography>
          <Box sx={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(250px, 1fr))",
            gap: 1,
            fontFamily: "monospace",
            fontSize: "0.875rem"
          }}>
            {wasmInitialized && (() => {
              // Use default values for initial render
              const regNames = ["MDEC_CT", "ACC", "SHIFTED", "X", "Y", "B", "INPUTS", "MEMVAL[0]", "MEMVAL[1]", "MEMVAL[2]", "MEMVAL[3]", "FRC_REG", "Y_REG", "ADRS_REG"];
              const counters = [
                { name: "Sample", value: 0 },
                { name: "Step", value: 0 },
              ];
              const internalRegs = Array(regNames.length).fill(0);
              const tempRegs = Array.from({ length: 128 }, (_, i) => ({ name: `TEMP[${i}]`, value: 0 }));
              const coefRegs = Array.from({ length: 128 }, (_, i) => ({ name: `COEF[${i}]`, value: 0 }));
              const madrsRegs = Array.from({ length: 64 }, (_, i) => ({ name: `MADRS[${i}]`, value: 0 }));
              const memsRegs = Array.from({ length: 32 }, (_, i) => ({ name: `MEMS[${i}]`, value: 0 }));
              const mixsRegs = Array.from({ length: 16 }, (_, i) => ({ name: `MIXS[${i}]`, value: 0 }));
              const efregRegs = Array.from({ length: 16 }, (_, i) => ({ name: `EFREG[${i}]`, value: 0 }));
              const extsRegs = Array.from({ length: 2 }, (_, i) => ({ name: `EXTS[${i}]`, value: 0 }));

              // Sign extend values based on bit width
              const signExtend = (value: number, bits: number): number => {
                const shift = 32 - bits;
                return (value << shift) >> shift;
              };

              const renderRegister = (name: string, value: number, bits: number = 32) => {
                const signedValue = signExtend(value, bits);
                const hexValue = (value >>> 0).toString(16).toUpperCase().padStart(Math.ceil(bits / 4), '0');

                return (
                  <Box key={name} sx={{ display: "flex", gap: 1 }}>
                    <Typography component="span" sx={{ color: "text.secondary", minWidth: "100px" }}>
                      {name}:
                    </Typography>
                    <Typography
                      component="span"
                      ref={(el) => {
                        if (el) registerElementsRef.current.set(name, el);
                      }}
                      sx={{ fontWeight: "bold", cursor: "default" }}
                      title={`0x${hexValue}`}
                    >
                      {signedValue.toString().padStart(11, ' ')}
                    </Typography>
                  </Box>
                );
              };

              return (
                <>
                  {/* Counters */}
                  {counters.map(({ name, value }) => (
                    <Box key={name} sx={{ display: "flex", gap: 1 }}>
                      <Typography component="span" sx={{ color: "primary.main", minWidth: "100px", fontWeight: "bold" }}>
                        {name}:
                      </Typography>
                      <Typography
                        component="span"
                        ref={(el) => {
                          if (el) registerElementsRef.current.set(name, el);
                        }}
                        sx={{ fontWeight: "bold", cursor: "default" }}
                        title={`0x${value.toString(16).toUpperCase()}`}
                      >
                        {value.toString().padStart(11, ' ')}
                      </Typography>
                    </Box>
                  ))}

                  {/* Internal registers header */}
                  <Box key="internal-header" sx={{ gridColumn: "1 / -1", mt: 1 }}>
                    <Typography variant="caption" sx={{ color: "text.disabled", fontWeight: "bold" }}>
                      Internal Registers
                    </Typography>
                  </Box>
                  {regNames.map((name, i) => {
                    // Internal registers are 24-bit signed values, except MDEC_CT and ADRS_REG
                    const bits = (name === "MDEC_CT" || name === "ADRS_REG") ? 32 : 24;
                    return renderRegister(name, internalRegs[i], bits);
                  })}

                  {/* TEMP registers - 24-bit signed */}
                  <Box key="temp-header" sx={{ gridColumn: "1 / -1", mt: 1 }}>
                    <Typography variant="caption" sx={{ color: "text.disabled", fontWeight: "bold" }}>
                      TEMP (Temporary Storage)
                    </Typography>
                  </Box>
                  {tempRegs.map(({ name, value }) => renderRegister(name, value, 24))}

                  {/* COEF registers - 16-bit signed */}
                  <Box key="coef-header" sx={{ gridColumn: "1 / -1", mt: 1 }}>
                    <Typography variant="caption" sx={{ color: "text.disabled", fontWeight: "bold" }}>
                      COEF (Coefficients)
                    </Typography>
                  </Box>
                  {coefRegs.map(({ name, value }) => renderRegister(name, value, 16))}

                  {/* MADRS registers - 16-bit unsigned */}
                  <Box key="madrs-header" sx={{ gridColumn: "1 / -1", mt: 1 }}>
                    <Typography variant="caption" sx={{ color: "text.disabled", fontWeight: "bold" }}>
                      MADRS (Memory Addresses)
                    </Typography>
                  </Box>
                  {madrsRegs.map(({ name, value }) => renderRegister(name, value, 16))}

                  {/* MEMS registers - 24-bit signed */}
                  <Box key="mems-header" sx={{ gridColumn: "1 / -1", mt: 1 }}>
                    <Typography variant="caption" sx={{ color: "text.disabled", fontWeight: "bold" }}>
                      MEMS (Memory Samples)
                    </Typography>
                  </Box>
                  {memsRegs.map(({ name, value }) => renderRegister(name, value, 24))}

                  {/* MIXS registers - 20-bit signed */}
                  <Box key="mixs-header" sx={{ gridColumn: "1 / -1", mt: 1 }}>
                    <Typography variant="caption" sx={{ color: "text.disabled", fontWeight: "bold" }}>
                      MIXS (Mixer Samples)
                    </Typography>
                  </Box>
                  {mixsRegs.map(({ name, value }) => renderRegister(name, value, 20))}

                  {/* EFREG registers - 32-bit unsigned */}
                  <Box key="efreg-header" sx={{ gridColumn: "1 / -1", mt: 1 }}>
                    <Typography variant="caption" sx={{ color: "text.disabled", fontWeight: "bold" }}>
                      EFREG (Effect Outputs)
                    </Typography>
                  </Box>
                  {efregRegs.map(({ name, value }) => renderRegister(name, value, 32))}

                  {/* EXTS registers - 16-bit signed */}
                  <Box key="exts-header" sx={{ gridColumn: "1 / -1", mt: 1 }}>
                    <Typography variant="caption" sx={{ color: "text.disabled", fontWeight: "bold" }}>
                      EXTS (External Inputs)
                    </Typography>
                  </Box>
                  {extsRegs.map(({ name, value }) => renderRegister(name, value, 16))}
                </>
              );
            })()}
          </Box>
          </Box>
        </Box>

        {/* Waveforms Section */}
        <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {/* Waveforms Header */}
          <Box sx={{ display: "flex", alignItems: "center", gap: 1, mt: 2 }}>
            <Typography variant="h6">Waveforms</Typography>
            <Tooltip title="Collapse/Expand section">
              <IconButton
                size="small"
                onClick={(e) => {
                  waveformsExpandedRef.current = !waveformsExpandedRef.current;
                  if (waveformsContainerRef.current) {
                    waveformsContainerRef.current.style.display = waveformsExpandedRef.current ? 'block' : 'none';
                  }
                  const btn = e.currentTarget;
                  btn.style.transform = waveformsExpandedRef.current ? 'rotate(180deg)' : 'rotate(0deg)';
                }}
                sx={{
                  transform: 'rotate(180deg)',
                  transition: 'transform 0.2s',
                }}
              >
                <KeyboardArrowDownIcon />
              </IconButton>
            </Tooltip>
          </Box>

          <Box ref={waveformsContainerRef}>
            <Typography variant="subtitle2" sx={{ mt: 2 }}>Inputs</Typography>
          <Box
            sx={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
              gap: 1,
            }}
          >
            {Array.from({ length: 16 }, (_, i) => (
              <Box
                key={i}
                ref={(el) => {
                  inputContainerRefs.current[i] = el as HTMLElement | null;
                }}
                onDragOver={(e) => {
                  e.preventDefault();
                  e.dataTransfer.dropEffect = "copy";
                }}
                onDrop={(e) => {
                  e.preventDefault();
                  const fileId = e.dataTransfer.getData("application/x-audio-file-id");
                  if (fileId) {
                    handleInputSourceChange(i, { type: "file", fileId });
                  }
                }}
              >
                <Stack direction="row" spacing={0.5} alignItems="center" justifyContent="space-between" sx={{ mb: 0.5 }}>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.25 }}>
                    <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
                      In {i.toString().padStart(2, '0')}
                    </Typography>
                    <Tooltip title="Listen to this input only">
                      <IconButton
                        ref={(el) => {
                          tapButtonRefs.current[i] = el;
                        }}
                        size="small"
                        onClick={() => handleTapToggle(i)}
                        sx={{
                          width: 16,
                          height: 16,
                          minWidth: 16,
                          minHeight: 16,
                          p: 0.25,
                          mx: '0.25em',
                          '&.tap-active': {
                            color: 'primary.main',
                          },
                        }}
                      >
                        <HearingIcon sx={{ fontSize: 12 }} />
                      </IconButton>
                    </Tooltip>
                  </Box>
                  {loadingChannels.has(i) && (
                    <CircularProgress size={12} sx={{ mr: 0.5 }} />
                  )}
                  <Tooltip
                    title={(() => {
                      const source = inputSourcesRef.current[i];
                      return typeof source === "object" && source.type === "file"
                        ? `File: ${audioFiles.find((f) => f.id === source.fileId)?.name || ""}`
                        : "";
                    })()}
                    disableInteractive
                  >
                    <Box component="span">
                      <Select
                        size="small"
                        disabled={loadingChannels.has(i)}
                        inputRef={(el) => { inputSelectRefs.current[i] = el; }}
                        defaultValue={
                          typeof inputSourcesRef.current[i] === "object"
                            ? `file:${inputSourcesRef.current[i].fileId}`
                            : inputSourcesRef.current[i]
                        }
                        onChange={(e) => {
                          const val = e.target.value as string;
                          if (val === "add-file") {
                            handleAddAudioFileForChannel(i, inputSourcesRef.current[i]);
                            // Don't change the selection
                            return;
                          } else if (val.startsWith("file:")) {
                            const fileId = val.substring(5);
                            handleInputSourceChange(i, { type: "file", fileId });
                          } else {
                            handleInputSourceChange(i, val as "none" | "keyboard");
                          }
                        }}
                        sx={{
                          fontSize: "0.75rem",
                          height: "20px",
                          maxWidth: "120px",
                          "& .MuiSelect-select": {
                            py: 0,
                            px: 0.5,
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                          },
                        }}
                      >
                        <MenuItem value="none">None</MenuItem>
                        <MenuItem value="keyboard">Keyboard</MenuItem>
                        {audioFiles.map((file) => (
                          <MenuItem key={file.id} value={`file:${file.id}`}>
                            File: {file.name}
                          </MenuItem>
                        ))}
                        <MenuItem value="add-file" disabled={audioPlaying}>Add audio file...</MenuItem>
                      </Select>
                    </Box>
                  </Tooltip>
                </Stack>
                <WaveformPlotter
                  ref={(el) => {
                    inputPlotterRefs.current[i] = el;
                  }}
                  height={100}
                  maxSamples={180}
                  fillWidth={true}
                />
              </Box>
            ))}
          </Box>

          <Typography variant="subtitle2" sx={{ mt: 2 }}>
            Outputs
          </Typography>
          <Box
            sx={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
              gap: 1,
            }}
          >
            {Array.from({ length: 16 }, (_, i) => (
              <Box
                key={i}
                ref={(el) => {
                  outputContainerRefs.current[i] = el as HTMLElement | null;
                }}
              >
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.25, mb: 0.5, justifyContent: 'space-between' }}>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.25 }}>
                    <Typography variant="caption" color="text.secondary" sx={{ fontFamily: 'monospace' }}>
                      Out {i.toString().padStart(2, '0')}
                    </Typography>
                    <Box sx={{ display: 'flex', alignItems: 'center', mx: '0.25em' }}>
                      <Tooltip title="Mute/Unmute this output channel">
                        <IconButton
                          ref={(el) => {
                            muteButtonRefs.current[i] = el;
                          }}
                          size="small"
                          onClick={() => handleMuteToggle(i)}
                          sx={{
                            width: 16,
                            height: 16,
                            minWidth: 16,
                            minHeight: 16,
                            p: 0.25,
                            '&.mute-active': {
                              color: 'error.main',
                            },
                          }}
                        >
                          <VolumeUpIcon sx={{ fontSize: 12 }} />
                        </IconButton>
                      </Tooltip>
                      <Tooltip title="Solo this output channel">
                        <IconButton
                          ref={(el) => {
                            soloButtonRefs.current[i] = el;
                          }}
                          size="small"
                          onClick={() => handleSoloToggle(i)}
                          sx={{
                            width: 16,
                            height: 16,
                            minWidth: 16,
                            minHeight: 16,
                            p: 0.25,
                            '&.solo-active': {
                              color: 'warning.main',
                            },
                          }}
                        >
                          <RadioButtonUncheckedIcon sx={{ fontSize: 12 }} />
                        </IconButton>
                      </Tooltip>
                    </Box>
                  </Box>
                  <Tooltip title="10x scale">
                    <IconButton
                      ref={(el) => {
                        zoomButtonRefs.current[i] = el;
                      }}
                      size="small"
                      onClick={() => {
                        const wasScaled = outputScale10xRef.current.has(i);
                        const nowScaled = !wasScaled;

                        if (nowScaled) {
                          outputScale10xRef.current.add(i);
                        } else {
                          outputScale10xRef.current.delete(i);
                        }

                        // Update button class
                        const btn = zoomButtonRefs.current[i];
                        if (btn) {
                          if (nowScaled) {
                            btn.classList.add('zoom-active');
                          } else {
                            btn.classList.remove('zoom-active');
                          }
                        }

                        // Update the plotter's max amplitude
                        const plotter = outputPlotterRefs.current[i];
                        if (plotter) {
                          plotter.setMaxAmplitude(nowScaled ? 3276.7 : 32767);
                        }
                      }}
                      sx={{
                        width: 16,
                        height: 16,
                        minWidth: 16,
                        minHeight: 16,
                        p: 0.25,
                        '&.zoom-active': {
                          color: 'primary.main',
                        },
                      }}
                    >
                      <ZoomInIcon sx={{ fontSize: 12 }} />
                    </IconButton>
                  </Tooltip>
                </Box>
                <WaveformPlotter
                  ref={(el) => {
                    outputPlotterRefs.current[i] = el;
                  }}
                  height={100}
                  maxSamples={180}
                  maxAmplitude={outputScale10xRef.current.has(i) ? 3276.7 : 32767}
                  fillWidth={true}
                />
              </Box>
            ))}
          </Box>

          <Typography variant="subtitle2" sx={{ mt: 2 }}>
            Output Mix
          </Typography>
          <Box>
            <WaveformPlotter
              ref={(el) => {
                mixPlotterRef.current = el;
              }}
              height={100}
              maxSamples={800}
              fillWidth={true}
            />
          </Box>
          </Box>
        </Box>

        {/* Audio File Sources Section */}
        <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
          {/* Audio File Sources Header */}
          <Box sx={{ display: "flex", alignItems: "center", gap: 1, mt: 2 }}>
            <Typography variant="h6">Audio File Sources</Typography>
            <Tooltip title="Collapse/Expand section">
              <IconButton
                size="small"
                onClick={(e) => {
                  audioFilesExpandedRef.current = !audioFilesExpandedRef.current;
                  if (audioFilesContainerRef.current) {
                    audioFilesContainerRef.current.style.display = audioFilesExpandedRef.current ? 'block' : 'none';
                  }
                  const btn = e.currentTarget;
                  btn.style.transform = audioFilesExpandedRef.current ? 'rotate(180deg)' : 'rotate(0deg)';
                }}
                sx={{
                  transform: 'rotate(180deg)',
                  transition: 'transform 0.2s',
                }}
              >
                <KeyboardArrowDownIcon />
              </IconButton>
            </Tooltip>
          </Box>

          <Box ref={audioFilesContainerRef}>
          {/* File List */}
          {audioFiles.length > 0 && (
            <Box
              sx={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
                gap: 1,
                mb: 1,
                opacity: audioPlaying ? 0.5 : 1,
                pointerEvents: audioPlaying ? "none" : "auto",
                filter: audioPlaying ? "grayscale(100%)" : "none",
              }}
            >
              {audioFiles.map((file) => (
                <Paper
                  key={file.id}
                  variant="outlined"
                  draggable={!file.loading && !file.error}
                  onDragStart={(e) => {
                    e.dataTransfer.setData("application/x-audio-file-id", file.id);
                    e.dataTransfer.effectAllowed = "copy";
                  }}
                  sx={{
                    p: 1,
                    display: "flex",
                    alignItems: "center",
                    gap: 1,
                    overflow: "hidden",
                    position: "relative",
                    cursor: file.loading || file.error ? "default" : "grab",
                    "&:active": {
                      cursor: file.loading || file.error ? "default" : "grabbing",
                    },
                  }}
                >
                  {file.loading && (
                    <Box
                      sx={{
                        position: "absolute",
                        top: 0,
                        left: 0,
                        right: 0,
                        bottom: 0,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        backgroundColor: "rgba(0, 0, 0, 0.5)",
                        zIndex: 1,
                      }}
                    >
                      <CircularProgress size={24} />
                    </Box>
                  )}
                  <Box sx={{ flex: 1, minWidth: 0 }}>
                    <Box
                      sx={{
                        overflow: "hidden",
                        position: "relative",
                        "&:hover .filename-wrapper": {
                          animation: "scroll 10s linear infinite",
                        },
                        "@keyframes scroll": {
                          "0%": { transform: "translateX(0%)" },
                          "100%": { transform: "translateX(-100%)" },
                        },
                      }}
                    >
                      <Box
                        className="filename-wrapper"
                        sx={{
                          display: "inline-flex",
                          whiteSpace: "nowrap",
                        }}
                      >
                        <Typography
                          variant="body2"
                          component="span"
                          sx={{ display: "inline-block", pr: 4 }}
                        >
                          {file.name}
                        </Typography>
                        <Typography
                          variant="body2"
                          component="span"
                          sx={{ display: "inline-block", pr: 4 }}
                        >
                          {file.name}
                        </Typography>
                      </Box>
                    </Box>
                    {file.error ? (
                      <Typography variant="caption" color="error">
                        Error: {file.error}
                      </Typography>
                    ) : file.audioBuffer ? (
                      <Typography variant="caption" color="text.secondary">
                        {file.audioBuffer.duration.toFixed(2)}s, {file.audioBuffer.sampleRate}Hz, {((file.audioBuffer.length * 2) / (1024 * 1024)).toFixed(2)}MB
                      </Typography>
                    ) : (
                      <Typography variant="caption" color="text.secondary">
                        Loading...
                      </Typography>
                    )}
                  </Box>
                  <Box sx={{ display: "flex", flexDirection: "column", gap: 0.5 }}>
                    {playingFileId === file.id ? (
                      <IconButton size="small" onClick={handleStopFile} disabled={file.loading || !!file.error}>
                        <StopIcon fontSize="small" />
                      </IconButton>
                    ) : (
                      <IconButton size="small" onClick={() => handlePlayFile(file.id)} disabled={file.loading || !!file.error}>
                        <PlayArrowIcon fontSize="small" />
                      </IconButton>
                    )}
                    <IconButton size="small" onClick={() => handleDeleteFile(file.id)} disabled={file.loading}>
                      <DeleteIcon fontSize="small" />
                    </IconButton>
                  </Box>
                </Paper>
              ))}
            </Box>
          )}

          {/* Drop Zone */}
          <Tooltip
            title={audioPlaying ? "Cannot add files while DSP is active" : ""}
            disableInteractive
          >
            <Box>
              <Paper
                variant="outlined"
                onDrop={audioPlaying ? undefined : handleFileDrop}
                onDragOver={audioPlaying ? undefined : handleDragOver}
                onClick={audioPlaying ? undefined : handleDropZoneClick}
                sx={{
                  p: 3,
                  textAlign: "center",
                  border: "2px dashed",
                  borderColor: "divider",
                  backgroundColor: "action.hover",
                  cursor: audioPlaying ? "not-allowed" : "pointer",
                  opacity: audioPlaying ? 0.5 : 1,
                  pointerEvents: audioPlaying ? "none" : "auto",
                  filter: audioPlaying ? "grayscale(100%)" : "none",
                  "&:hover": audioPlaying ? {} : {
                    borderColor: "primary.main",
                    backgroundColor: "action.selected",
                  },
                }}
              >
                <Typography variant="body2" color="text.secondary">
                  Drop sound files here
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  Supports: .pcm, .wav, .mp3, .ogg, and other common audio formats
                </Typography>
              </Paper>
            </Box>
          </Tooltip>
          </Box>
        </Box>
      </Box>
    </Panel>
  );
};
