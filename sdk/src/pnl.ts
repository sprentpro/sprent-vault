const ZERO = BigInt(0);
const E12 = BigInt("1000000000000");

export interface PnlFill {
  at: Date;
  deskBought: boolean;
  baseAtoms: bigint;
  quoteAtoms: bigint;
}

export interface PnlMark {
  at: Date;
  midE12: bigint;
}

export interface Horizon {
  seconds: number;
  fills: number;
  markoutAtoms: bigint;
  adverseAtoms: bigint;
}

export interface PnlBreakdown {
  fills: number;
  marked: number;
  unmarked: number;
  headlineFills: number;
  spreadCaptureAtoms: bigint;
  adverseSelectionAtoms: bigint;
  costAtoms: bigint;
  netAtoms: bigint;
  horizons: Horizon[];
  markToleranceSec: number;
}