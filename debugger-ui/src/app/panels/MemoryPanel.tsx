import { useMemo } from "react";
import { useSearchParams } from "react-router-dom";
import { Panel } from "../layout/Panel";
import type { MemorySlice } from "../../lib/debuggerSchema";
import { useSessionStore } from "../../state/sessionStore";
import { useDebuggerDataStore } from "../../state/debuggerDataStore";
import { MemoryView, type MemoryViewConfig, type MemoryViewCallbacks } from "../components/MemoryView";

const BYTES_PER_ROW = 16;
const VISIBLE_ROWS = 60;

const formatHexAddress = (value: number) => `0x${value.toString(16).toUpperCase().padStart(8, "0")}`;

const parseAddressInput = (input: string) => {
  const normalized = input.trim();
  const parsed = Number.parseInt(normalized.replace(/^0x/i, ""), 16);
  return Number.isNaN(parsed) ? undefined : parsed;
};

interface MemoryPanelProps {
  target: string;
  defaultAddress: number;
  encoding?: MemorySlice["encoding"];
  wordSize?: MemorySlice["wordSize"];
}

const MemoryPanel = ({ target, defaultAddress, encoding, wordSize }: MemoryPanelProps) => {
  const [searchParams, setSearchParams] = useSearchParams();
  const client = useSessionStore((state) => state.client);
  const initialized = useDebuggerDataStore((state) => state.initialized);

  const length = VISIBLE_ROWS * BYTES_PER_ROW;

  // Build configuration
  const config: MemoryViewConfig = useMemo(
    () => ({
      formatAddress: formatHexAddress,
      parseAddress: parseAddressInput,
      maxAddress: 0xffffffff - Math.max(length - 1, 0),
      length,
    }),
    [length],
  );

  // Build callbacks
  const callbacks: MemoryViewCallbacks = useMemo(
    () => ({
      onFetchMemory: async (address: number, length: number, encoding?: MemorySlice["encoding"], wordSize?: MemorySlice["wordSize"]) => {
        if (!client) {
          throw new Error("Client not connected");
        }
        return await client.fetchMemorySlice({
          target,
          address,
          length,
          encoding,
          wordSize,
        });
      },
      onAddressChange: (address: number) => {
        setSearchParams({ address: formatHexAddress(address) });
      },
    }),
    [client, target, setSearchParams],
  );

  // Get initial URL address
  const initialUrlAddress = useMemo(() => {
    const addressParam = searchParams.get("address");
    if (addressParam) {
      const parsed = parseAddressInput(addressParam);
      if (parsed !== undefined) {
        return { address: parsed, fromUrl: true };
      }
    }
    return undefined;
  }, [searchParams]);

  return (
    <Panel>
      <MemoryView
        config={config}
        callbacks={callbacks}
        defaultAddress={defaultAddress}
        initialized={initialized}
        initialUrlAddress={initialUrlAddress}
        encoding={encoding}
        wordSize={wordSize}
      />
    </Panel>
  );
};

export const Sh4MemoryPanel = () => (
  <MemoryPanel target="sh4" defaultAddress={0x8c000000} />
);

export const Arm7MemoryPanel = () => (
  <MemoryPanel target="arm7" defaultAddress={0x00200000} />
);
