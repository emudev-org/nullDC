import init, { read_reg, write_reg, step, step128_start, step128_end, get_dsp_registers } from "../wasm/aica-dsp/aica_dsp.js";

class AicaDsp {
  private initialized = false;
  private currentStep = 0;
  private sampleCounter = 0;

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

  /**
   * Execute one DSP step with automatic sample boundary handling.
   * Manages step counter (0-127) and sample counter.
   */
  doDspStep(): void {
    if (!this.initialized) {
      throw new Error("AICA DSP WASM module not initialized");
    }

    // Start of new sample
    if (this.currentStep === 0) {
      step128_start();
    }

    // Execute the current step
    step(this.currentStep);

    // Increment step counter
    this.currentStep++;

    // End of sample
    if (this.currentStep === 128) {
      step128_end();
      this.currentStep = 0;
      this.sampleCounter++;
    }
  }

  /**
   * Run DSP until the start of the next sample (until currentStep === 0).
   */
  runToNextSample(): void {
    if (!this.initialized) {
      throw new Error("AICA DSP WASM module not initialized");
    }

    do {
      this.doDspStep();
    } while (this.currentStep !== 0);
  }

  /**
   * Get the current step number (0-127).
   */
  getCurrentStep(): number {
    return this.currentStep;
  }

  /**
   * Get the current sample counter.
   */
  getSampleCounter(): number {
    return this.sampleCounter;
  }

  /**
   * Reset step and sample counters.
   */
  resetCounters(): void {
    this.currentStep = 0;
    this.sampleCounter = 0;
  }

  /**
   * Get DSP internal registers.
   * Returns: [MDEC_CT, ACC, SHIFTED, X, Y, B, INPUTS, MEMVAL[0-3], FRC_REG, Y_REG, ADRS_REG]
   */
  getDspRegisters(): Int32Array {
    if (!this.initialized) {
      throw new Error("AICA DSP WASM module not initialized");
    }
    return get_dsp_registers();
  }

  isInitialized(): boolean {
    return this.initialized;
  }
}

// Export singleton instance
export const aicaDsp = new AicaDsp();
