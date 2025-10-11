/**
 * AICA DSP high-level language compiler
 * TypeScript port of the Python compiler from vendor/aica-dsp-compiler/dspc
 *
 * GPL-2.0-or-later
 * Copyright (C) 2025 Paul Cercueil <paul@crapouillou.net>
 * TypeScript transvibe by Stefanos Kornilios Mitsis Poiitidis <skmp@emudev.org>
 * transvibe under MIT license, with permission from original author
 */

import { preprocessDspSource } from './dspCompilerPreprocessor';

const INPUT_MAX: Record<string, number> = {
  mems: 32,
  mixer: 16,
  cdda: 2,
};

const INPUT_OFFSET: Record<string, number> = {
  mems: 0,
  mixer: 32,
  cdda: 48,
};

const SMODES: Record<string, number> = {
  sat: 0,
  sat2: 1,
  trim2: 2,
  trim: 3,
};

function BIT(x: number): bigint {
  return BigInt(1) << BigInt(x);
}

function GENMASK(h: number, l: number): bigint {
  const mask = (BigInt('0xffffffffffffffff') << BigInt(l)) & (BigInt('0xffffffffffffffff') >> BigInt(63 - h));
  return mask;
}

function CTZ(value: bigint): number {
  if (value === BigInt(0)) return -1;
  let count = 0;
  let v = value;
  while ((v & BigInt(1)) === BigInt(0)) {
    v >>= BigInt(1);
    count++;
  }
  return count;
}

function FIELD_GET(field: bigint, value: bigint): number {
  return Number((value & field) >> BigInt(CTZ(field)));
}

function FIELD_PREP(field: bigint, value: number | bigint): bigint {
  return (BigInt(value) << BigInt(CTZ(field))) & field;
}

const TRA = GENMASK(63, 57);
const TWT = BIT(56);
const TWA = GENMASK(55, 49);
const XSEL = BIT(47);
const YSEL = GENMASK(46, 45);
const IRA = GENMASK(44, 39);
const IWT = BIT(38);
const IWA = GENMASK(37, 33);
const TABLE = BIT(31);
const MWT = BIT(30);
const MRD = BIT(29);
const EWT = BIT(28);
const EWA = GENMASK(27, 24);
const ADRL = BIT(23);
const FRCL = BIT(22);
const SHIFT = GENMASK(21, 20);
const YRL = BIT(19);
const NEGB = BIT(18);
const ZERO = BIT(17);
const BSEL = BIT(16);
const NOFL = BIT(15);
const MASA = GENMASK(14, 9);
const ADREB = BIT(8);
const NXADR = BIT(7);

const dspFields = [
  "TRA", "TWT", "TWA", "XSEL", "YSEL", "IRA", "IWT", "IWA",
  "TABLE", "MWT", "MRD", "EWT", "EWA", "ADRL", "FRCL", "SHIFT",
  "YRL", "NEGB", "ZERO", "BSEL", "NOFL", "MASA", "ADREB", "NXADR"
];

const dummyAcc = FIELD_PREP(YSEL, 1) | BSEL; // acc = x * 0 + acc

interface CompilerState {
  steps: bigint[];
  coefs: Record<number, number>;
  madrs: string[];
  nofl: number;
  smode: number;
  imode: number;
  errors: Array<{ line: number; message: string }>;
}

