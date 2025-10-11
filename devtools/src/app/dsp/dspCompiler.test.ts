/**
 * DSP Compiler Tests
 */

import { compileDspSource } from './dspCompiler';
import { DEFAULT_DSP_SOURCE } from './defaultDspSource';
import { readFileSync } from 'fs';
import { join } from 'path';

// Read the expected output from the test fixtures
const EXPECTED_REVERB_ASM = readFileSync(
  join(__dirname, 'test-fixtures', 'reverb.asm'),
  'utf-8'
).trim();

describe('DSP Compiler', () => {
  describe('compileDspSource', () => {
    it('should compile the default DSP source successfully', () => {
      // This test verifies that the compiler runs without errors
      const result = compileDspSource(DEFAULT_DSP_SOURCE);

      expect(result).toBeTruthy();
    });

    it('should produce correct output for reverb.txt', () => {
      const result1 = compileDspSource(DEFAULT_DSP_SOURCE);

      expect(result1).toBe(EXPECTED_REVERB_ASM);
    });
  });
});
