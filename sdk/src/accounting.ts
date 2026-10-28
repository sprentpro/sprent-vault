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