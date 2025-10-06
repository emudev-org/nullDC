# sh4-core

A standalone SH4 CPU emulation core extracted from the nullDC Dreamcast emulator.

## Features

- **Self-contained**: No dependencies on emulator-specific code
- **Minimal dependencies**: Only requires `bitfield`, `paste`, and `seq-macro`
- **Direct memory access**: Uses memory pointers stored in `Sh4Ctx` for zero-cost abstraction
- **Multiple backends**:
  - Interpreter (`backend_ipr`): Direct execution of instructions
  - Function generator (`backend_fns`): JIT-like block compilation

## Architecture

The SH4 context (`Sh4Ctx`) contains:
- CPU registers (R0-R15, bank registers)
- Floating-point registers (FR, XF banks)
- Control registers (SR, PR, FPSCR, etc.)
- **Memory map** (`memmap`, `memmask` arrays): Direct pointers to memory regions
- Execution state (PC, delay slot tracking, etc.)

### Memory Interface

Memory is accessed through pointer arrays stored directly in `Sh4Ctx`:
- `memmap[256]`: Array of pointers to memory regions
- `memmask[256]`: Masks for each region

Region is determined by the top 8 bits of the address (`addr >> 24`).

## Usage

```rust
use sh4_core::{Sh4Ctx, sh4_ipr_dispatcher, sh4_init_ctx};

// Create and initialize context
let mut ctx = Sh4Ctx::default();
sh4_init_ctx(&mut ctx);

// Setup memory regions
ctx.memmap[0x0C] = your_ram.as_mut_ptr();
ctx.memmask[0x0C] = 0x00FFFFFF; // 16MB

// Set initial PC and run
ctx.pc0 = 0x8C010000;
ctx.remaining_cycles = 1000;
sh4_ipr_dispatcher(&mut ctx);
```

## Testing with SingleStepTests

The `vendor/sh4-tests` submodule contains single-step test cases from the [SingleStepTests](https://github.com/SingleStepTests/sh4) project.

### Test Format

Each test is a JSON file with:
- Initial register/memory state
- The instruction to execute
- Expected final state

### Running Tests

```bash
cd crates/sh4-core
cargo test
```

### Adding New Tests

TODO: Implement JSON test loader that:
1. Parses test case JSON from `../../vendor/sh4-tests`
2. Loads initial state into `Sh4Ctx`
3. Executes one instruction via interpreter
4. Validates final state

## Building

```bash
cargo build --release
```

## License

See the main nullDC LICENSE file.
