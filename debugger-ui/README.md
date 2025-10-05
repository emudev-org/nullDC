# nullDC Debugger

A web UI that connects to nullDC and exposes emulator hardware in an inspectable way.

## Development

- `npm install` installs dependencies.
- `npm run dev` launches the stock Vite dev server for the UI only.
- `npm run dev:mock` boots an Express-based mock debugger server that embeds Vite middleware, serves the UI, and exposes a JSON-RPC/WebSocket endpoint at `ws://localhost:5173/ws`.

The mock server watches source changes via `tsx` and Vite HMR, so front-end edits reload automatically and stub responses refresh without restarting.

### Runtime configuration

All transport choices are build-time options driven by Vite env vars:

| Variable | Description | Default |
| --- | --- | --- |
| `VITE_TRANSPORT_MODE` | `native` (WebSocket) or `wasm` (BroadcastChannel) | `native` |
| `VITE_WS_PATH` | Relative WebSocket path served by the host | `/ws` |
| `VITE_BROADCAST_CHANNEL` | Channel name used for WASM builds | `nulldc-debugger` |

The UI automatically connects to the host that served it. For WebSocket builds the URL is inferred from `window.location`; for WASM builds the BroadcastChannel is opened immediately so emulator-hosted tabs can communicate without extra UI toggles.

## Mock debugger protocol

The mock endpoint implements a subset of the planned JSON-RPC schema:

- Handshake, describe, subscribe/unsubscribe.
- Register/memory/disassembly fetches plus watch management.
- Breakpoint CRUD, basic stepping commands, event log, waveform, and log streaming topics.

It periodically emits fake register, waveform, watch, and event log updates so panels demonstrate live updates while the real emulator backend is under construction.
