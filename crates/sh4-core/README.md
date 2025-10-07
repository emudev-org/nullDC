# sh4-core

SH4 CPU emulation core for nullDC

## Architecture

The SH4 context (`Sh4Ctx`) contains:
- CPU registers (R0-R15, banked registers R0_BANK-R7_BANK)
- Floating-point registers (FR0-FR15, XF0-XF15 banks)
- Control registers (SR, GBR, VBR, SSR, SPC, SGR, DBR, PR, FPSCR, FPUL, PC, MACH, MACL)
- **Memory map** (`memmap`, `memmask` arrays): Direct pointers to memory regions
- Execution state (PC pipeline, delay slot tracking, cycle counting)

### Execution Backends

Two execution backends are available:

- **IPR (Interpreter)**: Pure Rust interpreter using function dispatch
  - Used for testing and reference implementation
  - Handles delay slots and all SH4 instructions
  - Synchronizes host FPU flags (DAZ/FZ, rounding mode) with FPSCR

- **Unrolled Cached IPR**: gadget configuration for performance
  - Not yet fully implemented

### Memory Interface

Memory is accessed through pointer arrays stored directly in `Sh4Ctx`:
- `memmap[256]`: Array of pointers to memory regions (or null for handler-based regions)
- `memmask[256]`: Masks for each region
- `mem_handlers[256]`: Optional function pointers for custom memory access handling

Region is determined by the top 8 bits of the address (`addr >> 24`).

Two modes of memory access:
1. **Direct pointer access**: When `memmap[region]` is > 256 , memory is accessed directly through the pointer
2. **Handler-based access**: When `memmap[region]` is < 256, `memhandlers[memmap[region]][region]` is called with the masked address and operation type (read/write, size)

### FPU Emulation

The emulator synchronizes host FPU control flags with SH4's FPSCR:
- **Denormals-Are-Zero (DAZ/FZ)**: Set via FPSCR.DN bit
  - x86_64: MXCSR DAZ flag (bit 6)
  - aarch64: FPCR FZ flag (bit 24)
- **Rounding Mode**: Set via FPSCR.RM bits (0-1)
  - 00 = Round to nearest
  - 01 = Round to zero
  - 10/11 = Reserved (defaults to round to nearest)
  - Maps to x86_64 MXCSR bits 13-14 and aarch64 FPCR bits 22-23

## Usage

### Direct Memory Access

```rust
use sh4_core::{Sh4Ctx, sh4_ipr_dispatcher, sh4_init_ctx};

// Create and initialize context
let mut ctx = Sh4Ctx::default();
sh4_init_ctx(&mut ctx);

// Setup memory regions with direct pointer access
ctx.memmap[0x0C] = your_ram.as_mut_ptr();
ctx.memmask[0x0C] = 0x00FFFFFF; // 16MB

// Set initial PC and run
ctx.pc0 = 0x8C010000;
ctx.remaining_cycles = 1000;
sh4_ipr_dispatcher(&mut ctx);
```

## Testing

The test suite uses single-step test cases from the [SingleStepTests](https://github.com/SingleStepTests/sh4) project, located in `vendor/sh4-tests`.

Each test case is a json.bin file containing:
- Initial CPU state (registers, memory)
- Expected final CPU state after executing one instruction
- Memory operations trace (reads/writes with cycle numbers)

### Running Tests

```bash
cd crates/sh4-core
cargo test
```

## Building

```bash
cargo build --release
```

Supported platforms:
- **x86_64**: Full support including FPU flag synchronization
- **aarch64**: Full support (tested on Apple Silicon)
- **wasm32**: Basic support (no FPU flag control)

## License

See the main nullDC LICENSE file.
