import type { aicaDsp } from "../../lib/aicaDsp";
import { preprocessDspSource } from "./dspCompilerPreprocessor";

type AicaDsp = typeof aicaDsp;

// Store the last preprocessed macros for hover support in assembly editor
let lastAssemblyMacros: Map<string, import('./dspCompilerPreprocessor').MacroDefinition> = new Map();

export function getLastAssemblyMacros() {
  return lastAssemblyMacros;
}

export interface DspInstDesc {
  TRA: number;
  TWT: number;
  TWA: number;
  XSEL: number;
  YSEL: number;
  IRA: number;
  IWT: number;
  IWA: number;
  TABLE: number;
  MWT: number;
  MRD: number;
  EWT: number;
  EWA: number;
  ADRL: number;
  FRCL: number;
  SHIFT: number;
  YRL: number;
  NEGB: number;
  ZERO: number;
  BSEL: number;
  NOFL: number;
  MASA: number;
  ADREB: number;
  NXADR: number;
}

export interface ParsedData {
  COEF: Array<{ index: number; value: number }>;
  MADRS: Array<{ index: number; value: number }>;
  MEMS: Array<{ index: number; low?: number; high?: number }>;
  MPRO: Array<{ index: number; encoded: number[] }>;
}

export const decodeInst = (inst: number[]): DspInstDesc => {
  return {
    TRA: (inst[0] >> 9) & 0x7f,
    TWT: (inst[0] >> 8) & 0x01,
    TWA: (inst[0] >> 1) & 0x7f,

    XSEL: (inst[1] >> 15) & 0x01,
    YSEL: (inst[1] >> 13) & 0x03,
    IRA: (inst[1] >> 7) & 0x3f,
    IWT: (inst[1] >> 6) & 0x01,
    IWA: (inst[1] >> 1) & 0x1f,

    TABLE: (inst[2] >> 15) & 0x01,
    MWT: (inst[2] >> 14) & 0x01,
    MRD: (inst[2] >> 13) & 0x01,
    EWT: (inst[2] >> 12) & 0x01,
    EWA: (inst[2] >> 8) & 0x0f,
    ADRL: (inst[2] >> 7) & 0x01,
    FRCL: (inst[2] >> 6) & 0x01,
    SHIFT: (inst[2] >> 4) & 0x03,
    YRL: (inst[2] >> 3) & 0x01,
    NEGB: (inst[2] >> 2) & 0x01,
    ZERO: (inst[2] >> 1) & 0x01,
    BSEL: (inst[2] >> 0) & 0x01,

    NOFL: (inst[3] >> 15) & 0x01,
    MASA: (inst[3] >> 9) & 0x3f,
    ADREB: (inst[3] >> 8) & 0x01,
    NXADR: (inst[3] >> 7) & 0x01,
  };
};

export const encodeInst = (desc: Partial<DspInstDesc>): number[] => {
  const inst = [0, 0, 0, 0];

  const fields: Array<keyof DspInstDesc> = [
    "TRA",
    "TWT",
    "TWA",
    "XSEL",
    "YSEL",
    "IRA",
    "IWT",
    "IWA",
    "TABLE",
    "MWT",
    "MRD",
    "EWT",
    "EWA",
    "ADRL",
    "FRCL",
    "SHIFT",
    "YRL",
    "NEGB",
    "ZERO",
    "BSEL",
    "NOFL",
    "MASA",
    "ADREB",
    "NXADR",
  ];

  const keys = Object.keys(desc) as Array<keyof DspInstDesc>;
  keys.forEach((key) => {
    if (!fields.includes(key)) {
      throw new Error(`Invalid instruction field: ${key}`);
    }
  });

  inst[0] |= ((desc.TRA ?? 0) & 0x7f) << 9;
  inst[0] |= ((desc.TWT ?? 0) & 0x01) << 8;
  inst[0] |= ((desc.TWA ?? 0) & 0x7f) << 1;

  inst[1] |= ((desc.XSEL ?? 0) & 0x01) << 15;
  inst[1] |= ((desc.YSEL ?? 0) & 0x03) << 13;
  inst[1] |= ((desc.IRA ?? 0) & 0x3f) << 7;
  inst[1] |= ((desc.IWT ?? 0) & 0x01) << 6;
  inst[1] |= ((desc.IWA ?? 0) & 0x1f) << 1;

  inst[2] |= ((desc.TABLE ?? 0) & 0x01) << 15;
  inst[2] |= ((desc.MWT ?? 0) & 0x01) << 14;
  inst[2] |= ((desc.MRD ?? 0) & 0x01) << 13;
  inst[2] |= ((desc.EWT ?? 0) & 0x01) << 12;
  inst[2] |= ((desc.EWA ?? 0) & 0x0f) << 8;
  inst[2] |= ((desc.ADRL ?? 0) & 0x01) << 7;
  inst[2] |= ((desc.FRCL ?? 0) & 0x01) << 6;
  inst[2] |= ((desc.SHIFT ?? 0) & 0x03) << 4;
  inst[2] |= ((desc.YRL ?? 0) & 0x01) << 3;
  inst[2] |= ((desc.NEGB ?? 0) & 0x01) << 2;
  inst[2] |= ((desc.ZERO ?? 0) & 0x01) << 1;
  inst[2] |= ((desc.BSEL ?? 0) & 0x01) << 0;

  inst[3] |= ((desc.NOFL ?? 0) & 0x01) << 15;
  inst[3] |= ((desc.MASA ?? 0) & 0x3f) << 9;
  inst[3] |= ((desc.ADREB ?? 0) & 0x01) << 8;
  inst[3] |= ((desc.NXADR ?? 0) & 0x01) << 7;

  return inst;
};

