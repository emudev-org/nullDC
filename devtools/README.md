# nullDC DevTools

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

## Features

### Breakpoints

The debugger UI provides multiple ways to manage breakpoints:

#### Disassembly View
- **Click gutter**: Add or remove breakpoint at that address
- **Ctrl+Click gutter** (or Cmd+Click on Mac): Toggle breakpoint enabled/disabled
- **Visual indicators**:
  - Red filled circle (●): Active breakpoint
  - Red outlined circle (○): Disabled breakpoint
  - Hover over empty gutter to see where you can add breakpoints

#### Breakpoints Panels
- **Events: Breakpoints**: Add event breakpoints using free-form event names (e.g., `dc.aica.channel[0].step`)
- **SH4: Breakpoints**: Add code breakpoints using hex addresses (e.g., `0x8C0000A0`)
- **ARM7: Breakpoints**: Add code breakpoints using hex addresses (e.g., `0x00200000`)

All breakpoints are persisted on the server and synchronized across all connected clients in real-time.

## Mock debugger protocol

The mock endpoint implements a subset of the planned JSON-RPC schema:

- Handshake, describe, subscribe/unsubscribe.
- Register/memory/disassembly fetches plus watch management.
- Breakpoint CRUD (add, remove, toggle), basic stepping commands, event log, waveform, and log streaming topics.

It periodically emits fake register, waveform, watch, and event log updates so panels demonstrate live updates while the real emulator backend is under construction.
