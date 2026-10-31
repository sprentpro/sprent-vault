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