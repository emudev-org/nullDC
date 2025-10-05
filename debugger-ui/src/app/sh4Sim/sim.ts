/* eslint-disable @typescript-eslint/ban-ts-comment */
// @ts-nocheck

const Flags = {
    L: 16, // Lock
    LP: 32, // Lock Partial (no-compute)
    R: 64, // Result
    KShift: 7,
    Kick(x) { return (x << this.KShift); }
}

const Stage = {
    I: 0,
    D: 1,
    EX: 3,
    SX: 4,
    F0: 5,
    F1: 6,
    F2: 7,
    F3: 8,
    NA: 9,
    MA: 10,
    S: 11,
    FS: 12,
    f1: 13,
    d: 14,
    Count: 15,
    Mask: 15
};

const StageNames = { }
StageNames[Stage.I] = "I";
StageNames[Stage.D] = "D";
StageNames[Stage.d] = "d";
StageNames[Stage.EX] = "EX";
StageNames[Stage.SX] = "SX";
StageNames[Stage.F0] = "F0";
StageNames[Stage.F1] = "F1";
StageNames[Stage.f1] = "f1";
StageNames[Stage.F2] = "F2";
StageNames[Stage.F3] = "F3";
StageNames[Stage.NA] = "NA";
StageNames[Stage.MA] = "MA";
StageNames[Stage.S] = "S";
StageNames[Stage.FS] = "FS";

const Group = {
    MT: "MT",
    EX: "EX",
    BR: "BR",
    LS: "LS",
    FE: "FE",
    CO: "CO"
};

function isParallel(group1, group2) {
    if (group1 == Group.MT && group2 == Group.MT)
        return true;
    else if (group1 == Group.CO || group2 == Group.CO)
        return false;
    else
        return group1 != group2;
}

//fdiv: F3 2 10 F1 11 1
//fsqrt: F3 2 9 F1 10 1
function pattern_37(f3_locks) {
    if (f3_locks == 10) {
        return [
            [Stage.I, Stage.D, Stage.F1 | Flags.Kick(1), Stage.F2, Stage.FS],
            [Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L | Flags.Kick(2)],
            [Stage.F1 | Flags.L, Stage.F2, Stage.FS | Flags.R]
        ];
    } else if (f3_locks == 9) {
        return [
            [Stage.I, Stage.D, Stage.F1 | Flags.Kick(1), Stage.F2, Stage.FS],
            [Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L | Flags.Kick(2)],
            [Stage.F1 | Flags.L, Stage.F2, Stage.FS | Flags.R]
        ];
    } else {
        throw new Error(`Unknown f3_locks ${f3_locks}`);
    }
}

//fdiv: F3 2 21 F1 20 3
//fsqrt: F3 2 20 F1 19 3
// Hmm, no FS here?
function pattern_41(f3_locks) {
    if (f3_locks == 21) {
        return [
            [Stage.I, Stage.D, Stage.F1 | Flags.Kick(1), Stage.F2, Stage.FS],
            [Stage.d | Flags.Kick(2), Stage.F1, Stage.F2],
            [Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L | Flags.Kick(3), Stage.F3 | Flags.L],
            [Stage.F1 | Flags.L, Stage.F2 | Flags.Kick(4), Stage.F3],
            [Stage.F1 | Flags.L, Stage.F2 | Flags.Kick(5), Stage.F3],
            [Stage.F1 | Flags.L, Stage.F2, Stage.F3 | Flags.R],
        ];
    } else if (f3_locks == 20) {
        return [
            [Stage.I, Stage.D, Stage.F1 | Flags.Kick(1), Stage.F2, Stage.FS],
            [Stage.d | Flags.Kick(2), Stage.F1, Stage.F2],
            [Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L, Stage.F3 | Flags.L | Flags.Kick(3), Stage.F3 | Flags.L],
            [Stage.F1 | Flags.L, Stage.F2 | Flags.Kick(4), Stage.F3],
            [Stage.F1 | Flags.L, Stage.F2 | Flags.Kick(5), Stage.F3],
            [Stage.F1 | Flags.L, Stage.F2, Stage.F3 | Flags.R],
        ];
    } else {
        throw new Error(`Unknown f3_locks ${f3_locks}`);
    }
}

const Patterns = {
    // 1-step operation: 1 issue cycle
    // EXT[SU].[BW], MOV, MOV#, MOVA, MOVT, SWAP.[BW], XTRCT, ADD*, CMP*, 
    // DIV*, DT, NEG*, SUB*, AND, AND#, NOT, OR, OR#, TST, TST#, XOR, XOR#, 
    // ROT*, SHA*, SHL*, BF*, BT*, BRA, NOP, CLRS, CLRT, SETS, SETT, 
    // LDS to FPUL, STS from FPUL/FPSCR, FLDI0, FLDI1, FMOV, FLDS, FSTS, 
    // single-/double-precision FABS/FNEG
    1: [
        [Stage.I, Stage.D, Stage.EX, Stage.NA, Stage.S]
    ],

    // Load/store: 1 issue cycle
    // MOV.[BWL]. FMOV*@, LDS.L to FPUL, LDTLB, PREF, STS.L from FPUL/FPSCR
    2: [
        [Stage.I, Stage.D, Stage.EX, Stage.MA, Stage.S]
    ],

    // GBR-based load/store: 1 issue cycle
    // MOV.[BWL]@(d,GBR)
    3: [
        [Stage.I, Stage.D, Stage.SX, Stage.MA, Stage.S]
    ],

    // JMP, RTS, BRAF: 2 issue cycles
    4: [
        [Stage.I, Stage.D | Flags.L, Stage.EX | Flags.Kick(1), Stage.NA, Stage.S | Flags.R],
        [Stage.D | Flags.L, Stage.EX, Stage.NA, Stage.S]
    ],

    // TST.B: 3 issue cycles
    5: [
        [Stage.I, Stage.D | Flags.L, Stage.SX | Flags.Kick(1), Stage.MA, Stage.S],
        [Stage.D | Flags.L, Stage.SX | Flags.Kick(2), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.SX, Stage.MA, Stage.S]
    ],

    // AND.B, OR.B, XOR.B: 4 issue cycles
    6: [
        [Stage.I, Stage.D | Flags.L, Stage.SX | Flags.Kick(1), Stage.MA, Stage.S],
        [Stage.D | Flags.L, Stage.SX | Flags.Kick(2), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.SX | Flags.Kick(3), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.SX, Stage.MA, Stage.S],
    ],

    // TAS.B: 5 issue cycles
    7: [
        [Stage.I, Stage.D | Flags.L, Stage.EX | Flags.Kick(1), Stage.MA, Stage.S | Flags.R],
        [Stage.D | Flags.L, Stage.EX | Flags.Kick(2), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.EX | Flags.Kick(3), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.EX | Flags.Kick(4), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.EX, Stage.MA, Stage.S],
    ],

    8: [
        [ Stage.I, Stage.D, Stage.EX | Flags.Kick(1), Stage.NA, Stage.S ],
        [ Stage.D, Stage.EX | Flags.Kick(1), Stage.NA, Stage.S ],
        [ Stage.D, Stage.EX | Flags.Kick(2), Stage.NA, Stage.S ],
        [ Stage.D, Stage.EX | Flags.Kick(3), Stage.NA, Stage.S ],
        [ Stage.D, Stage.EX, Stage.NA, Stage.S ]
    ],

    9: [
        [ Stage.I, Stage.D, Stage.EX | Flags.Kick(1), Stage.NA, Stage.S ],
        [ Stage.D, Stage.EX | Flags.Kick(1), Stage.NA, Stage.S ],
        [ Stage.D, Stage.EX | Flags.Kick(2), Stage.NA, Stage.S ],
        [ Stage.D, Stage.EX, Stage.NA, Stage.S ]
    ],

    // 10. OCBI: 1 issue cycle
    10: [
        
        [Stage.I, Stage.D, Stage.EX, Stage.MA, Stage.S | Flags.Kick(1)],
        [Stage.MA | Flags.LP]
    ],
    // 11. OCBP, OCBWB: 1 issue cycle
    11: [
        [Stage.I, Stage.D, Stage.EX, Stage.MA, Stage.S | Flags.Kick(1)],
        [Stage.MA | Flags.LP, Stage.MA | Flags.LP, Stage.MA | Flags.LP, Stage.MA | Flags.LP],
    ],

    // 12. MOVCA.L: 1 issue cycle
    12: [
        [Stage.I, Stage.D, Stage.EX, Stage.MA, Stage.S | Flags.Kick(1)],
        [Stage.MA | Flags.LP, Stage.MA | Flags.LP, Stage.MA | Flags.LP, Stage.MA | Flags.LP, Stage.MA | Flags.LP, Stage.MA | Flags.LP],
    ],

    // 13. TRAPA: 7 issue cycles
    13: [
        [Stage.I, Stage.D | Flags.Kick(1) | Flags.L, Stage.EX, Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.EX | Flags.Kick(2), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.EX | Flags.Kick(3), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.EX | Flags.Kick(4), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.EX | Flags.Kick(5), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.EX | Flags.Kick(6), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.EX, Stage.NA, Stage.S]
    ],

    // Single stages that are in waterfall have been merged to a single sequence

    // 14. CR definition: 1 issue cycle: LDC to DBR/Rp_BANK/SSR/SPC/VBR, BSR
    14: [
        [Stage.I, Stage.D, Stage.EX, Stage.NA | Flags.Kick(1), Stage.S | Flags.R],
        [Stage.SX | Flags.LP, Stage.SX | Flags.LP]
    ],

    // 15. LDC to GBR: 3 issue cycles
    15: [
        [Stage.I, Stage.D | Flags.L, Stage.EX | Flags.Kick(1), Stage.NA, Stage.S | Flags.R],
        [Stage.D | Flags.LP, Stage.SX | Flags.Kick(2) | Flags.LP],
        [Stage.D | Flags.LP, Stage.SX | Flags.LP],
    ],

    // 16. LDC to SR: 4 issue cycles
    16: [
        [Stage.I, Stage.D | Flags.Kick(1) | Flags.L, Stage.EX, Stage.NA, Stage.S | Flags.R],
        [Stage.D | Flags.LP, Stage.SX | Flags.Kick(2) | Flags.LP],
        [Stage.D | Flags.LP, Stage.SX | Flags.Kick(3) | Flags.LP],
        [Stage.D | Flags.LP, Stage.SX | Flags.LP],
    ],

    // 17. LDC.L to DBR/Rp_BANK/SSR/SPC/VBR: 1 issue cycle
    17: [
        [Stage.I, Stage.D, Stage.EX, Stage.MA | Flags.Kick(1), Stage.S | Flags.R],
        [Stage.SX | Flags.LP, Stage.SX | Flags.LP]
    ],

    // 18. LDC.L to GBR: 3 issue cycles
    18: [
        [Stage.I, Stage.D | Flags.Kick(1) | Flags.L, Stage.EX, Stage.MA, Stage.S | Flags.R],
        [Stage.D | Flags.LP, Stage.SX | Flags.Kick(2) | Flags.LP],
        [Stage.D | Flags.LP, Stage.SX | Flags.LP],
    ],

    // 19. LDC.L to SR: 4 issue cycles
    19: [
        [Stage.I, Stage.D | Flags.Kick(1) | Flags.L, Stage.EX, Stage.MA, Stage.S | Flags.R],
        [Stage.D | Flags.LP, Stage.SX | Flags.Kick(2) | Flags.LP],
        [Stage.D | Flags.LP, Stage.SX | Flags.Kick(3) | Flags.LP],
        [Stage.D | Flags.LP, Stage.SX | Flags.LP],
    ],

    // 20. STC from DBR/GBR/Rp_BANK/SR/SSR/SPC/VBR: 2 issue cycles
    20: [
        [Stage.I, Stage.D | Flags.L, Stage.SX | Flags.Kick(1), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.SX, Stage.NA, Stage.S | Flags.R],
    ],

    // 21. STC.L from SGR: 3 issue cycles
    // Shouldn't there be an MA stage here? -> This looks like plain STC, not STC.L
    21: [
        [Stage.I, Stage.D | Flags.L, Stage.SX | Flags.Kick(1), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.SX | Flags.Kick(2), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.SX, Stage.NA, Stage.S | Flags.R],
    ],

    // 22. STC.L from DBR/GBR/Rp_BANK/SR/SSR/SPC/VBR: 2 issue cycles
    22: [
        [Stage.I, Stage.D | Flags.L, Stage.SX | Flags.Kick(1) | Flags.R, Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.SX, Stage.MA, Stage.S],
    ],

    // 23. STC.L from SGR: 3 issue cycles
    23: [
        [Stage.I, Stage.D | Flags.L, Stage.SX | Flags.Kick(1) | Flags.R, Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.SX | Flags.Kick(2), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.SX, Stage.MA, Stage.S],
    ],
    
    // 24. LDS to PR, JSR, BSRF: 2 issue cycles
    24: [
        [Stage.I, Stage.D, Stage.EX | Flags.Kick(1), Stage.NA, Stage.S | Flags.R],
        [Stage.D | Flags.LP, Stage.SX | Flags.LP, Stage.SX | Flags.LP]
    ],

    // 25. LDS.L to PR: 2 issue cycles
    25: [
        [Stage.I, Stage.D | Flags.L, Stage.EX | Flags.Kick(1), Stage.MA, Stage.S | Flags.R],
        [Stage.D | Flags.LP, Stage.SX | Flags.LP, Stage.SX | Flags.LP]
    ],

    // 26. STS from PR: 2 issue cycles
    26: [
        [Stage.I, Stage.D | Flags.L, Stage.SX | Flags.Kick(1), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.SX, Stage.NA, Stage.S | Flags.R],
    ],

    // 27. STS.L from PR: 2 issue cycles
    27: [
        [Stage.I, Stage.D | Flags.L, Stage.SX | Flags.Kick(1), Stage.NA, Stage.S],
        [Stage.D | Flags.L, Stage.SX, Stage.MA, Stage.S | Flags.R],
    ],

    // 28. MACH/L definition: 1 issue cycle: CLRMAC, LDS to MACH/L
    28: [
        [Stage.I, Stage.D, Stage.EX, Stage.NA | Flags.Kick(1), Stage.S],
        [Stage.F1 | Flags.LP, Stage.F1 | Flags.L, Stage.F2, Stage.FS | Flags.R]
    ],
    // 29. LDS.L to MACH/L: 1 issue cycle
    29: [
        [Stage.I, Stage.D, Stage.EX, Stage.MA | Flags.Kick(1), Stage.S],
        [Stage.F1 | Flags.LP, Stage.F1, Stage.F2, Stage.FS | Flags.R]
    ],

    // 30. STS from MACH/L: 1 issue cycle
    30: [
        [Stage.I, Stage.D, Stage.EX, Stage.NA, Stage.S],
    ],

    // 31. STS.L from MACH/L: 1 issue cycle
    31: [
        [Stage.I, Stage.D, Stage.EX, Stage.MA, Stage.S],
    ],

    // 32. LDS to FPSCR: 1 issue cycle
    32: [
        [Stage.I, Stage.D, Stage.EX, Stage.NA | Flags.Kick(1), Stage.S | Flags.R],
        [Stage.F1 | Flags.LP,Stage.F1 | Flags.LP,Stage.F1 | Flags.LP]
    ],

    // 33. LDS.L to FPSCR: 1 issue cycle
    33: [
        [Stage.I, Stage.D, Stage.EX, Stage.MA | Flags.Kick(1), Stage.S | Flags.R],
        [Stage.F1 | Flags.LP,Stage.F1 | Flags.LP,Stage.F1 | Flags.LP]
    ],

    // 34. Fixed-point multiplication: 2 issue cycles: DMULS.L, DMULU.L, MUL.L, MULS.W, MULU.W
    // TODO: Implement f1 semantics
    34: [
        [Stage.I, Stage.D, Stage.EX | Flags.Kick(1), Stage.NA, Stage.S],
        [Stage.D | Flags.L | Flags.Kick(2), Stage.EX | Flags.Kick(3), Stage.NA | Flags.Kick(4), Stage.S | Flags.Kick(5)],
        [Stage.f1],
        [Stage.f1],
        [Stage.f1],
        [Stage.f1, Stage.F2, Stage.FS | Flags.R],
    ],

    // 35. MAC.W, MAC.L: 2 issue cycles
    35: [
        [Stage.I, Stage.D | Flags.L, Stage.EX | Flags.Kick(1), Stage.MA, Stage.S],
        [Stage.D | Flags.L | Flags.Kick(2), Stage.EX | Flags.Kick(3), Stage.MA | Flags.Kick(4), Stage.S | Flags.Kick(5)],
        [Stage.f1],
        [Stage.f1],
        [Stage.f1],
        [Stage.f1, Stage.F2, Stage.FS | Flags.R],
    ],

    // Single-precision floating-point computation: 1 issue cycle
    // FCMP/EQ,FCMP/GT, FADD,FLOAT,FMAC,FMUL,FSUB,FTRC,FRCHG,FSCHG
    36: [
        [Stage.I, Stage.D, Stage.F1, Stage.F2, Stage.FS]
    ],

    // 37 is a special case

    // 38. Double-precision floating-point computation 1: 1 issue cycle: FCNVDS, FCNVSD, FLOAT, FTRC
    38: [
        [Stage.I, Stage.D, Stage.F1 | Flags.Kick(1) | Flags.L, Stage.F2, Stage.FS],
        [Stage.d, Stage.F1 | Flags.L, Stage.F2, Stage.FS | Flags.R]
    ],

    // 39. Double-precision floating-point computation 2: 1 issue cycle: FADD, FMUL, FSUB
    39: [
        [Stage.I, Stage.D, Stage.F1 | Flags.Kick(1) | Flags.L, Stage.F2, Stage.FS],
        [Stage.d, Stage.F1 | Flags.Kick(2) | Flags.L, Stage.F2, Stage.FS],
        [Stage.d, Stage.F1 | Flags.Kick(3) | Flags.L, Stage.F2, Stage.FS],
        [Stage.d, Stage.F1 | Flags.Kick(4) | Flags.L, Stage.F2, Stage.FS],
        [Stage.d, Stage.F1 | Flags.L, Stage.F2 | Flags.Kick(5), Stage.FS],
        [Stage.F1 | Flags.L, Stage.F2, Stage.FS | Flags.R]
    ],

    // 40. Double-precision FCMP: 2 issue cycles: FCMP/EQ,FCMP/GT
    40: [
        [Stage.I, Stage.D | Flags.L, Stage.F1 | Flags.Kick(1) | Flags.L, Stage.F2, Stage.FS],
        [Stage.D | Flags.L, Stage.F1 | Flags.L, Stage.F2, Stage.FS | Flags.R],
    ],
    // 41. Double-precision FDIV/SQRT: 1 issue cycle: FDIV, FSQRT
    // This is special

    // 42.  FIPR: 1 issue cycle
    42: [
        [Stage.I, Stage.D, Stage.F0, Stage.F1, Stage.F2, Stage.FS]
    ],

    // 43.  FTRV: 1 issue cycle
    43: [
        [Stage.I, Stage.D, Stage.F0 | Flags.Kick(1), Stage.F1, Stage.F2, Stage.FS],
        [Stage.d, Stage.F0 | Flags.Kick(2), Stage.F1, Stage.F2, Stage.FS],
        [Stage.d, Stage.F0 | Flags.Kick(3), Stage.F1, Stage.F2, Stage.FS],
        [Stage.d, Stage.F0, Stage.F1, Stage.F2, Stage.FS | Flags.R],
    ],

};