function createSteps(lines: string[]): { steps: bigint[]; coefs: number[]; madrs: string[]; errors: Array<{ line: number; message: string }> } {
  const state: CompilerState = {
    steps: [],
    coefs: {},
    madrs: [],
    nofl: 0,
    smode: 0,
    imode: 0,
    errors: [],
  };

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
    let line = lines[lineIndex].trim();
    const lineNum = lineIndex + 1;

    if (line.length === 0) continue;

    // Comments
    if (/^(\/\/|#)/.test(line)) continue;

    // MADRS
    let match = /^MADRS\s*\[\s*(\d+)\s*\]\s*=\s*(-?\d+|0[xX][0-9a-fA-F]+)$/.exec(line);
    if (match) {
      state.madrs.push(line);
      continue;
    }

    // INPUT
    match = /^INPUT\s+(mixer|mems|cdda):(\d+)$/i.exec(line);
    if (match) {
      const idx = parseInt(match[2], 10);
      const type = match[1].toLowerCase();
      if (idx >= INPUT_MAX[type]) {
        state.errors.push({ line: lineNum, message: `Invalid instruction: ${match[0]}` });
        continue;
      }
      state.imode = idx + INPUT_OFFSET[type];
      continue;
    }

    // OUTPUT yreg
    match = /^OUTPUT\s+yreg$/i.exec(line);
    if (match) {
      state.steps.push(dummyAcc | FIELD_PREP(IRA, state.imode) | YRL);
      continue;
    }

    // OUTPUT adrs
    match = /^OUTPUT\s+adrs$/i.exec(line);
    if (match) {
      if (state.smode === 3) {
        state.steps.push(dummyAcc | FIELD_PREP(SHIFT, state.smode) | ADRL);
        state.steps.push(dummyAcc | FIELD_PREP(IRA, state.imode) | ADRL);
      } else {
        state.steps.push(dummyAcc | FIELD_PREP(IRA, state.imode) | FIELD_PREP(SHIFT, state.smode) | ADRL);
      }
      continue;
    }

    // OUTPUT adrs/s
    match = /^OUTPUT\s+adrs\/s$/i.exec(line);
    if (match) {
      state.steps.push(dummyAcc | FIELD_PREP(IRA, state.imode) | ADRL | FIELD_PREP(SHIFT, 3));
      continue;
    }

    // OUTPUT mixer
    match = /^OUTPUT\s+mixer:(\d+)$/i.exec(line);
    if (match) {
      const idx = parseInt(match[1], 10);
      if (idx >= 16) {
        state.errors.push({ line: lineNum, message: `Invalid instruction: ${match[0]}` });
        continue;
      }
      state.steps.push(dummyAcc | EWT | FIELD_PREP(EWA, idx) | FIELD_PREP(SHIFT, state.smode));
      continue;
    }

    // MAC
    match = /^MAC\s+(input|\[\s*temp:(\d+)\s*\])\s*,\s*((shifted|yreg):(lo|hi)|#0[xX][0-9a-fA-F]+|#-?\d+)(\s*,\s*(-?)(acc|\[\s*temp:(\d+)\s*\]))?/i.exec(line);
    if (match) {
      let xsel: bigint, tra: bigint, ira: bigint;
      if (match[1] === "input") {
        xsel = XSEL;
        tra = BigInt(0);
        ira = FIELD_PREP(IRA, state.imode);
      } else {
        xsel = BigInt(0);
        tra = FIELD_PREP(TRA, parseInt(match[2], 10));
        ira = BigInt(0);
      }

      let ysel: bigint;
      if (match[4] === "yreg") {
        if (match[5] === "lo") {
          ysel = FIELD_PREP(YSEL, 3);
        } else {
          ysel = FIELD_PREP(YSEL, 2);
        }
      } else if (match[4] === "shifted") {
        const newOp = dummyAcc | FRCL;
        if (match[5] === "lo") {
          state.steps.push(newOp | FIELD_PREP(SHIFT, 3));
        } else {
          state.steps.push(newOp);
        }
        ysel = BigInt(0);
      } else {
        ysel = FIELD_PREP(YSEL, 1);
        state.coefs[state.steps.length] = parseInt(match[3].substring(1), 0) << 3;
      }

      let negb: bigint, zero: bigint, bsel: bigint;
      if (match[6] !== undefined) {
        negb = FIELD_PREP(NEGB, match[7] === '-' ? 1 : 0);
        zero = BigInt(0);

        if (match[8] === "acc") {
          bsel = BSEL;
        } else {
          bsel = BigInt(0);
          const tra2 = FIELD_PREP(TRA, parseInt(match[9], 10));
          if (xsel === BigInt(0) && tra !== tra2) {
            state.errors.push({ line: lineNum, message: `Invalid instruction: ${match[0]}` });
            continue;
          }
          tra = tra2;
        }
      } else {
        bsel = BigInt(0);
        negb = BigInt(0);
        zero = ZERO;
      }

      state.steps.push(ira | xsel | tra | ysel | negb | zero | bsel);
      continue;
    }

    // SMODE
    match = /^SMODE\s+(sat|trim|sat2|trim2)$/i.exec(line);
    if (match) {
      state.smode = SMODES[match[1].toLowerCase()];
      continue;
    }

    // ST [temp:N]
    match = /^ST\s+\[\s*temp:(\d+)\s*\]$/i.exec(line);
    if (match) {
      const twa = parseInt(match[1], 10);
      if (twa >= 128) {
        state.errors.push({ line: lineNum, message: `Invalid instruction: ${match[0]}` });
        continue;
      }
      state.steps.push(dummyAcc | FIELD_PREP(SHIFT, state.smode) | TWT | FIELD_PREP(TWA, twa));
      continue;
    }

    // ST(F) madrs
    match = /^ST(F)?\s+(\[)?madrs:(\d+)(\s*\+)?(?:\/s)?(\])?$/i.exec(line);
    if (match) {
      const masa = parseInt(match[3], 10);
      const table = match[2] ? BigInt(0) : TABLE;
      const adreb = /\/s/.test(line) ? ADREB : BigInt(0);
      const nxadr = match[4] ? NXADR : BigInt(0);
      const nofl = match[1] ? BigInt(0) : NOFL;

      if (masa >= 64 || (!!match[2] !== !!match[5])) {
        state.errors.push({ line: lineNum, message: `Invalid instruction: ${match[0]}` });
        continue;
      }

      // Align to odd step
      if ((state.steps.length & 1) === 0) {
        state.steps.push(dummyAcc);
      }

      state.steps.push(dummyAcc | FIELD_PREP(SHIFT, state.smode) | MWT | table | adreb | nxadr | nofl | FIELD_PREP(MASA, masa));
      continue;
    }

    // LD(F) madrs, mems
    match = /^LD(F)?\s+(\[)?madrs:(\d+)(\s*\+)?(?:\/s)?(\])?\s*,\s*mems:(\d+)$/i.exec(line);
    if (match) {
      const masa = parseInt(match[3], 10);
      const iwa = parseInt(match[6], 10);
      const table = match[2] ? BigInt(0) : TABLE;
      const adreb = /\/s/.test(line) ? ADREB : BigInt(0);
      const nxadr = match[4] ? NXADR : BigInt(0);
      const nofl = match[1] ? BigInt(0) : NOFL;

      if (masa >= 64 || iwa >= 32 || (!!match[2] !== !!match[5])) {
        state.errors.push({ line: lineNum, message: `Invalid instruction: ${match[0]}` });
        continue;
      }

      // Align to odd step
      if ((state.steps.length & 1) === 0) {
        state.steps.push(dummyAcc);
      }

      state.steps.push(dummyAcc | MRD | table | adreb | nxadr | nofl | FIELD_PREP(MASA, masa));
      state.steps.push(dummyAcc);
      state.steps.push(dummyAcc | IWT | FIELD_PREP(IWA, iwa));
      continue;
    }

    state.errors.push({ line: lineNum, message: `Unhandled instruction: ${line}` });
  }

  // Convert coefs dict to sparse array
  const coefsArray = Array.from({ length: state.steps.length }, (_, i) => state.coefs[i] || 0);

  return { steps: state.steps, coefs: coefsArray, madrs: state.madrs, errors: state.errors };
}

function optLoads(steps: bigint[]): bigint[] {
  for (let idx = 3; idx < steps.length; idx++) {
    let step = steps[idx];

    if (!FIELD_GET(MRD, step) || FIELD_GET(IWT, step)) {
      continue;
    }

    const iwa = FIELD_GET(IWA, steps[idx + 2]);
    let oldIdx = idx;

    while (oldIdx > 2 && (step & IWT) === BigInt(0)) {
      oldIdx -= 1;
      step = steps[oldIdx];

      // Break if previous opcode reads INPUTS from the mems register we're loading to
      if ((step & (ADRL | YRL | XSEL)) !== BigInt(0) && FIELD_GET(IRA, step) === iwa) {
        break;
      }
    }

    // Align to odd
    oldIdx += ((oldIdx & 1) ^ 1);

    // If this step has MWT, we can't reuse the table/adreb/etc. bits
    while ((steps[oldIdx] & MWT) !== BigInt(0)) {
      oldIdx += 2;
    }

    if (oldIdx < idx) {
      steps[oldIdx] |= steps[idx] & (MRD | TABLE | ADREB | NXADR | MASA | NOFL);
      steps[oldIdx + 2] |= IWT | FIELD_PREP(IWA, iwa);
      steps[idx] &= ~(MRD | TABLE | ADREB | NXADR | MASA | NOFL);
      steps[idx + 2] &= ~(IWT | IWA);
    }
  }

  return steps;
}

function trickleDown(steps: bigint[], coefs: number[]): void {
  const nbSteps = steps.length;

  while (true) {
    let found = false;

    for (let idx = nbSteps - 1; idx >= 1; idx--) {
      const step = steps[idx];

      if (step === dummyAcc || (step & (MWT | MRD | IWT)) !== BigInt(0)) {
        continue;
      }

      if (steps[idx - 1] === dummyAcc && coefs[idx - 1] === 0) {
        // Previous instruction is a NOP, swap with it
        coefs[idx - 1] = coefs[idx];
        steps[idx - 1] = steps[idx];
        steps[idx] = dummyAcc;
        coefs[idx] = 0;
        found = true;
      }
    }

    if (!found) break;
  }
}

function dropNops(steps: bigint[], coefs: number[]): { steps: bigint[]; coefs: number[] } {
  let nbSteps = steps.length;
  let wasNop = false;

  for (let idx = nbSteps - 1; idx >= 0; idx--) {
    const step = steps[idx];

    if (step === dummyAcc && coefs[idx] === 0) {
      if (wasNop) {
        steps = steps.slice(0, idx).concat(steps.slice(idx + 2));
        coefs = coefs.slice(0, idx).concat(coefs.slice(idx + 2));
        wasNop = false;
      } else {
        wasNop = true;
      }
      continue;
    }

    wasNop = false;
  }

  return { steps, coefs };
}

function generateAsm(steps: bigint[], coefs: number[], madrs: string[]): string {
  const lines: string[] = [];

  // Add MADRS lines
  for (const line of madrs) {
    lines.push(line);
  }

  // Add COEF and MPRO lines
  for (let idx = 0; idx < steps.length; idx++) {
    if (idx < coefs.length && coefs[idx]) {
      lines.push(`COEF[${idx}] = ${coefs[idx]}`);
    }

    let line = `MPRO[${idx}] =`;
    const step = steps[idx];

    const fieldVals: Record<string, bigint> = {
      TRA, TWT, TWA, XSEL, YSEL, IRA, IWT, IWA,
      TABLE, MWT, MRD, EWT, EWA, ADRL, FRCL, SHIFT,
      YRL, NEGB, ZERO, BSEL, NOFL, MASA, ADREB, NXADR
    };

    for (const field of dspFields) {
      const val = FIELD_GET(fieldVals[field], step);
      if (val) {
        if (val === 1) {
          line += ` ${field}`;
        } else {
          line += ` ${field}:${val}`;
        }
      }
    }

    lines.push(line);
  }

  return lines.join('\n');
}

export interface CompileError {
  line: number;
  message: string;
}

export class CompilationError extends Error {
  errors: CompileError[];

  constructor(errors: CompileError[]) {
    super(errors.map(e => `Line ${e.line}: ${e.message}`).join('\n'));
    this.name = 'CompilationError';
    this.errors = errors;
  }
}

// Store the last preprocessed macros for hover support
let lastMacros: Map<string, import('./dspCompilerPreprocessor').MacroDefinition> = new Map();

export function getLastPreprocessedMacros() {
  return lastMacros;
}

export function compileDspSource(source: string): string {
  // Preprocess the source (expand macros, handle #define)
  const { output: preprocessedSource, errors: preprocessErrors, macros } = preprocessDspSource(source);

  // Store macros for hover support
  lastMacros = macros;

  // If preprocessing had errors, include them in the compilation errors
  const lines = preprocessedSource.split('\n');
  let { steps, coefs, madrs, errors } = createSteps(lines);

  // Combine preprocessing and compilation errors
  const allErrors = [...preprocessErrors, ...errors];

  // If there are errors, throw them all
  if (allErrors.length > 0) {
    throw new CompilationError(allErrors);
  }

  steps = optLoads(steps);
  trickleDown(steps, coefs);
  ({ steps, coefs } = dropNops(steps, coefs));
  return generateAsm(steps, coefs, madrs);
}
