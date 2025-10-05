export const SH4_SIM_DEFAULT_SOURCE = `; Basic examples from the manual (not all of these are simulated correctly).
; # means a section split w/ title
; ## means a section split w/ subtitle


# Serial execution: non-parallel-executable instructions
    
SHAD R0,R1
ADD R2,R3


# Parallel execution: parallel-executable and no dependency

ADD R2,R1
MOV.L @R4,R5


# Issue rate: multi-step instruction

AND.B #1,@(R0,GBR)
MOV R1,R2


# Flow dependency

## Zero-cycle latency

MOV R0,R1
ADD R2,R1


## 1-cycle latency

ADD R2,R1
MOV.L @R1,R1

## 2-cycle latency / 1 cycle stall 
nop
MOV.L @R1,R1
ADD R0,R1

## 2-cycle latency / 1 cycle increase
nop
MOV.L @R1,R1
SHAD R1,R2

## 4-cycle latency for FPSCR

FADD FR1,FR2
nop
STS FPUL,R1
nop
STS FPSCR,R2

## 7-cycle latency for lower FR, 8-cycle latency for upper FR

FADD DR0,DR2
FMOV FR3,FR5
FMOV FR2,FR4

## 3-cycle latency for lower FR / 4-cycle latency for upper FR
FLOAT FPUL,DR0
FMOV.S FR1,@-R15

## Zero-cycle latency / 3-cycle increase
FLDI1 FR3
FIPR FV0,FV4

## 2-cycle latency / 1-cycle increase
FMOV @R1,XD14
FTRV XMTRX,FV0

## Effectively 1-cycle latency for consecutive LDS/FLOAT instructions
LDS R0,FPUL
FLOAT FPUL,FR0
LDS R1,FPUL
FLOAT FPUL,FR1

## Effectively 1-cycle latency for consecutive FTRC/STS instructions
FTRC FR0,FPUL
STS FPUL,R0
FTRC FR1,FPUL
STS FPUL,R1

# Output dependency

## 11-cycle latency / WaW cycles -2 / The registers are written-back in program order.

FSQRT FR4
FMOV FR0,FR4

## 7-cycle latency for lower FR / 8-cycle latency for upper FR / WaW in program order

FADD DR0,DR2
FMOV FR0,FR3

# Anti-flow dependency

## 1 stall cycle
FTRV XMTRX,FV0
FMOV @R1,XD0

## 2 stall cycles
FADD DR0,DR2
FMOV FR4,FR1

# Resource Conflict
## F1-lock for 1 cycle
FDIV FR7, FR7
FMAC FR0,FR8,FR9
FMAC FR0,FR10,FR11
FMAC FR0,FR12,FR13
FMAC FR0,FR14,FR15

## F1-non lock stall 1 cycle
nop
FIPR FV8,FV0
FADD FR15,FR4

## Partial stage locks, 3 stall cycles
LDS.L @R15+,PR
STC GBR,R2

## 5 stall cycles (sim is wrong here)
nop
FADD DR0,DR2
MAC.W @R1+,@R2+

## 1, 2, 3 stalls: f1 stage can overlap f1, but not F1
MAC.W @R1+,@R2+
MAC.W @R1+,@R2+
FADD DR4,DR6
`;
