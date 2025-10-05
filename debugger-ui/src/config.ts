import type { DebuggerClientConfig } from "./services/debuggerClient";
import type { TransportOptions } from "./services/transport";

type TransportMode = DebuggerClientConfig["mode"];

type TransportConfig = {
  mode: TransportMode;
  websocketPath: string;
  broadcastChannel: string;
  clientName: string;
  clientVersion: string;
};

const TRANSPORT_MODE = (import.meta.env.VITE_TRANSPORT_MODE ?? "native") as TransportMode;

export const appConfig: TransportConfig = {
  mode: TRANSPORT_MODE,
  websocketPath: import.meta.env.VITE_WS_PATH ?? "/ws",
  broadcastChannel: import.meta.env.VITE_BROADCAST_CHANNEL ?? "nulldc-debugger",
  clientName: "nullDC Debugger UI",
  clientVersion: "0.1.0",
};

export const resolveEndpoint = (): string => {
  if (appConfig.mode === "native") {
    const { protocol, host } = window.location;
    const wsProtocol = protocol === "https:" ? "wss:" : "ws:";
    return `${wsProtocol}//${host}${appConfig.websocketPath}`;
  }
  return appConfig.broadcastChannel;
};

export const resolveTransportOptions = (): TransportOptions | undefined => {
  if (appConfig.mode === "wasm") {
    return { channelName: appConfig.broadcastChannel };
  }
  return undefined;
};
