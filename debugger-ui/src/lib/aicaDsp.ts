import init, { read_reg, write_reg, step, step128, step128_start, step128_end } from "../wasm/aica-dsp/aica_dsp.js";

class AicaDsp {
  private initialized = false;

  async initialize(): Promise<void> {
    if (this.initialized) {
      return;
    }

    try {
      await init();
      this.initialized = true;
    } catch (error) {
      console.error("Failed to initialize AICA DSP WASM module:", error);
      throw error;
    }
  }

  readReg(addr: number): number {
    if (!this.initialized) {
      throw new Error("AICA DSP WASM module not initialized");
    }
    return read_reg(addr);
  }

  writeReg(addr: number, data: number): void {
    if (!this.initialized) {
      throw new Error("AICA DSP WASM module not initialized");
    }
    write_reg(addr, data);
  }

  step(stepNum: number): void {
    if (!this.initialized) {
      throw new Error("AICA DSP WASM module not initialized");
    }
    step(stepNum);
  }

  step128(): void {
    if (!this.initialized) {
      throw new Error("AICA DSP WASM module not initialized");
    }
    step128();
  }

  step128Start(): void {
    if (!this.initialized) {
      throw new Error("AICA DSP WASM module not initialized");
    }
    step128_start();
  }

  step128End(): void {
    if (!this.initialized) {
      throw new Error("AICA DSP WASM module not initialized");
    }
    step128_end();
  }

  isInitialized(): boolean {
    return this.initialized;
  }
}

// Export singleton instance
export const aicaDsp = new AicaDsp();
