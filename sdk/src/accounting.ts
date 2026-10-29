const ZERO = BigInt(0);
const ONE = BigInt(1);

export const PPS_SCALE = BigInt("1000000000000");
export const BPS_DENOM = BigInt(10000);
export const DEFAULT_PERF_FEE_BPS = BigInt(1000);
export const BOOTSTRAP_SHARES = BigInt(1000);
export const MIN_FIRST_DEPOSIT = BigInt(1_000_000);

export const SERVICE_SPLIT_BPS = { holder: BigInt(3000), buyback: BigInt(1000), team: BigInt(6000) } as const;
export const CREATOR_BUYBACK_BPS = BigInt(2000);

export class AccountingError extends Error {
  constructor(
    public readonly code: "ZERO_SHARES" | "FIRST_DEPOSIT_TOO_SMALL" | "NEGATIVE" | "INSUFFICIENT_SHARES" | "EMPTY_VAULT" | "BAD_BPS",
    message: string,
  ) {
    super(message);
    this.name = "AccountingError";
  }
}

function nonNeg(v: bigint, what: string): bigint {
  if (v < ZERO) throw new AccountingError("NEGATIVE", `${what} must be >= 0`);
  return v;
}

function floorDiv(a: bigint, b: bigint): bigint {
  return a / b;
}

export function ceilDiv(a: bigint, b: bigint): bigint {
  return a === ZERO ? ZERO : (a - ONE) / b + ONE;
}

export interface VaultState {
  totalAssets: bigint;
  totalShares: bigint;
}

export function ppsOf(s: VaultState): bigint {
  nonNeg(s.totalAssets, "totalAssets");
  nonNeg(s.totalShares, "totalShares");
  if (s.totalShares === ZERO) return PPS_SCALE;
  return floorDiv(s.totalAssets * PPS_SCALE, s.totalShares);
}

export interface MintResult {
  sharesMinted: bigint;
  deadShares: bigint;
  state: VaultState;
}

export function mintShares(depositAtoms: bigint, s: VaultState): MintResult {
  nonNeg(depositAtoms, "deposit");
  if (s.totalShares === ZERO) {
    if (depositAtoms < MIN_FIRST_DEPOSIT) throw new AccountingError("FIRST_DEPOSIT_TOO_SMALL", "first deposit below the minimum");
    const shares = depositAtoms - BOOTSTRAP_SHARES;
    return {
      sharesMinted: shares,
      deadShares: BOOTSTRAP_SHARES,
      state: { totalAssets: s.totalAssets + depositAtoms, totalShares: depositAtoms },
    };
  }
  if (s.totalAssets === ZERO) throw new AccountingError("EMPTY_VAULT", "vault has shares but no assets");
  const shares = floorDiv(depositAtoms * s.totalShares, s.totalAssets);
  if (shares === ZERO) throw new AccountingError("ZERO_SHARES", "deposit buys zero shares");
  return {
    sharesMinted: shares,
    deadShares: ZERO,
    state: { totalAssets: s.totalAssets + depositAtoms, totalShares: s.totalShares + shares },
  };
}

export function sharesAtPps(depositAtoms: bigint, ppsE12: bigint): bigint {
  nonNeg(depositAtoms, "deposit");
  if (ppsE12 <= ZERO) throw new AccountingError("EMPTY_VAULT", "pps must be > 0");
  const shares = floorDiv(depositAtoms * PPS_SCALE, ppsE12);
  if (shares === ZERO) throw new AccountingError("ZERO_SHARES", "deposit buys zero shares");
  return shares;
}

export function redeemShares(shares: bigint, s: VaultState): bigint {
  nonNeg(shares, "shares");
  if (s.totalShares === ZERO) return ZERO;
  if (shares > s.totalShares) throw new AccountingError("INSUFFICIENT_SHARES", "redeeming more shares than exist");
  return floorDiv(shares * s.totalAssets, s.totalShares);
}