export const disassembleDesc = (desc: DspInstDesc): string => {
  // Fields that are 1-bit flags (only show name when set to 1)
  const oneBitFields = new Set([
    'TWT', 'XSEL', 'IWT', 'TABLE', 'MWT', 'MRD', 'EWT',
    'ADRL', 'FRCL', 'YRL', 'NEGB', 'ZERO', 'BSEL', 'NOFL', 'ADREB', 'NXADR'
  ]);

  return Object.entries(desc)
    .filter(([, value]) => value !== 0)
    .map(([key, value]) => {
      // For 1-bit fields with value 1, just show the name
      if (oneBitFields.has(key) && value === 1) {
        return key;
      }
      // For all other fields or values, show name:value
      return `${key}:${value}`;
    })
    .join(" ");
};

export const assembleDesc = (text: string): Partial<DspInstDesc> => {
  const rv = getDefaultInst();
  const tokens = text.split(/\s+/).filter(t => t.length > 0);

  tokens.forEach((token) => {
    const split = token.split(":");

    const key = split[0];
    if (!key) {
      throw new Error(`Invalid asm ${text}`);
    }

    let value = split[1];

    // Default to 1 if value is not provided
    if (value === undefined) {
      value = "1";
    }

    // Validate that the value is a valid number (decimal or hex)
    const numValue = value.startsWith("0x") || value.startsWith("0X")
      ? parseInt(value, 16)
      : parseInt(value, 10);

    if (isNaN(numValue) || !/^(0[xX][0-9a-fA-F]+|\d+)$/.test(value)) {
      throw new Error(`Invalid value '${value}' for field '${key}' in: ${text}`);
    }

    (rv as Record<string, number>)[key] = numValue;
  });

  return rv;
};

export const getDefaultInst = (): Partial<DspInstDesc> => {
  return {
    TRA: 0,
    TWT: 0,
    TWA: 0,
    XSEL: 0,
    YSEL: 0,
    IRA: 0,
    IWT: 0,
    IWA: 0,
    TABLE: 0,
    MWT: 0,
    MRD: 0,
    EWT: 0,
    EWA: 0,
    ADRL: 0,
    FRCL: 0,
    SHIFT: 0,
    YRL: 0,
    NEGB: 0,
    ZERO: 0,
    BSEL: 0,
    NOFL: 0,
    MASA: 0,
    ADREB: 0,
    NXADR: 0,
  };
};

const parseDecOrHex16 = (str: string): number => {
  const rv = parseInt(str);
  return rv & 0xffff;
};

// Assemble with preprocessing support
export const assembleSourceWithPreprocessing = (source: string): ParsedData => {
  const { output: preprocessedSource, macros } = preprocessDspSource(source);

  // Store macros for hover support
  lastAssemblyMacros = macros;

  const lines = preprocessedSource.split('\n');
  return assembleSource(lines);
};