function index_of_part(asm, part) {
    for (let i = 0; i < asm.length; i++) {
        if (asm[i] == part || asm[i] == `@${part}` || asm[i] == `@${part}+` || asm[i] == `@-${part}`)
            return i;
    }
    throw new Error(`Part ${part} not found in ${asm}`);
}
function rm() {
    return [this.variant[index_of_part(this.asm, "Rm")].replace("@", "").replace("+", "")];
}
function rm_bank() {
    return [this.variant[index_of_part(this.asm, "Rm_BANK")]];
}
function rn() {
    return [this.variant[index_of_part(this.asm, "Rn")].replace("@", "").replace("-", "").replace("+", "")];
}
function rn_bank() {
    return [this.variant[index_of_part(this.asm, "Rn_BANK")]];
}

function none() {
    return [];
}

function r0() {
    return ["R0"];
}

function rmn() {
    return [...rm.apply(this), ...rn.apply(this)];
}

function rmnsr() {
    return [...rmn.apply(this), "SR"];
}

function rnsr() {
    return [...rn.apply(this), "SR"];
}

function sr() {
    return ["SR"]
}

function at_r0m() {
    return  ["R0", this.variant[index_of_part(this.asm, "@(R0,Rm)")].match(/@\(R0,([^)]*)\)/)[1]];
}

function at_d4rm() {
    return  [this.variant[index_of_part(this.asm, "@(disp4,Rm)")].match(/@\([0-9]*,([^)]*)\)/)[1]];
}

function at_r0n() {
    return  ["R0", this.variant[index_of_part(this.asm, "@(R0,Rn)")].match(/@\(R0,([^)]*)\)/)[1]];
}

function rm_at_r0n() {
    return [...rm.apply(this), ...at_r0n.apply(this)];
}

function r0_at_d4rn() {
    return ["R0", this.variant[index_of_part(this.asm, "@(disp4,Rn)")].match(/@\([0-9]*,([^)]*)\)/)[1]];
}

function rm_at_d4rn() {
    return [...rm.apply(this), this.variant[index_of_part(this.asm, "@(disp4,Rn)")].match(/@\([0-9]*,([^)]*)\)/)[1]];
}

function fn() {
    return [this.variant[index_of_part(this.asm, "FRn")]];
}

function fm() {
    return [this.variant[index_of_part(this.asm, "FRm")]];
}


function dn() {
    const d_dec = {
        "DR0": ["FR0", "FR1"],
        "DR2": ["FR2", "FR3"],
        "DR4": ["FR4", "FR5"],
        "DR6": ["FR6", "FR7"],
        "DR8": ["FR8", "FR9"],
        "DR10": ["FR10", "FR11"],
        "DR12": ["FR12", "FR13"],
        "DR14": ["FR14", "FR15"],
    };
    const rv = d_dec[this.variant[index_of_part(this.asm, "DRn")]];
    if (!rv)
        throw new Error(`Unknown DRn ${this.variant[index_of_part(this.asm, "DRn")]} in ${this.asm}`);
    return rv;
}

function dm() {
    const d_dec = {
        "DR0": ["FR0", "FR1"],
        "DR2": ["FR2", "FR3"],
        "DR4": ["FR4", "FR5"],
        "DR6": ["FR6", "FR7"],
        "DR8": ["FR8", "FR9"],
        "DR10": ["FR10", "FR11"],
        "DR12": ["FR12", "FR13"],
        "DR14": ["FR14", "FR15"],
    };
    const rv = d_dec[this.variant[index_of_part(this.asm, "DRm")]];
    if (!rv)
        throw new Error(`Unknown DRm ${this.variant[index_of_part(this.asm, "DRm")]} in ${this.asm}`);
    return rv;
}

function xdn() {
    const d_dec = {
        "XD0": ["XF0", "XF1"],
        "XD2": ["XF2", "XF3"],
        "XD4": ["XF4", "XF5"],
        "XD6": ["XF6", "XF7"],
        "XD8": ["XF8", "XF9"],
        "XD10": ["XF10", "XF11"],
        "XD12": ["XF12", "XF13"],
        "XD14": ["XF14", "XF15"],
    };
    const rv = d_dec[this.variant[index_of_part(this.asm, "XDn")]];
    if (!rv)
        throw new Error(`Unknown XDn ${this.variant[index_of_part(this.asm, "XDn")]} in ${this.asm}`);
    return rv;
}

function xdm() {
    const d_dec = {
        "XD0": ["XF0", "XF1"],
        "XD2": ["XF2", "XF3"],
        "XD4": ["XF4", "XF5"],
        "XD6": ["XF6", "XF7"],
        "XD8": ["XF8", "XF9"],
        "XD10": ["XF10", "XF11"],
        "XD12": ["XF12", "XF13"],
        "XD14": ["XF14", "XF15"],
    };
    const rv = d_dec[this.variant[index_of_part(this.asm, "XDm")]];
    if (!rv)
        throw new Error(`Unknown XDm ${this.variant[index_of_part(this.asm, "XDm")]} in ${this.asm}`);
    return rv;
}

function fpul() {
    return ["FPUL"];
}


function r0gbr() {
    return ["R0", "GBR"];
}

function fnm() {
    return [this.variant[index_of_part(this.asm, "FRm")], this.variant[index_of_part(this.asm, "FRn")]];
}

function fmrn() {
    return [this.variant[index_of_part(this.asm, "FRm")], ...rn.apply(this)];
}
function fm_at_r0n() {
    return [this.variant[index_of_part(this.asm, "FRm")], "R0", this.variant[index_of_part(this.asm, "@(R0,Rn)")].match(/@\(R0,([^)]*)\)/)[1]];
}