export const assembleSource = (source: string[]): ParsedData => {
  const parsedData: ParsedData = {
    COEF: [],
    MADRS: [],
    MEMS: [],
    MPRO: [],
  };

  source.forEach((line, lineNumber) => {
    line = line.trim();

    // Skip empty lines and comments
    if (line === "" || line.startsWith("#") || line.startsWith("//")) {
      return;
    }

    // Determine the type of data and parse accordingly
    const coefMatch = line.match(/^COEF\[(\d+)\]\s*=\s*(\d+|[-0-9a-fA-Fx]+)$/);
    const madrsMatch = line.match(/^MADRS\[(\d+)\]\s*=\s*(\d+|[-0-9a-fA-Fx]+)$/);
    const memsLMatch = line.match(/^MEMS_L\[(\d+)\]\s*=\s*(\d+|[-0-9a-fA-Fx]+)$/);
    const memsHMatch = line.match(/^MEMS_H\[(\d+)\]\s*=\s*(\d+|[-0-9a-fA-Fx]+)$/);
    const mproMatch = line.match(/^MPRO\[(\d+)\]\s*=\s*(.+)$/);

    if (coefMatch) {
      parsedData.COEF.push({
        index: parseInt(coefMatch[1], 10),
        value: parseDecOrHex16(coefMatch[2]),
      });
    } else if (madrsMatch) {
      parsedData.MADRS.push({
        index: parseInt(madrsMatch[1], 10),
        value: parseDecOrHex16(madrsMatch[2]),
      });
    } else if (memsLMatch) {
      const index = parseInt(memsLMatch[1], 10);
      const valueL = parseDecOrHex16(memsLMatch[2]);

      // Add MEMS_L or pair with existing MEMS_H
      const existingEntry = parsedData.MEMS.find((entry) => entry.index === index);
      if (existingEntry) {
        existingEntry.low = valueL;
      } else {
        parsedData.MEMS.push({ index, low: valueL });
      }
    } else if (memsHMatch) {
      const index = parseInt(memsHMatch[1], 10);
      const valueH = parseDecOrHex16(memsHMatch[2]);

      // Add MEMS_H or pair with existing MEMS_L
      const existingEntry = parsedData.MEMS.find((entry) => entry.index === index);
      if (existingEntry) {
        existingEntry.high = valueH;
      } else {
        parsedData.MEMS.push({ index, high: valueH });
      }
    } else if (mproMatch) {
      const index = parseInt(mproMatch[1], 10);
      const encodedDesc = mproMatch[2];

      try {
        // Use `assembleDesc` to parse the description into an object
        const desc = assembleDesc(encodedDesc);

        // Use `encodeInst` to encode the description into instruction data
        const inst = encodeInst(desc);

        parsedData.MPRO.push({
          index,
          encoded: inst,
        });
      } catch (error) {
        throw new Error(
          `Error processing MPRO line at line ${lineNumber + 1}: ${line}. ${error instanceof Error ? error.message : String(error)}`
        );
      }
    } else {
      throw new Error(`Invalid data line at line ${lineNumber + 1}: ${line}`);
    }
  });

  return parsedData;
};

export const writeRegisters = (dsp: AicaDsp, parsedData: ParsedData): void => {
  // Write COEF registers
  for (let i = 0; i < 128; i++) {
    dsp.writeReg(0x3000 + i * 4, 0);
  }
  parsedData.COEF.forEach(({ index, value }) => {
    dsp.writeReg(0x3000 + index * 4, value);
  });

  // Write MADRS registers
  for (let i = 0; i < 64; i++) {
    dsp.writeReg(0x3200 + i * 4, 0);
  }
  parsedData.MADRS.forEach(({ index, value }) => {
    dsp.writeReg(0x3200 + index * 4, value);
  });

  // Write MEMS registers
  for (let i = 0; i < 32; i++) {
    dsp.writeReg(0x4400 + i * 8 + 0, 0);
    dsp.writeReg(0x4400 + i * 8 + 4, 0);
  }
  parsedData.MEMS.forEach(({ index, low, high }) => {
    if (low !== undefined) {
      dsp.writeReg(0x4400 + index * 8 + 0, low);
    }
    if (high !== undefined) {
      dsp.writeReg(0x4400 + index * 8 + 4, high);
    }
  });

  // Write MPRO registers
  for (let i = 0; i < 128; i++) {
    for (let j = 0; j < 4; j++) {
      dsp.writeReg(0x3000 + 0x400 + i * 4 * 4 + j * 4, 0);
    }
  }
  parsedData.MPRO.forEach(({ index, encoded }) => {
    // Write each part of the encoded instruction
    encoded.forEach((value, offset) => {
      dsp.writeReg(0x3000 + 0x400 + index * 4 * 4 + offset * 4, value);
    });
  });
};