function xmtrx() {
    return ["XF0", "XF1", "XF2", "XF3", "XF4", "XF5", "XF6", "XF7", 
            "XF8", "XF9", "XF10", "XF11", "XF12", "XF13", "XF14", "XF15"];
}

function fvm() {
    const fvdec = {
        "FV0": ["FR0", "FR1", "FR2", "FR3"],
        "FV4": ["FR4", "FR5", "FR6", "FR7"],
        "FV8": ["FR8", "FR9", "FR10", "FR11"],
        "FV12": ["FR12", "FR13", "FR14", "FR15"],
    };
    const rv = fvdec[this.variant[index_of_part(this.asm, "FVm")]];
    if (!rv)
        throw new Error(`Unknown FVm ${this.variant[index_of_part(this.asm, "FVm")]} in ${this.asm}`);
    return rv;
}
function fvn() {
    const fvdec = {
        "FV0": ["FR0", "FR1", "FR2", "FR3"],
        "FV4": ["FR4", "FR5", "FR6", "FR7"],
        "FV8": ["FR8", "FR9", "FR10", "FR11"],
        "FV12": ["FR12", "FR13", "FR14", "FR15"],
    };
    const rv = fvdec[this.variant[index_of_part(this.asm, "FVn")]];
    if (!rv)
        throw new Error(`Unknown FVn ${this.variant[index_of_part(this.asm, "FVn")]} in ${this.asm}`);
    return rv;
}


const Instructions = {
    1: {asm: ["EXTS.B", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rm, writes: rn },
    2: {asm: ["EXTS.W", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rm, writes: rn },
    3: {asm: ["EXTU.B", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rm, writes: rn },
    4: {asm: ["EXTU.W", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rm, writes: rn },
    5: {asm: ["MOV", "Rm","Rn"], group: Group.MT, issue: 1, latency: 0, pattern: Patterns[1], reads: rm, writes: rn },
    6: {asm: ["MOV", "#imm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: none, writes: rn },
    7: {asm: ["MOVA", "@(disp,PC)","R0"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: none, writes: r0 },
    8: {asm: ["MOV.W", "@(disp,PC)","Rn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: none, writes: rn },
    9: {asm: ["MOV.L", "@(disp,PC)","Rn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: none, writes: rn },
    10: {asm: ["MOV.B", "@Rm","Rn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: rm, writes: rn },
    11: {asm: ["MOV.W", "@Rm","Rn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: rm, writes: rn },
    12: {asm: ["MOV.L", "@Rm","Rn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: rm, writes: rn },

    // 13 MOV.B @Rm+,Rn LS 1 1/2 #2 — — —
    13: {asm: ["MOV.B", "@Rm+","Rn"], group: Group.LS, issue: 1, latency: [1, 2], pattern: Patterns[2], reads: rm, writes: rmn },
    // 14 MOV.W @Rm+,Rn LS 1 1/2 #2 — — —
    14: {asm: ["MOV.W", "@Rm+","Rn"], group: Group.LS, issue: 1, latency: [1, 2], pattern: Patterns[2], reads: rm, writes: rmn },
    // 15 MOV.L @Rm+,Rn LS 1 1/2 #2 — — —
    15: {asm: ["MOV.L", "@Rm+","Rn"], group: Group.LS, issue: 1, latency: [1, 2], pattern: Patterns[2], reads: rm, writes: rmn },

    // 16 MOV.B @(disp,Rm),R0 LS 1 2 #2
    16: {asm: ["MOV.B", "@(disp4,Rm)","R0"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: at_d4rm, writes: r0 },
    // 17 MOV.W @(disp,Rm),R0 LS 1 2 #2 — — —
    17: {asm: ["MOV.W", "@(disp4,Rm)","R0"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: at_d4rm, writes: r0 },
    // 18 MOV.L @(disp,Rm),Rn LS 1 2 #2
    18: {asm: ["MOV.L", "@(disp4,Rm)","Rn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: at_d4rm, writes: rn },
    // 19 MOV.B @(R0,Rm),Rn LS 1 2 #2 — — —
    19: {asm: ["MOV.B", "@(R0,Rm)","Rn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: at_r0m, writes: rn },
    // 20 MOV.W @(R0,Rm),Rn LS 1 2 #2 — — —
    20: {asm: ["MOV.W", "@(R0,Rm)","Rn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: at_r0m, writes: rn },
    // 21 MOV.L @(R0,Rm),Rn LS 1 2 #2
    21: {asm: ["MOV.L", "@(R0,Rm)","Rn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: at_r0m, writes: rn },
    // 22 MOV.B @(disp,GBR),R0 LS 1 2 #3 — — —
    // 23 MOV.W @(disp,GBR),R0 LS 1 2 #3 — — —
    // 24 MOV.L @(disp,GBR),R0 LS 1 2 #3 — — —
    // 25 MOV.B Rm,@Rn LS 1 1 #2 — — —
    // 26 MOV.W Rm,@Rn LS 1 1 #2 — — —
    // 27 MOV.L Rm,@Rn LS 1 1 #2 — — —
    // 28 MOV.B Rm,@-Rn LS 1 1/1 #2 — — —
    28: {asm: ["MOV.B", "Rm","@-Rn"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: rmn, writes: rn },
    // 29 MOV.W Rm,@-Rn LS 1 1/1 #2 — — —
    29: {asm: ["MOV.W", "Rm","@-Rn"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: rmn, writes: rn },
    // 30 MOV.L Rm,@-Rn LS 1 1/1 #2 — — —
    30: {asm: ["MOV.L", "Rm","@-Rn"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: rmn, writes: rn },
    // 31 MOV.B R0,@(disp,Rn) LS 1 1 #2 — — —
    31: {asm: ["MOV.B", "R0","@(disp4,Rn)"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: r0_at_d4rn, writes: none },
    // 32 MOV.W R0,@(disp,Rn) LS 1 1 #2 — — —
    32: {asm: ["MOV.W", "R0","@(disp4,Rn)"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: r0_at_d4rn, writes: none },
    // 33 MOV.L Rm,@(disp,Rn) LS 1 1 #2
    33: {asm: ["MOV.L", "Rm","@(disp4,Rn)"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: rm_at_d4rn, writes: none },
    // 34 MOV.B Rm,@(R0,Rn) LS 1 1 #2 — — —
    34: {asm: ["MOV.B", "Rm","@(R0,Rn)"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: rm_at_r0n, writes: none },
    // 35 MOV.W Rm,@(R0,Rn) LS 1 1 #2 — — —
    35: {asm: ["MOV.W", "Rm","@(R0,Rn)"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: rm_at_r0n, writes: none },
    // 36 MOV.L Rm,@(R0,Rn) LS 1 1 #2
    36: {asm: ["MOV.L", "Rm","@(R0,Rn)"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: rm_at_r0n, writes: none },
    // 37 MOV.B R0,@(disp,GBR) LS 1 1 #3 — — —
    37: {asm: ["MOV.B", "R0","@(disp,GBR)"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[3], reads: () => ["R0", "GBR"], writes: none },
    // 38 MOV.W R0,@(disp,GBR) LS 1 1 #3 — — —
    38: {asm: ["MOV.W", "R0","@(disp,GBR)"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[3], reads: () => ["R0", "GBR"], writes: none },
    // 39 MOV.L R0,@(disp,GBR) LS 1 1 #3 — — —
    39: {asm: ["MOV.L", "R0","@(disp,GBR)"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[3], reads: () => ["R0", "GBR"], writes: none },
    // 40 MOVCA.L R0,@Rn LS 1 3–7 #12 MA 4 3–7
    // 41 MOVT Rn EX 1 1 #1
    41: {asm: ["MOVT", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: sr, writes: rn },
    // 42 OCBI @Rn LS 1 1–2 #10 MA 4 1–2
    // 43 OCBP @Rn LS 1 1–5 #11 MA 4 1–5
    // 44 OCBWB @Rn LS 1 1–5 #11 MA 4 1–5
    // 45 PREF @Rn LS 1 1 #2 — — —
    45: {asm: ["PREF", "@Rn"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: rn, writes: none },
    // 46 SWAP.B Rm,Rn EX 1 1 #1 — — —
    46: {asm: ["SWAP.B", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rm, writes: rn },
    // 47 SWAP.W Rm,Rn EX 1 1 #1 — — —
    47: {asm: ["SWAP.W", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rm, writes: rn },
    // 48 XTRCT Rm,Rn EX 1 1 #1 — — —
    48: {asm: ["XTRCT", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: rn },

    // Fixed point arithmetic instructions
    49: {asm: ["ADD", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: rn },
    50: {asm: ["ADD", "#imm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: none, writes: rn },
    51: {asm: ["ADDC", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rmnsr, writes: rn },
    52: {asm: ["ADDV", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rmnsr, writes: rn },
    53: {asm: ["CMP/EQ", "#imm","R0"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: r0, writes: sr },
    54: {asm: ["CMP/EQ", "Rm","Rn"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: sr },
    55: {asm: ["CMP/GE", "Rm","Rn"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: sr },
    56: {asm: ["CMP/GT", "Rm","Rn"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: sr },
    57: {asm: ["CMP/HI", "Rm","Rn"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: sr },
    58: {asm: ["CMP/HS", "Rm","Rn"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: sr },
    59: {asm: ["CMP/PL", "Rn"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: sr },
    60: {asm: ["CMP/PZ", "Rn"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: sr },
    61: {asm: ["CMP/STR", "Rm","Rn"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: sr },
    62: {asm: ["DIV0S", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: sr },
    63: {asm: ["DIV0U"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: none, writes: sr },
    64: {asm: ["DIV1", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rm, writes: rnsr },
    // 65 DMULS.L Rm,Rn CO 2 4/4 #34 F1 4 2
    65: {asm: ["DMULS.L", "Rm","Rn"], group: Group.CO, issue: 2, latency: 4, pattern: Patterns[34], reads: [rm, rn], writes: ["MACH", "MACL"] },
    // 66 DMULU.L Rm,Rn CO 2 4/4 #34 F1 4 2
    66: {asm: ["DMULU.L", "Rm","Rn"], group: Group.CO, issue: 2, latency: 4, pattern: Patterns[34], reads: [rm, rn], writes: ["MACH", "MACL"] },
    67: {asm: ["DT", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: rnsr },
    // 68 MAC.L @Rm+,@Rn+ CO 2 2/2/4/4 #35 F1 4 2
    68: {asm: ["MAC.L", "@Rm+","@Rn+"], group: Group.CO, issue: 2, latency: [2, 2, 4, 4], pattern: Patterns[35], reads: [rm, rn], writes: [rm, rn, "MACH", "MACL"] },
    // 69 MAC.W @Rm+,@Rn+ CO 2 2/2/4/4 #35 F1 4 2
    69: {asm: ["MAC.W", "@Rm+","@Rn+"], group: Group.CO, issue: 2, latency: [2, 2, 4, 4], pattern: Patterns[35], reads: [rm, rn], writes: [rm, rn, "MACH", "MACL"] },
    // 70 MUL.L Rm,Rn CO 2 4/4 #34 F1 4 2
    70: {asm: ["MUL.L", "Rm","Rn"], group: Group.CO, issue: 2, latency: 4, pattern: Patterns[34], reads: [rm, rn], writes: ["MACL"] },
    // 71 MULS.W Rm,Rn CO 2 4/4 #34 F1 4 2
    71: {asm: ["MULS.W", "Rm","Rn"], group: Group.CO, issue: 2, latency: 4, pattern: Patterns[34], reads: [rm, rn], writes: ["MACL"] },
    // 72 MULU.W Rm,Rn CO 2 4/4 #34 F1 4 2
    72: {asm: ["MULU.W", "Rm","Rn"], group: Group.CO, issue: 2, latency: 4, pattern: Patterns[34], reads: [rm, rn], writes: ["MACL"] },
    73: {asm: ["NEG", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rm, writes: rn },
    // 74 NEGC Rm,Rn EX 1 1 #1 — — —
    74: {asm: ["NEGC", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: [rm, "SR"], writes: [rn, "SR"] },
    75: {asm: ["SUB", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: rn },
    // 76 SUBC Rm,Rn EX 1 1 #1 — — —
    76: {asm: ["SUBC", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: [rm, rn, "SR"], writes: [rn, "SR"] },
    // 77 SUBV Rm,Rn EX 1 1 #1 — — —
    77: {asm: ["SUBV", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: [rm, rn], writes: [rn, "SR"] },

    // Logical Instructions
    78: {asm: ["AND", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: rn  },
    79: {asm: ["AND", "#imm","R0"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: none, writes: r0 },
    80: {asm: ["AND.B", "#imm","@(R0,GBR)"], group: Group.CO, issue: 4, latency: -1, pattern: Patterns[6], reads: r0gbr, writes: none },

    81: {asm: ["NOT", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rm, writes: rn  },
    82: {asm: ["OR", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: rn  },
    83: {asm: ["OR", "#imm","R0"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: r0, writes: r0  },
    // 84 OR.B #imm,@(R0,GBR) CO 4 4 #6 — — —
    84: {asm: ["OR.B", "#imm","@(R0,GBR)"], group: Group.CO, issue: 4, latency: -1, pattern: Patterns[6], reads: r0gbr, writes: none },
    // 85 TAS.B @Rn CO 5 5 #7 — — —
    85: {asm: ["TAS", "@Rn"], group: Group.CO, issue: 5, latency: 5, pattern: Patterns[7], reads: rn, writes: sr  },
    86: {asm: ["TST", "Rm","Rn"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: sr  },
    87: {asm: ["TST", "#imm","R0"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: r0, writes: sr  },
    // 88 TST.B #imm,@(R0,GBR) CO 3 3 #5 — — —
    89: {asm: ["XOR", "Rm","Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: rn  },
    90: {asm: ["XOR", "#imm","R0"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: r0, writes: r0  },
    // 91 XOR.B #imm,@(R0,GBR) CO 4 4 #6 — — —
    91: {asm: ["XOR.B", "#imm","@(R0,GBR)"], group: Group.CO, issue: 4, latency: -1, pattern: Patterns[6], reads: r0gbr, writes: none },
    92: {asm: ["ROTL", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rnsr, writes: rnsr },
    93: {asm: ["ROTR", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rnsr, writes: rnsr },
    94: {asm: ["ROTCL", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rnsr, writes: rnsr },
    95: {asm: ["ROTCR", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rnsr, writes: rnsr },
    96: {asm: ["SHAD", "Rm", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: rn },
    97: {asm: ["SHAL", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: rnsr },
    98: {asm: ["SHAR", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: rnsr },
    99: {asm: ["SHLD", "Rm", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rmn, writes: rn },
    100: {asm: ["SHLL", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: rnsr },
    101: {asm: ["SHLL2", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: rn },
    102: {asm: ["SHLL8", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: rn },
    103: {asm: ["SHLL16", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: rn },
    104: {asm: ["SHLR", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: rnsr },
    105: {asm: ["SHLR2", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: rn },
    106: {asm: ["SHLR8", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: rn },
    107: {asm: ["SHLR16", "Rn"], group: Group.EX, issue: 1, latency: 1, pattern: Patterns[1], reads: rn, writes: rn },

    // Branch Instructions

    // 108 BF disp BR 1 2 (or 1) #1
    108: {asm: ["BF"/*, "disp8"*/], group: Group.BR, issue: 1, latency: 2, pattern: Patterns[1], reads: none, writes: none },

    // 109 BF/S disp BR 1 2 (or 1) #1 — — —
    109: {asm: ["BF/S"/*, "disp8"*/], group: Group.BR, issue: 1, latency: 2, pattern: Patterns[1], reads: none, writes: none },
    // 110 BT disp BR 1 2 (or 1) #1 — — —
    110: {asm: ["BT"/*, "disp8"*/], group: Group.BR, issue: 1, latency: 2, pattern: Patterns[1], reads: none, writes: none },
    // 111 BT/S disp BR 1 2 (or 1) #1 — — —
    111: {asm: ["BT/S"/*, "disp8"*/], group: Group.BR, issue: 1, latency: 2, pattern: Patterns[1], reads: none, writes: none },

    // 112 BRA disp BR 1 2 #1
    112: {asm: ["BRA"/*, "disp12"*/], group: Group.BR, issue: 1, latency: 2, pattern: Patterns[1], reads: none, writes: none },

    // 113 BRAF Rn CO 2 3 #4 — — —
    113: {asm: ["BRAF", "Rn"], group: Group.CO, issue: 2, latency: 3, pattern: Patterns[4], reads: rn, writes: none },
    // 114 BSR disp BR 1 2 #14 SX 3 2
    // TODO: Also support label here
    114: {asm: ["BSR"/*, "disp12"*/], group: Group.BR, issue: 1, latency: 2, pattern: Patterns[14], reads: none, writes: none },
    // 115 BSRF Rn CO 2 3 #24 SX 3 2
    115: {asm: ["BSRF", "Rn"], group: Group.CO, issue: 2, latency: 3, pattern: Patterns[24], reads: rn, writes: none },
    // 116 JMP @Rn CO 2 3 #4 — — —
    116: {asm: ["JMP", "@Rn"], group: Group.CO, issue: 2, latency: 3, pattern: Patterns[4], reads: rn, writes: none },
    // 117 JSR @Rn CO 2 3 #24 SX 3 2
    117: {asm: ["JSR", "@Rn"], group: Group.CO, issue: 2, latency: 3, pattern: Patterns[24], reads: rn, writes: none },

    // 118 RTS CO 2 3 #4 — — —
    118: {asm: ["RTS"], group: Group.CO, issue: 2, latency: 3, pattern: Patterns[4], reads: none, writes: none },

    // System Control Instructions
    119: {asm: ["NOP"], group: Group.MT, issue: 1, latency: 0, pattern: Patterns[1], reads: none, writes: none },
    // 120 CLRMAC CO 1 3 #28 F1 3 2
    121: {asm: ["CLRS"], group: Group.CO, issue: 1, latency: 1, pattern: Patterns[1], reads: none, writes: sr },
    122: {asm: ["CLRT"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: none, writes: sr },
    123: {asm: ["SETS"], group: Group.CO, issue: 1, latency: 1, pattern: Patterns[1], reads: none, writes: sr },
    124: {asm: ["SETT"], group: Group.MT, issue: 1, latency: 1, pattern: Patterns[1], reads: none, writes: sr },
    // 125 TRAPA #imm CO 7 7 #13 — — —
    125: {asm: ["TRAPA", "#imm"], group: Group.CO, issue: 7, latency: -1, pattern: Patterns[13], reads: none, writes: none },
    // 126 RTE CO 5 5 #8 — — —
    126: {asm: ["RTE"], group: Group.CO, issue: 5, latency: -1, pattern: Patterns[8], reads: none, writes: none },
    // 127 SLEEP CO 4 4 #9 — — —
    127: {asm: ["SLEEP"], group: Group.CO, issue: 4, latency: -1, pattern: Patterns[9], reads: none, writes: none },
    // 128 LDTLB CO 1 1 #2 — — —
    128: {asm: ["LDTLB"], group: Group.CO, issue: 1, latency: -1, pattern: Patterns[2], reads: none, writes: none },
    // 129 LDC Rm,DBR CO 1 3 #14 SX 3 2
    129: {asm: ["LDC", "Rm","DBR"], group: Group.CO, issue: 1, latency: 3, pattern: Patterns[14], reads: rm, writes: "DBR" },
    // 130 LDC Rm,GBR CO 3 3 #15 SX 3 2
    130: {asm: ["LDC", "Rm","GBR"], group: Group.CO, issue: 3, latency: 3, pattern: Patterns[15], reads: rm, writes: "GBR" },
    // 131 LDC Rm,Rn_BANK CO 1 3 #14 SX 3 2
    130: {asm: ["LDC", "Rm","Rn_BANK"], group: Group.CO, issue: 1, latency: 3, pattern: Patterns[14], reads: rm, writes: rn_bank },
    // 132 LDC Rm,SR CO 4 4 #16 SX 3 2
    132: {asm: ["LDC", "Rm","SR"], group: Group.CO, issue: 4, latency: 4, pattern: Patterns[16], reads: rm, writes: "SR" },
    // 133 LDC Rm,SSR CO 1 3 #14 SX 3 2
    133: {asm: ["LDC", "Rm","SSR"], group: Group.CO, issue: 1, latency: 3, pattern: Patterns[14], reads: rm, writes: "SSR" },
    // 134 LDC Rm,SPC CO 1 3 #14 SX 3 2
    134: {asm: ["LDC", "Rm","SPC"], group: Group.CO, issue: 1, latency: 3, pattern: Patterns[14], reads: rm, writes: "SPC" },
    // 135 LDC Rm,VBR CO 1 3 #14 SX 3 2
    135: {asm: ["LDC", "Rm","VBR"], group: Group.CO, issue: 1, latency: 3, pattern: Patterns[14], reads: rm, writes: "VBR" },
    // 136 LDC.L @Rm+,DBR CO 1 1/3 #17 SX 3 2
    136: {asm: ["LDC.L", "@Rm+","DBR"], group: Group.CO, issue: 1, latency: [1, 3], pattern: Patterns[17], reads: rm, writes: [rm, "DBR"] },
    // 137 LDC.L @Rm+,GBR CO 3 3/3 #18 SX 3 2
    137: {asm: ["LDC.L", "@Rm+","GBR"], group: Group.CO, issue: 3, latency: [3, 3], pattern: Patterns[18], reads: rm, writes: [rm, "GBR"] },
    // 138 LDC.L @Rm+,Rn_BANK CO 1 1/3 #17 SX 3 2
    137: {asm: ["LDC.L", "@Rm+","Rn_BANK"], group: Group.CO, issue: 1, latency: [1, 3], pattern: Patterns[17], reads: rm, writes: [rm, rn_bank] },
    // 139 LDC.L @Rm+,SR CO 4 4/4 #19 SX 3 2
    139: {asm: ["LDC.L", "@Rm+","SR"], group: Group.CO, issue: 4, latency: [4, 4], pattern: Patterns[19], reads: rm, writes: [rm, "SR"] },
    // 140 LDC.L @Rm+,SSR CO 1 1/3 #17 SX 3 2
    140: {asm: ["LDC.L", "@Rm+","SSR"], group: Group.CO, issue: 1, latency: [1, 3], pattern: Patterns[17], reads: rm, writes: [rm, "SSR"] },
    // 141 LDC.L @Rm+,SPC CO 1 1/3 #17 SX 3 2
    141: {asm: ["LDC.L", "@Rm+","SPC"], group: Group.CO, issue: 1, latency: [1, 3], pattern: Patterns[17], reads: rm, writes: [rm, "SPC"] },
    // 142 LDC.L @Rm+,VBR CO 1 1/3 #17 SX 3 2
    142: {asm: ["LDC.L", "@Rm+","VBR"], group: Group.CO, issue: 1, latency: [1, 3], pattern: Patterns[17], reads: rm, writes: [rm, "VBR"] },
    // 143 LDS Rm,MACH CO 1 3 #28 F1 3 2
    143: {asm: ["LDS", "Rm","MACH"], group: Group.CO, issue: 1, latency: 3, pattern: Patterns[28], reads: rm, writes: "MACH" },
    // 144 LDS Rm,MACL CO 1 3 #28 F1 3 2
    144: {asm: ["LDS", "Rm","MACL"], group: Group.CO, issue: 1, latency: 3, pattern: Patterns[28], reads: rm, writes: "MACL" },
    // 145 LDS Rm,PR CO 2 3 #24 SX 3 2
    145: {asm: ["LDS", "Rm","PR"], group: Group.CO, issue: 2, latency: 3, pattern: Patterns[24], reads: rm, writes: "PR" },
    // 146 LDS.L @Rm+,MACH CO 1 1/3 #29 F1 3 2
    146: {asm: ["LDS.L", "@Rm+","MACH"], group: Group.CO, issue: 1, latency: [1, 3], pattern: Patterns[29], reads: rm, writes: [rm, "MACH"] },
    // 147 LDS.L @Rm+,MACL CO 1 1/3 #29 F1 3 2
    147: {asm: ["LDS.L", "@Rm+","MACL"], group: Group.CO, issue: 1, latency: [1, 3], pattern: Patterns[29], reads: rm, writes: [rm, "MACL"] },
    // 148 LDS.L @Rm+,PR CO 2 2/3 #25 SX 3 2
    148: {asm: ["LDS.L", "@Rm+","PR"], group: Group.CO, issue: 2, latency: [2, 3], pattern: Patterns[25], reads: rm, writes: [rm, "PR"] },
    // 149 STC DBR,Rn CO 2 2 #20 — — —
    149: {asm: ["STC", "DBR","Rn"], group: Group.CO, issue: 2, latency: 2, pattern: Patterns[20], reads: "DBR", writes: rn },
    // 150 STC SGR,Rn CO 3 3 #21 — — —
    150: {asm: ["STC", "SGR","Rn"], group: Group.CO, issue: 3, latency: 3, pattern: Patterns[21], reads: "SGR", writes: rn },
    // 151 STC GBR,Rn CO 2 2 #20 — — —
    151: {asm: ["STC", "GBR","Rn"], group: Group.CO, issue: 2, latency: 2, pattern: Patterns[20], reads: "GBR", writes: rn },
    // 152 STC Rm_BANK,Rn CO 2 2 #20 — — —
    152: {asm: ["STC", "Rm_BANK","Rn"], group: Group.CO, issue: 2, latency: 2, pattern: Patterns[20], reads: rm_bank, writes: rn },
    // 153 STC SR,Rn CO 2 2 #20 — — —
    153: {asm: ["STC", "SR","Rn"], group: Group.CO, issue: 2, latency: 2, pattern: Patterns[20], reads: "SR", writes: rn },
    // 154 STC SSR,Rn CO 2 2 #20 — — —
    154: {asm: ["STC", "SSR","Rn"], group: Group.CO, issue: 2, latency: 2, pattern: Patterns[20], reads: "SSR", writes: rn },
    // 155 STC SPC,Rn CO 2 2 #20 — — —
    155: {asm: ["STC", "SPC","Rn"], group: Group.CO, issue: 2, latency: 2, pattern: Patterns[20], reads: "SPC", writes: rn },
    // 156 STC VBR,Rn CO 2 2 #20 — — —
    156: {asm: ["STC", "VBR","Rn"], group: Group.CO, issue: 2, latency: 2, pattern: Patterns[20], reads: "VBR", writes: rn },
    // 157 STC.L DBR,@-Rn CO 2 2/2 #22 — — —
    157: {asm: ["STC.L", "DBR","@-Rn"], group: Group.CO, issue: 2, latency: [2, 2], pattern: Patterns[22], reads: "DBR", writes: rn },
    // 158 STC.L SGR,@-Rn CO 3 3/3 #23 — — —
    158: {asm: ["STC.L", "SGR","@-Rn"], group: Group.CO, issue: 3, latency: [3, 3], pattern: Patterns[23], reads: "SGR", writes: rn },
    // 159 STC.L GBR,@-Rn CO 2 2/2 #22 — — —
    159: {asm: ["STC.L", "GBR","@-Rn"], group: Group.CO, issue: 2, latency: [2, 2], pattern: Patterns[22], reads: "GBR", writes: rn },
    // 160 STC.L Rm_BANK,@-Rn CO 2 2/2 #22 — — —
    160: {asm: ["STC.L", "Rm_BANK","@-Rn"], group: Group.CO, issue: 2, latency: 2, pattern: Patterns[22], reads: [rm_bank, rn], writes: rn },
    // 161 STC.L SR,@-Rn CO 2 2/2 #22 — — —
    161: {asm: ["STC.L", "SR","@-Rn"], group: Group.CO, issue: 2, latency: [2, 2], pattern: Patterns[22], reads: "SR", writes: rn },
    // 162 STC.L SSR,@-Rn CO 2 2/2 #22 — — —
    162: {asm: ["STC.L", "SSR","@-Rn"], group: Group.CO, issue: 2, latency: [2, 2], pattern: Patterns[22], reads: "SSR", writes: rn },
    // 163 STC.L SPC,@-Rn CO 2 2/2 #22 — — —
    163: {asm: ["STC.L", "SPC","@-Rn"], group: Group.CO, issue: 2, latency: [2, 2], pattern: Patterns[22], reads: "SPC", writes: rn },
    // 164 STC.L VBR,@-Rn CO 2 2/2 #22 — — —
    164: {asm: ["STC.L", "VBR","@-Rn"], group: Group.CO, issue: 2, latency: [2, 2], pattern: Patterns[22], reads: "VBR", writes: rn },
    // 165 STS MACH,Rn CO 1 3 #30 — — —
    165: {asm: ["STS", "MACH","Rn"], group: Group.CO, issue: 1, latency: 3, pattern: Patterns[30], reads: "MACH", writes: rn },
    // 166 STS MACL,Rn CO 1 3 #30 — — —
    166: {asm: ["STS", "MACL","Rn"], group: Group.CO, issue: 1, latency: 3, pattern: Patterns[30], reads: "MACL", writes: rn },
    // 167 STS PR,Rn CO 2 2 #26 — — —
    167: {asm: ["STS", "PR","Rn"], group: Group.CO, issue: 2, latency: 2, pattern: Patterns[26], reads: "PR", writes: rn },
    // 168 STS.L MACH,@-Rn CO 1 1/1 #31 — — —
    168: {asm: ["STS.L", "MACH","@-Rn"], group: Group.CO, issue: 1, latency: 1, pattern: Patterns[31], reads: "MACH", writes: rn },
    // 169 STS.L MACL,@-Rn CO 1 1/1 #31 — — —
    169: {asm: ["STS.L", "MACL","@-Rn"], group: Group.CO, issue: 1, latency: 1, pattern: Patterns[31], reads: "MACL", writes: rn },
    // 170 STS.L PR,@-Rn CO 2 2/2 #27 — — —
    170: {asm: ["STS.L", "PR","@-Rn"], group: Group.CO, issue: 2, latency: 2, pattern: Patterns[27], reads: "PR", writes: rn },

    //Single-precision floating-point instructions
    171: {asm: ["FLDI0", "FRn"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: none, writes: fn },
    172: {asm: ["FLDI1", "FRn"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: none, writes: fn },
    173: {asm: ["FMOV", "FRm","FRn"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: fm, writes: fn },
    174: {asm: ["FMOV.S","@Rm","FRn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: rm, writes: fn },
    // 175 FMOV.S @Rm+,FRn LS 1 1/2 #2 — — —
    175: {asm: ["FMOV.S","@Rm+","FRn"], group: Group.LS, issue: 1, latency: [1, 2], pattern: Patterns[2], reads: rm, writes: [rm, fn] },
    176: {asm: ["FMOV.S", "@(R0,Rm)","FRn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: at_r0m, writes: fn },
    177: {asm: ["FMOV.S", "FRm","@Rn"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: fmrn, writes: none },
    178: {asm: ["FMOV.S", "FRm","@-Rn"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: fmrn, writes: rn },
    179: {asm: ["FMOV.S", "FRm","@(R0,Rn)"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: fm_at_r0n, writes: none },
    180: {asm: ["FLDS", "FRm","FPUL"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: fm, writes: fpul },
    181: {asm: ["FSTS", "FPUL","FRn"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: fpul, writes: fn },
    //182 FABS FRn LS 1 0 #1 — — —
    182: {asm: ["FABS", "FRn"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: fn, writes: fn },
    183: {asm: ["FADD", "FRm","FRn"], group: Group.FE, issue: 1, latency: 3 /*3/4*/, pattern: Patterns[36], reads: fnm, writes: fn },

    // 184 FCMP/EQ FRm,FRn FE 1 2/4 #36 — — —
    184: {asm: ["FCMP/EQ", "FRm","FRn"], group: Group.FE, issue: 1, latency: 2 /*2/4*/, pattern: Patterns[36], reads: fnm, writes: sr },
    //185 FCMP/GT FRm,FRn FE 1 2/4 #36 — — —
    185: {asm: ["FCMP/GT", "FRm","FRn"], group: Group.FE, issue: 1, latency: 2 /*2/4*/, pattern: Patterns[36], reads: fnm, writes: sr },
    // 186 FDIV FRm,FRn FE 1 12/13 #37 F3 2 10 F1 11 1
    186: {asm: ["FDIV", "FRm","FRn"], group: Group.FE, issue: 1, latency: [12, 13], pattern: pattern_37(10), reads: fnm, writes: [fn, "FPSCR"]},
    // 187 FLOAT FPUL,FRn FE 1 3/4 #36 F1 2 2
    187: {asm: ["FLOAT", "FPUL","FRn"], group: Group.FE, issue: 1, latency: 3 /*3/4*/, pattern: Patterns[36], reads: fpul, writes: fn },
    // 188 FMAC FR0,FRm,FRn FE 1 3/4 #36 — — —
    188: {asm: ["FMAC", "FR0","FRm","FRn"], group: Group.FE, issue: 1, latency: 3 /*3/4*/, pattern: Patterns[36], reads: [fnm, "FR0"], writes: [fn, "FPSCR"] },
    189: {asm: ["FMUL", "FRm","FRn"], group: Group.FE, issue: 1, latency: 3 /*3/4*/, pattern: Patterns[36], reads: fnm, writes: fn },
    
    // 190 FNEG FRn LS 1 0 #1 — — —
    190: {asm: ["FNEG", "FRn"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: fn, writes: fn },
    // 191 FSQRT FRn FE 1 11/12 #37 F3 2 9 F1 10 1
    191: {asm: ["FSQRT", "FRn"], group: Group.FE, issue: 1, latency: 11 /*11/12*/, pattern: pattern_37(9), reads: fn, writes: fn },

    192: {asm: ["FSUB", "FRm","FRn"], group: Group.FE, issue: 1, latency: 3 /*3/4*/, pattern: Patterns[36], reads: fnm, writes: fn },
    // 193 FTRC FRm,FPUL FE 1 3/4 #36 — — —
    193: {asm: ["FTRC", "FRm","FPUL"], group: Group.FE, issue: 1, latency: 3 /*[3, 4]*/, pattern: Patterns[36], reads: fm, writes: ["FPUL", "FPSCR"] },
    // 194 FMOV DRm,DRn LS 1 0 #1 — — —
    194: {asm: ["FMOV", "DRm","DRn"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: dm, writes: dn },
    // 195 FMOV @Rm,DRn LS 1 2 #2 — — —
    195: {asm: ["FMOV", "@Rm","DRn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: rm, writes: dn },
    // 196 FMOV @Rm+,DRn LS 1 1/2 #2 — — —
    196: {asm: ["FMOV", "@Rm+","DRn"], group: Group.LS, issue: 1, latency: [1, 2], pattern: Patterns[2], reads: rm, writes: [rm, dn] },
    // 197 FMOV @(R0,Rm),DRn LS 1 2 #2 — — —
    197: {asm: ["FMOV", "@(R0,Rm)","DRn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: at_r0m, writes: dn },
    // 198 FMOV DRm,@Rn LS 1 1 #2 — — —
    198: {asm: ["FMOV", "DRm","@Rn"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: [dm, rn], writes: none },
    // 199 FMOV DRm,@-Rn LS 1 1/1 #2 — — —
    199: {asm: ["FMOV", "DRm","@-Rn"], group: Group.LS, issue: 1, latency: [1, 1], pattern: Patterns[2], reads: [dm, rn], writes: rn },
    // 200 FMOV DRm,@(R0,Rn) LS 1 1 #2 — — —
    200: {asm: ["FMOV", "DRm","@(R0,Rn)"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: [dm, at_r0n], writes: none },

    // Double-precision floating-point instructions
    // 201 FABS DRn LS 1 0 #1 — — —
    201: {asm: ["FABS", "DRn"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: dn, writes: dn },
    // 202 FADD DRm,DRn FE 1 (7, 8)/9 #39 F1 2 6
    202: {asm: ["FADD", "DRm","DRn"], group: Group.FE, issue: 1, latency: [7, 8/*, 9*/], pattern: Patterns[39], reads: [dm, dn], writes: [dn, "FPSCR"] },
    // 203 FCMP/EQ DRm,DRn CO 2 3/5 #40 F1 2 2
    203: {asm: ["FCMP/EQ", "DRm","DRn"], group: Group.CO, issue: 2, latency: [3/*, 5*/], pattern: Patterns[40], reads: [dm, dn], writes: [sr, "FPSCR"] },
    // 204 FCMP/GT DRm,DRn CO 2 3/5 #40 F1 2 2
    204: {asm: ["FCMP/GT", "DRm","DRn"], group: Group.CO, issue: 2, latency: [3/*, 5*/], pattern: Patterns[40], reads: [dm, dn], writes: [sr, "FPSCR"] },
    // 205 FCNVDS DRm,FPUL FE 1 4/5 #38 F1 2 2
    205: {asm: ["FCNVDS", "DRm","FPUL"], group: Group.FE, issue: 1, latency: [4/*, 5*/], pattern: Patterns[38], reads: dm, writes: ["FPUL", "FPSCR"] },
    // 206 FCNVSD FPUL,DRn FE 1 (3, 4)/5 #38 F1 2 2
    206: {asm: ["FCNVSD", "FPUL","DRn"], group: Group.FE, issue: 1, latency: [3, 4/*, 5*/], pattern: Patterns[38], reads: fpul, writes: [dn, "FPSCR"] },
    // 207 FDIV DRm,DRn FE 1 (24, 25)/26 #41 F3 2 21 F1 20 3
    207: {asm: ["FDIV", "DRm","DRn"], group: Group.FE, issue: 1, latency: [24, 25/*, 26*/], pattern: pattern_41(21), reads: [dm, dn], writes: [dn, "FPSCR"] },
    // 208 FLOAT FPUL,DRn FE 1 (3, 4)/5 #38 F1 2 2
    208: {asm: ["FLOAT", "FPUL","DRn"], group: Group.FE, issue: 1, latency: [3, 4/*, 5*/], pattern: Patterns[38], reads: fpul, writes: [dn, "FPSCR"] },
    // 209 FMUL DRm,DRn FE 1 (7, 8)/9 #39 F1 2 6
    209: {asm: ["FMUL", "DRm","DRn"], group: Group.FE, issue: 1, latency: [7, 8/*, 9*/], pattern: Patterns[39], reads: [dm, dn], writes: [dn, "FPSCR"] },
    // 210 FNEG DRn LS 1 0 #1 — — —
    210: {asm: ["FNEG", "DRn"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: dn, writes: dn },
    // 211 FSQRT DRn FE 1 (23, 24)/25 #41 F3 2 20 F1 19 3
    211: {asm: ["FSQRT", "DRn"], group: Group.FE, issue: 1, latency: [23, 24/*, 25*/], pattern: pattern_41(20), reads: dn, writes: [dn, "FPSCR"] },
    // 212 FSUB DRm,DRn FE 1 (7, 8)/9 #39 F1 2 6
    212: {asm: ["FSUB", "DRm","DRn"], group: Group.FE, issue: 1, latency: [7, 8/*, 9*/], pattern: Patterns[39], reads: [dm, dn], writes: [dn, "FPSCR"] },
    // 213 FTRC DRm,FPUL FE 1 4/5 #38 F1 2 2
    213: {asm: ["FTRC", "DRm","FPUL"], group: Group.FE, issue: 1, latency: [4/*, 5*/], pattern: Patterns[38], reads: dm, writes: ["FPUL", "FPSCR"] },

    // FPU system control instructions
    // 214 LDS Rm,FPUL LS 1 1 #1 — — —
    214: {asm: ["LDS", "Rm","FPUL"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[1], reads: rm, writes: fpul },
    // 215 LDS Rm,FPSCR CO 1 4 #32 F1 3 3
    215: {asm: ["LDS", "Rm","FPSCR"], group: Group.CO, issue: 1, latency: 3/* should be 4 */, pattern: Patterns[32], reads: rm, writes: "FPSCR" },
    // 216 LDS.L @Rm+,FPUL CO 1 1/2 #2 — — —
    216: {asm: ["LDS.L", "@Rm+","FPUL"], group: Group.CO, issue: 1, latency: [1, 2], pattern: Patterns[2], reads: rm, writes: [rm, "FPUL"] },
    // 217 LDS.L @Rm+,FPSCR CO 1 1/4 #33 F1 3 3
    217: {asm: ["LDS.L", "@Rm+","FPSCR"], group: Group.CO, issue: 1, latency: [1/*, 4*/], pattern: Patterns[33], reads: rm, writes: [rm, "FPSCR"] },
    // 218 STS FPUL,Rn LS 1 3 #1 — — —
    218: {asm: ["STS", "FPUL","Rn"], group: Group.LS, issue: 1, latency: 3, pattern: Patterns[1], reads: fpul, writes: rn },
    // 219 STS FPSCR,Rn CO 1 3 #1 — — —
    219: {asm: ["STS", "FPSCR","Rn"], group: Group.CO, issue: 1, latency: 3, pattern: Patterns[1], reads: "FPSCR", writes: rn },
    // 220 STS.L FPUL,@-Rn CO 1 1/1 #2 — — —
    220: {asm: ["STS.L", "FPUL","@-Rn"], group: Group.CO, issue: 1, latency: [1, 1], pattern: Patterns[2], reads: fpul, writes: rn },
    // 221 STS.L FPSCR,@-Rn CO 1 1/1 #2 — — —
    221: {asm: ["STS.L", "FPSCR","@-Rn"], group: Group.CO, issue: 1, latency: [1, 1], pattern: Patterns[2], reads: "FPSCR", writes: rn },

    // Graphics acceleration instructions
    // 222 FMOV DRm,XDn LS 1 0 #1 — — —
    222: {asm: ["FMOV", "DRm","XDn"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: dm, writes: xdn },
    // 223 FMOV XDm,DRn LS 1 0 #1 — — —
    223: {asm: ["FMOV", "XDm","DRn"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: xdm, writes: dn },
    // 224 FMOV XDm,XDn LS 1 0 #1 — — —
    224: {asm: ["FMOV", "XDm","XDn"], group: Group.LS, issue: 1, latency: 0, pattern: Patterns[1], reads: xdm, writes: xdn },
    // 225 FMOV @Rm,XDn LS 1 2 #2 — — —
    225: {asm: ["FMOV", "@Rm","XDn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: rm, writes: xdn },
    // 226 FMOV @Rm+,XDn LS 1 1/2 #2 — — —
    226: {asm: ["FMOV", "@Rm+","XDn"], group: Group.LS, issue: 1, latency: [1, 2], pattern: Patterns[2], reads: rm, writes: [rm, xdn] },
    // 227 FMOV @(R0,Rm),XDn LS 1 2 #2 — — —
    227: {asm: ["FMOV", "@(R0,Rm)","XDn"], group: Group.LS, issue: 1, latency: 2, pattern: Patterns[2], reads: at_r0m, writes: xdn },
    // 228 FMOV XDm,@Rn LS 1 1 #2 — — —
    228: {asm: ["FMOV", "XDm","@Rn"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: [xdm, rn], writes: none },
    // 229 FMOV XDm,@-Rm LS 1 1/1 #2 — — —
    229: {asm: ["FMOV", "XDm","@-Rn"], group: Group.LS, issue: 1, latency: [1, 1], pattern: Patterns[2], reads: [xdm, rn], writes: rn },
    // 230 FMOV XDm,@(R0,Rn) LS 1 1 #2 — — —
    230: {asm: ["FMOV", "XDm","@(R0,Rn)"], group: Group.LS, issue: 1, latency: 1, pattern: Patterns[2], reads: [xdm, at_r0n], writes: none },
    // 231 FIPR FVm,FVn FE 1 4/5 #42 F1 3 1
    231: {asm: ["FIPR", "FVm","FVn"], group: Group.FE, issue: 1, latency: 4 /*4/5*/, pattern: Patterns[42], reads: fvm, writes: fvn },
    // 232 FRCHG FE 1 1/4 #36 — — —
    // 233 FSCHG FE 1 1/4 #36 — — —
    // 234 FTRV XMTRX,FVn FE 1 (5, 5, 6,7)/8 #43 F0 2 4 F1 3 4
    234: {asm: ["FTRV", "XMTRX","FVn"], group: Group.FE, issue: 1, latency: [5, 5, 6,7, 8], pattern: Patterns[43], reads: xmtrx, writes: [fvn, "FPSCR"] },

    // special, not in manual
    256: {asm: ["FSRRA", "FRn"], group: Group.FE, issue: 1, latency: 3 /* test this */, pattern: Patterns[36], reads: fn, writes: fn },

    // special, not in manual
    257: {asm: ["FSCA", "FPUL", "DRn"], group: Group.FE, issue: 1, latency: 3 /* test this */, pattern: Patterns[36], reads: fpul, writes: dn },
};

// Export a unique list of SH4 mnemonics for syntax highlighting
export const SH4_MNEMONICS: string[] = Array.from(new Set<string>(
    Object.values(Instructions).map((def) => ((def as unknown as { asm: string[] }).asm[0] || "").toLowerCase())
));

// Export a comprehensive set of registers based on extractor variants used by the simulator
export const SH4_REGISTERS: string[] = [
    // General-purpose regs
    ...Array.from({ length: 16 }, (_, i) => `R${i}`),
    // Banked regs
    ...Array.from({ length: 8 }, (_, i) => `R${i}_BANK`),
    // Floating-point single registers
    ...Array.from({ length: 16 }, (_, i) => `FR${i}`),
    // Floating-point double registers (even numbers)
    ...Array.from({ length: 8 }, (_, i) => `DR${i*2}`),
    // Extended floating registers (even numbers)
    ...Array.from({ length: 8 }, (_, i) => `XD${i*2}`),
    // Vector registers
    "FV0","FV4","FV8","FV12",
    // XF registers used by XMTRX
    ...Array.from({ length: 16 }, (_, i) => `XF${i}`),
    // Special registers referenced in the simulator
    "PR","SR","GBR","VBR","MACH","MACL","PC","SSP","USP","FPSCR","FPUL","XMTRX",
];

function resolve_read_write(rw) {
    if (rw instanceof Array) {
        return function() {
            let res = [];
            for (const r of rw) {
                res = res.concat(resolve_read_write(r).apply(this));
            }
            return res;
        }
    } else if (typeof rw == 'string' || rw instanceof String) {
        return () => {
            return [rw];
        }
    } else if (rw instanceof Function) {
        return rw;
    } else {
        throw new Error(`Invalid read/write specifier ${rw}`);
    }
}
// Merge read/write arrays
for (const def of Object.values(Instructions)) {
    def.reads = resolve_read_write(def.reads);
    def.writes = resolve_read_write(def.writes);
}
// validate issue
for (const def of Object.values(Instructions)) {
    if (def.issue != 1) {
        if (!def.pattern[0][1] & Flags.L) {
            throw new Error(`Instruction ${def.asm.join(" ")} has issue > 1 but no lock flag`);
        }

        if (def.issue > def.pattern.length) {
            throw new Error(`Instruction ${def.asm.join(" ")} has issue > pattern length`);
        }
        for (let i = 1; i < def.issue; i++) {
            if (!def.pattern[i][0] & Flags.L) {
                throw new Error(`Instruction ${def.asm.join(" ")} has lock flag in pattern ${i}`);
            }
        }
    }
}

// validate latency and generate patterns
for (const def of Object.values(Instructions)) {
    if (def.latency == -1) {
        def.result_seq = -1;
        continue;
    }
    if (def.latency instanceof Array) {
        // TODO: support multiple latency values
        def.latency = Math.max(...def.latency);
    }
    if (def.pattern.length == 1) {
        def.pattern = deepcopy(def.pattern);
        if (1 + def.latency >= def.pattern[0].length) {
            throw new Error(`Latency too high for ${def.asm.join(" ")}`);
        }
        def.pattern[0][1 + def.latency] |= Flags.R;
        def.result_seq = 0;
    } else {
        if (!def.pattern.some(p => p.some(s => s & Flags.R))) {
            throw new Error(`No result in pattern ${def.pattern} for ${def.asm.join(" ")}`);
        }
        def.result_seq = def.pattern.findIndex(p => p.some(s => s & Flags.R));
    }
}

const instructions_rainbow = {}

function cartesianProduct(arrays) {
    return arrays.reduce((acc, array) => {
        return acc.flatMap(accItem => {
            return array.map(arrayItem => {
                return [...accItem, arrayItem];
            });
        });
    }, [[]]);
}

function deepcopy(v) {
    return JSON.parse(JSON.stringify(v))
}

function getVariants(str) {
    switch(str) {
        case "Rm":
        case "Rn":
            return ["R0", "R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R12", "R13", "R14", "R15"];
        case "Rn_BANK":
        case "Rm_BANK":
            return ["R0_BANK", "R1_BANK", "R2_BANK", "R3_BANK", "R4_BANK", "R5_BANK", "R6_BANK", "R7_BANK"];
        case "@Rm":
        case "@Rn":
            return ["@R0", "@R1", "@R2", "@R3", "@R4", "@R5", "@R6", "@R7", "@R8", "@R9", "@R10", "@R11", "@R12", "@R13", "@R14", "@R15"];
        case "@Rn+":
        case "@Rm+":
            return ["@R0+", "@R1+", "@R2+", "@R3+", "@R4+", "@R5+", "@R6+", "@R7+", "@R8+", "@R9+", "@R10+", "@R11+", "@R12+", "@R13+", "@R14+", "@R15+"];
        case "@-Rn":
            return ["@-R0", "@-R1", "@-R2", "@-R3", "@-R4", "@-R5", "@-R6", "@-R7", "@-R8", "@-R9", "@-R10", "@-R11", "@-R12", "@-R13", "@-R14", "@-R15"];
        case "#imm":
            return Array.from({ length: 384 }, (_, index) => `#${index-128}`);
        case "@(disp,PC)":
            return Array.from({ length: 256 }, (_, index) => `@(${index},PC)`);
        case "@(R0,Rm)":
        case "@(R0,Rn)":
            return ["@(R0,R0)", "@(R0,R1)", "@(R0,R2)", "@(R0,R3)", "@(R0,R4)", "@(R0,R5)", "@(R0,R6)", "@(R0,R7)", "@(R0,R8)", "@(R0,R9)", "@(R0,R10)", "@(R0,R11)", "@(R0,R12)", "@(R0,R13)", "@(R0,R14)", "@(R0,R15)"];
        case "FRm":
        case "FRn":
            return ["FR0", "FR1", "FR2", "FR3", "FR4", "FR5", "FR6", "FR7", "FR8", "FR9", "FR10", "FR11", "FR12", "FR13", "FR14", "FR15"];
        case "DRm":
        case "DRn":
            return ["DR0", "DR2", "DR4", "DR6", "DR8", "DR10", "DR12", "DR14"];
        case "XDm":
        case "XDn":
            return ["XD0", "XD2", "XD4", "XD6", "XD8", "XD10", "XD12", "XD14"];
        case "FVm":
        case "FVn":
            return ["FV0", "FV4", "FV8", "FV12"];
        case "@(disp4,Rn)":
        case "@(disp4,Rm)":
            return Array.from({ length: 16*4 }, (_, index) =>  Array.from({ length: 16 }, (_, index2) => `@(${index},R${index2})`)).flat();
        case "disp8":
            return Array.from({ length: 256 }, (_, index) => `#${(index-128)*2}`);
        case "disp12":
            return Array.from({ length: 4096 }, (_, index) => `#${(index-2048)*2}`);
        default:
            return [str]
    }
}

function processInsn(no_comments) {
    return no_comments
        .replace(/\s+/g, " ")
        .replace(/\s*,\s*/g, ",")
        .replace(/@\s*/g, "@")
        .replace(/\s*\(\s*/g, "(")
        .replace(/\s*\)\s*/g, ")")
        .toLowerCase()
        .trim();
}

for (const index of Object.keys(Instructions)) {
    const insn = Instructions[index];

    const parts = insn.asm;

    const insn_variants = cartesianProduct(parts.map(getVariants));
    for (const insn_variant of insn_variants) {
        const insn_rain = deepcopy(insn)
        insn_rain.reads = insn.reads;
        insn_rain.writes = insn.writes;
        insn_rain.variant = insn_variant;
        instructions_rainbow[processInsn((insn_variant[0] + " " + insn_variant.slice(1).join(",")))] = insn_rain;
    }
}

let lastAssembleError = null;

function assemble(lines) {
    lastAssembleError = null;
    let insns = []
    const rv = [insns]
    let pc = 0;
    let track = 0;
    for (const line of lines.split("\n")) {
        const no_comments = line.split(/(;|!|\/\/)/)[0].replace("\t", " ").trim();

        if (no_comments.length) {
            const processed = processInsn(no_comments);
            if (processed[0] == '#') {
                if (insns.length != 0) {
                    console.log(`Starting new fragment: ${processed} from ${line}`);
                    insns.tracks = track;
                    const newBlock = [];
                    rv.push(newBlock);
                    insns = newBlock;
                    pc = 0;
                    track = 0;
                }
                if (processed.startsWith("##")) {
                    insns.subtitle = processed.substring(2).trim();
                    console.log("Attaching subtitle: " + insns.subtitle);
                } else if (processed.startsWith("#")) {
                    insns.title = processed.substring(1).trim();
                    console.log("Attaching title: " + insns.title);
                }
                continue;
            }
            if (processed[0] == '.') {
                console.log(`Skipping directive: ${processed} from ${line}`);
                continue;
            }
            if (processed[processed.length - 1] == ':') {
                console.log(`Skipping label: ${processed} from ${line}`);
                continue;
            }
            let def = instructions_rainbow[processed];
            if (!def) {
                // special handling for branches
                const op = processed.split(" ")[0];
                def = instructions_rainbow[op];
            }

            if (!def) {
                // special handling for data moves w/ labels
                // note: Label validity is not checked, and neither is distance
                const op = processed.split(" ")[0];
                if (op == "mova" || op == "mov.w" || op == "mov.l") {
                    const with_pc_disp = processed.replace(/ .*,/, " @(0,pc),");
                    def = instructions_rainbow[with_pc_disp];
                }
            }

            if (!def) {
                const message = `Unknown instruction: ${processed}`;
                console.error(`${message} from ${line}`);
                lastAssembleError = message;
                return null;
            }
            insns.push({
                pc: pc, track: track, text: processed, def: def, program_order: -1, seq: [],
                format: function() {
                    return `${this.pc.toString(16).padStart(8,"0")} ${this.text}`;
                }
            });
            track += def.pattern.length;
            pc += 2;
        }
    }

    insns.tracks = track;

    lastAssembleError = null;
    return rv;
}
function getSeq(insn, num) {
    return insn.seq[num];
}

function makeSeq(insn, program_order) {
    const seqs = [];

    for (let num = 0; num < insn.def.pattern.length; num++) {
        const seq = {};
        seq.stage = function() {
            return this.pattern[this.step] & Stage.Mask;
        }
        seq.stage_lock = function() {
            return this.pattern[this.step] & (Flags.L | Flags.LP);
        }
        seq.generates_result = function() {
            return this.pattern[this.step] & Flags.R;
        }
        seq.kick = function() {
            return this.pattern[this.step] >> Flags.KShift;
        }
        seq.next_kick = function() {
            return this.pattern[this.step + 1] >> Flags.KShift;
        }
        seq.next_stage = function() {
            const nextStageValue = this.pattern[this.step + 1];
            return nextStageValue ? nextStageValue & Stage.Mask : nextStageValue;
        }
        seq.is_last_stage = function() {
            return this.step == this.pattern.length - 1;
        }
        seq.pattern = deepcopy(insn.def.pattern[num]);
        seq.step = 0;
        seq.program_order = program_order + num;
        seq.stall = false;
        seq.insn = insn;
        seq.reads = [...new Set(insn.def.reads())];
        seq.writes = [... new Set(insn.def.writes())];
        seq.latency = insn.def.latency;
        seq.group = insn.def.group;
        seq.pc = insn.pc;
        seq.track = insn.track + num;

        seqs.push(seq);
    }
    return seqs;
}

export const COLUMNS_PER_GROUP = 10;
type RawCell = Record<string, unknown>;

function selectorToHighlightKey(selector) {
    if (typeof selector !== "string") {
        return null;
    }
    const insnMatch = selector.match(/^\[data-insn="([^"\\]+)"\]$/);
    if (insnMatch) {
        const value = insnMatch[1];
        if (value.startsWith("step-")) {
            return `cell:${value}`;
        }
        return `insn:${value}`;
    }
    const resultMatch = selector.match(/^\[data-result-ready="([^"\\]+)"\]$/);
    if (resultMatch) {
        return `result:${resultMatch[1]}`;
    }
    return null;
}

function prepareTable(tableArray: RawCell[][]): SimTable {
    if (!tableArray.length) {
        return {
            rows: [],
            columnCount: 0,
            columnsPerGroup: COLUMNS_PER_GROUP,
        };
    }

    const numRows = tableArray[0].length;
    const columnCount = tableArray.length;
    const rows: SimRow[] = [];

    for (let rowIndex = 0; rowIndex < numRows; rowIndex++) {
        const baseCell = tableArray[0][rowIndex] || {};
        const instructionPc = baseCell && baseCell.pc !== undefined ? baseCell.pc : undefined;
        const rowKey = instructionPc !== undefined ? `row-insn-${instructionPc}-${rowIndex}` : `row-${rowIndex}`;
        const classes: string[] = [];
        if (instructionPc !== undefined) {
            if (rowIndex === 0 || ((tableArray[0][rowIndex - 1] || {}).pc !== instructionPc)) {
                classes.push("start");
            }
            if (rowIndex === numRows - 1 || ((tableArray[0][rowIndex + 1] || {}).pc !== instructionPc)) {
                classes.push("end");
            }
        }

        const cells: SimCell[] = [];
        let onlyColumn0HasText = true;

        for (let colIndex = 0; colIndex < columnCount; colIndex++) {
            const column = tableArray[colIndex] || [];
            const originalCell = column[rowIndex] ? { ...column[rowIndex] } : {};
            if (originalCell && originalCell.seq) {
                delete originalCell.seq;
            }

            let relevantKeys: string[] = [];
            if (originalCell && originalCell.relevant) {
                try {
                    const selectors = JSON.parse(originalCell.relevant);
                    if (Array.isArray(selectors)) {
                        relevantKeys = selectors
                            .map(selectorToHighlightKey)
                            .filter((key): key is string => Boolean(key));
                    }
                } catch {
                    relevantKeys = [];
                }
            }

            const selfKeys = new Set<string>();
            if (colIndex === 0 && originalCell && originalCell.id !== undefined) {
                selfKeys.add(`insn:${instructionPc ?? originalCell.id ?? `${rowIndex}`}`);
            }
            if (colIndex !== 0 && originalCell && originalCell.id !== undefined) {
                selfKeys.add(`cell:${originalCell.id}`);
            }
            if (originalCell && originalCell.result_ready !== undefined) {
                selfKeys.add(`result:${originalCell.result_ready}`);
            }

            const text = originalCell && originalCell.text ? originalCell.text : "";
            if (colIndex > 0) {
                if ((text && text.trim() !== "") || originalCell.stall || originalCell.lock || originalCell.full) {
                    onlyColumn0HasText = false;
                }
            }

            cells.push({
                id: originalCell && originalCell.id !== undefined ? String(originalCell.id) : undefined,
                text,
                explanation: originalCell && originalCell.explanation ? originalCell.explanation : null,
                stall: Boolean(originalCell && originalCell.stall),
                lock: Boolean(originalCell && originalCell.lock),
                full: Boolean(originalCell && originalCell.full),
                screenHidden: Boolean(originalCell && originalCell.screen_hidden),
                screenHiddenText: Boolean(originalCell && originalCell.screen_hidden_text),
                columnIndex: colIndex,
                rowIndex,
                cycle: colIndex === 0 ? null : colIndex - 1,
                currentRowKey: instructionPc !== undefined ? rowKey : null,
                relevantKeys,
                selfKeys: Array.from(selfKeys),
                resultReadyKey: originalCell && originalCell.result_ready !== undefined ? `result:${originalCell.result_ready}` : null,
            });
        }

        rows.push({
            rowIndex,
            rowKey,
            instructionPc,
            classes,
            printHidden: onlyColumn0HasText && rowIndex !== 0,
            cells,
        });
    }

    return {
        rows,
        columnCount,
        columnsPerGroup: COLUMNS_PER_GROUP,
    };
}

function simulateBlock(insns): { table: SimTable; cycleCount: number } {
    let pc = 0;
    let cycle = 0;
    let program_order = 0;
    let in_flight = [];

    const table: RawCell[][] = [];
    const initial_column: RawCell[] = Array.from({ length: insns.tracks }, () => ({}));
    for (let i = 0; i < insns.length; i++) {
        initial_column[insns[i].track] = {
            id: `${insns[i].pc}`,
            text: insns[i].format(),
            explanation: `group: ${insns[i].def.group}, issue: ${insns[i].def.issue}, latency: ${insns[i].def.latency}${insns[i].def.desc ? "<br/>" + insns[i].def.desc : ""}`,
            pc: insns[i].pc,
            current: `.row-insn-${insns[i].pc}`,
        };

        for (let j = 1; j < insns[i].def.pattern.length; j++) {
            const existing = initial_column[insns[i].track + j];
            if (!Object.keys(existing).length) {
                initial_column[insns[i].track + j] = deepcopy(initial_column[insns[i].track]);
            }
            initial_column[insns[i].track + j].screen_hidden_text = true;
        }
    }

    initial_column.unshift({ text: "inst\\cycle", explanation: "Instruction vs Cycle Number" });
    table.push(initial_column);

    const provides = {
        "R0": [],
        "R1": [],
        "R2": [],
        "R3": [],
        "R4": [],
        "R5": [],
        "R6": [],
        "R7": [],
        "R8": [],
        "R9": [],
        "R10": [],
        "R11": [],
        "R12": [],
        "R13": [],
        "R14": [],
        "R15": [],

        "R0_BANK": [],
        "R1_BANK": [],
        "R2_BANK": [],
        "R3_BANK": [],
        "R4_BANK": [],
        "R5_BANK": [],
        "R6_BANK": [],
        "R7_BANK": [],

        "FR0": [],
        "FR1": [],
        "FR2": [],
        "FR3": [],
        "FR4": [],
        "FR5": [],
        "FR6": [],
        "FR7": [],
        "FR8": [],
        "FR9": [],
        "FR10": [],
        "FR11": [],
        "FR12": [],
        "FR13": [],
        "FR14": [],
        "FR15": [],

        "XF0": [],
        "XF1": [],
        "XF2": [],
        "XF3": [],
        "XF4": [],
        "XF5": [],
        "XF6": [],
        "XF7": [],
        "XF8": [],
        "XF9": [],
        "XF10": [],
        "XF11": [],
        "XF12": [],
        "XF13": [],
        "XF14": [],
        "XF15": [],

        "SR": [],
        "SSR": [],
        "SPC": [],
        "FPUL": [],
        "DBR": [],
        "GBR": [],
        "VBR": [],
        "MACH": [],
        "MACL": [],
        "PR": [],
        "FPSCR": [],
    };

    function data_provided_by(provide_seqs, seq) {
        return provide_seqs.some((x) => x.insn !== seq.insn && x.program_order < seq.program_order);
    }
    const stage_lock = {};

    for (;;) {
        if ((in_flight.length === 0 && pc === insns.length) || cycle > 1000) {
            break;
        }

        table.push(Array.from({ length: insns.tracks }, () => ({})));
        const last_column = table[table.length - 1];

        for (let i = 0; i < insns.length; i++) {
            for (let j = 0; j < insns[i].def.pattern.length; j++) {
                if (last_column[insns[i].track + j]) {
                    last_column[insns[i].track + j].current = `.row-insn-${insns[i].pc}`;
                }
            }
        }

        const toremove = [];
        const toresult = [];
        let prevstall = undefined;

        for (let seq_index = 0; seq_index < in_flight.length; seq_index++) {
            const seq = in_flight[seq_index];

            const current_stage = seq.stage();
            const next_stage = seq.next_stage();
            const current_stage_name = StageNames[current_stage];
            const next_stage_name = StageNames[next_stage];

            const in_next_stage = in_flight.filter((x) => x !== seq && x.stage() == next_stage && x.program_order < seq.program_order);

            if (in_next_stage.length == 2) {
                const relevant_seqs = in_next_stage;
                last_column[seq.track] = { id: `step-${seq.track}-${cycle}`, seq: seq, lock: stage_lock[seq.stage()] == seq, stall: true, text: `${current_stage_name}*${next_stage_name}`, explanation: `Already two instructions @ Stage ${next_stage_name}<br/> ${in_next_stage.map((x) => `[${x.group}: ${x.insn.format()} @ ${StageNames[x.stage()]}]`).join("<br />")}` };
                last_column[seq.track].relevant = JSON.stringify(relevant_seqs.map((x) => [`[data-insn="${x.insn.pc}"]`, `[data-insn="step-${x.track}-${cycle}"]`]).flat());
                seq.stall = true;
            } else if (stage_lock[next_stage] && stage_lock[next_stage] != seq) {
                const relevant_seqs = [stage_lock[next_stage]];
                last_column[seq.track] = { id: `step-${seq.track}-${cycle}`, seq: seq, lock: stage_lock[seq.stage()] == seq, stall: true, text: `${current_stage_name}~${next_stage_name}`, explanation: `Stage Locked: ${next_stage_name}<br/>${relevant_seqs.map((x) => `[${x.group}: ${x.insn.format()}]`).join("<br/>")}` };
                last_column[seq.track].relevant = JSON.stringify(relevant_seqs.map((x) => [`[data-insn="${x.insn.pc}"]`, `[data-insn="step-${x.track}-${cycle}"]`]).flat());
            } else if ((next_stage != Stage.D) && !in_next_stage.every((x) => x.program_order > seq.program_order || isParallel(x.group, seq.group))) {
                const relevant_seqs = in_next_stage.filter((x) => !isParallel(x.group, seq.group));
                last_column[seq.track] = { id: `step-${seq.track}-${cycle}`, seq: seq, lock: stage_lock[seq.stage()] == seq, stall: true, text: `${current_stage_name}!${next_stage_name}`, explanation: `Resource hazard: ${seq.group} @ Stage ${next_stage_name}<br/>${in_next_stage.filter((x) => x.program_order <= seq.program_order && !isParallel(x.group, seq.group)).map((x) => `[${x.group}: ${x.insn.format()} @ ${StageNames[x.stage()]}]`).join("<br/>")}` };
                last_column[seq.track].relevant = JSON.stringify(relevant_seqs.map((x) => [`[data-insn="${x.insn.pc}"]`, `[data-insn="step-${x.track}-${cycle}"]`]).flat());
                seq.stall = true;
            } else if ((current_stage != Stage.I || seq.latency == 0) && seq.reads.some((reg) => data_provided_by(provides[reg], seq))) {
                const relevant_seqs = seq.reads.map((reg) => provides[reg].filter((provides_seq) => provides_seq.program_order < seq.program_order)).flat(Infinity);
                last_column[seq.track] = { id: `step-${seq.track}-${cycle}`, seq: seq, lock: stage_lock[seq.stage()] == seq, stall: true, text: `${current_stage_name}|${next_stage_name}`, explanation: `Flow Dependency<br/>${seq.reads.map((reg) => provides[reg].filter((provides_seq) => provides_seq.program_order < seq.program_order).map((provides_seq) => `${reg}: ${provides_seq.insn.format()}`)).flat(Infinity).join("<br/>")}` };
                last_column[seq.track].relevant = JSON.stringify(relevant_seqs.map((x) => [`[data-insn="${x.insn.pc}"]`, `[data-result-ready="${x.insn.program_order}"]`]).flat());
                seq.stall = true;
            } else if ((current_stage != Stage.I) && seq.writes.some((reg) => data_provided_by(provides[reg], seq))) {
                const relevant_seqs = seq.writes.map((reg) => provides[reg].filter((provides_seq) => provides_seq.program_order < seq.program_order)).flat(Infinity);
                last_column[seq.track] = { id: `step-${seq.track}-${cycle}`, seq: seq, lock: stage_lock[seq.stage()] == seq, stall: true, text: `${current_stage_name}^${next_stage_name}`, explanation: `Output Dependency<br/>${seq.writes.map((reg) => provides[reg].filter((provides_seq) => provides_seq.program_order < seq.program_order).map((provides_seq) => `${reg}: ${provides_seq.insn.format()}`)).flat(Infinity).join("<br/>")}` };
                last_column[seq.track].relevant = JSON.stringify(relevant_seqs.map((x) => [`[data-insn="${x.insn.pc}"]`, `[data-result-ready="${x.insn.program_order}"]`]).flat());
                seq.stall = true;
            } else if (prevstall) {
                const relevant_seqs = [prevstall];
                last_column[seq.track] = { id: `step-${seq.track}-${cycle}`, seq: seq, lock: stage_lock[seq.stage()] == seq, stall: true, text: `${current_stage_name}+${next_stage_name}`, explanation: `Previous Instruction Stalled<br/>${prevstall.insn.format()}` };
                last_column[seq.track].relevant = JSON.stringify(relevant_seqs.map((x) => [`[data-insn="${x.insn.pc}"]`, `[data-insn="step-${x.track}-${cycle}"]`]).flat());
                seq.stall = true;
            } else {
                seq.stall = false;
                if (seq.stage_lock()) {
                    stage_lock[seq.stage()] = null;
                }
                let result_ready = undefined;
                seq.step++;
                if (seq.generates_result()) {
                    result_ready = seq.insn.program_order.toString();
                    toresult.push(seq);
                }
                if (seq.stage_lock()) {
                    stage_lock[seq.stage()] = seq;
                }
                if (!seq.next_stage()) {
                    toremove.push(seq);
                }
                function do_kick(kicked_seq) {
                    if (kicked_seq.kick()) {
                        const kickSeq = getSeq(kicked_seq.insn, kicked_seq.kick());
                        if (kickSeq.stage_lock()) {
                            stage_lock[kickSeq.stage()] = kickSeq;
                        }
                        if (!kickSeq.next_stage()) {
                            toremove.push(kickSeq);
                        }
                        in_flight.splice(in_flight.indexOf(kicked_seq) + 1, 0, kickSeq);
                        seq_index++;
                        last_column[kickSeq.track] = { id: `step-${kickSeq.track}-${cycle}`, seq: kickSeq, lock: stage_lock[kickSeq.stage()] == kickSeq, text: StageNames[kickSeq.stage()], explanation: `No Stall, Group: ${kickSeq.group}${stage_lock[kickSeq.stage()] ? `<br/>Stage Lock: ${StageNames[kickSeq.stage()]}` : ""}` };
                        last_column[kickSeq.track].current = `.row-insn-${kickSeq.insn.pc}`;
                        do_kick(kickSeq);
                    }
                }
                do_kick(seq);

                last_column[seq.track] = { id: `step-${seq.track}-${cycle}`, seq: seq, lock: stage_lock[seq.stage()] == seq, text: StageNames[next_stage], explanation: `No Stall, Group: ${seq.group}${stage_lock[seq.stage()] ? `<br/>Stage Lock: ${StageNames[seq.stage()]}` : ""}`, result_ready: result_ready };
            }

            if (!prevstall && seq.stall) {
                prevstall = seq;
            }

            last_column[seq.track].current = `.row-insn-${seq.insn.pc}`;
        }

        for (let stage = 0; stage < Stage.Count; stage++) {
            const in_stage_exec = in_flight.filter((seq) => seq.stage() == stage && !seq.stall);
            if (in_stage_exec.length == 2) {
                in_stage_exec.forEach((seq) => {
                    last_column.forEach((x) => {
                        if (x && x.seq == seq) {
                            x.full = true;
                            x.explanation += `<br/>Fully Utilized`;
                        }
                    });
                });
            }
        }

        for (const seq of toresult) {
            seq.writes.forEach((reg) => {
                provides[reg] = provides[reg].filter((e) => e !== seq);
            });
        }
        toresult.length = 0;

        for (const seq of toremove) {
            in_flight = in_flight.filter((e) => e !== seq);

            if (in_flight.filter((x) => x.insn == seq.insn).length == 0) {
                seq.writes.forEach((reg) => {
                    if (provides[reg].filter((e) => e == seq).length) {
                        throw new Error(`Instruction finished before all data written ${seq.insn.format()}`);
                    }
                });
            }
            if (seq.stage_lock()) {
                stage_lock[seq.stage()] = null;
            }
        }
        toremove.length = 0;

        const in_i_stage = in_flight.filter((seq) => seq.stage() == Stage.I);

        for (let pipe = 0; pipe < 2 - in_i_stage.length && pc != insns.length; pipe++) {
            const insn = insns[pc++];
            insn.program_order = program_order;
            insn.seq = makeSeq(insn, program_order);
            program_order += insn.def.pattern.length;
            const seq = getSeq(insn, 0);
            in_flight.push(seq);
            if (insn.def.result_seq !== -1) {
                const resultSeq = getSeq(insn, insn.def.result_seq, program_order);
                resultSeq.writes.forEach((reg) => {
                    provides[reg].push(resultSeq);
                });
            }
            last_column[seq.track] = { id: `step-${seq.track}-${cycle}`, seq: seq, text: StageNames[Stage.I], explanation: `No Stall, Group: ${insn.def.group}` };
            last_column[seq.track].current = `.row-insn-${seq.insn.pc}`;
        }

        last_column.unshift({ text: cycle.toString(), explanation: "Cycle Number" });
        cycle++;
    }

    return { table: prepareTable(table), cycleCount: cycle };
}

export interface SimCell {
    id?: string;
    text: string;
    explanation: string | null;
    stall: boolean;
    lock: boolean;
    full: boolean;
    screenHidden: boolean;
    screenHiddenText: boolean;
    columnIndex: number;
    rowIndex: number;
    cycle: number | null;
    currentRowKey: string | null;
    relevantKeys: string[];
    selfKeys: string[];
    resultReadyKey: string | null;
}

export interface SimRow {
    rowIndex: number;
    rowKey: string;
    instructionPc?: number;
    classes: string[];
    printHidden: boolean;
    cells: SimCell[];
}

export interface SimTable {
    rows: SimRow[];
    columnCount: number;
    columnsPerGroup: number;
}

export interface SimBlock {
    id: string;
    title: string | null;
    subtitle: string | null;
    table: SimTable;
    cycleCount: number;
}

export interface SimulateResult {
    blocks: SimBlock[];
    error: string | null;
}

export function simulate(source: string): SimulateResult {
    const insnsBlocks = assemble(source);
    if (!insnsBlocks) {
        return {
            blocks: [],
            error: lastAssembleError || "Failed to assemble source.",
        };
    }

    const blocks: SimBlock[] = [];
    for (let index = 0; index < insnsBlocks.length; index++) {
        const insns = insnsBlocks[index];
        const { table, cycleCount } = simulateBlock(insns);
        blocks.push({
            id: `block-${index}`,
            title: insns.title || null,
            subtitle: insns.subtitle || null,
            table,
            cycleCount,
        });
    }

    return { blocks, error: null };
}

export function getAssembleError() {
    return lastAssembleError;
}
