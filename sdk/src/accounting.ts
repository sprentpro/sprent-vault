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

export function redeemValue(shares: bigint, epochPpsE12: bigint): bigint {
  nonNeg(shares, "shares");
  nonNeg(epochPpsE12, "pps");
  return floorDiv(shares * epochPpsE12, PPS_SCALE);
}

export interface Position {
  shares: bigint;
  hwmPpsE12: bigint;
}

export interface CrystalliseResult {
  gainPpsE12: bigint;
  feeAtoms: bigint;
  feeShares: bigint;
  position: Position;
  state: VaultState;
}

export function crystallise(pos: Position, s: VaultState, epochPpsE12: bigint, feeBps: bigint = DEFAULT_PERF_FEE_BPS): CrystalliseResult {
  nonNeg(pos.shares, "shares");
  if (feeBps < ZERO || feeBps > BPS_DENOM) throw new AccountingError("BAD_BPS", "fee bps out of range");
  if (pos.shares > s.totalShares) throw new AccountingError("INSUFFICIENT_SHARES", "position larger than the vault");
  const gain = epochPpsE12 > pos.hwmPpsE12 ? epochPpsE12 - pos.hwmPpsE12 : ZERO;
  const num = feeBps * gain * pos.shares;
  const den = BPS_DENOM * PPS_SCALE;
  const newHwm = epochPpsE12 > pos.hwmPpsE12 ? epochPpsE12 : pos.hwmPpsE12;
  if (num < den || epochPpsE12 <= ZERO) {
    return { gainPpsE12: gain, feeAtoms: ZERO, feeShares: ZERO, position: { shares: pos.shares, hwmPpsE12: newHwm }, state: s };
  }
  let fee = ceilDiv(num, den);
  let feeShares = ceilDiv(fee * PPS_SCALE, epochPpsE12);
  if (feeShares > pos.shares) {
    feeShares = pos.shares;
    fee = redeemValue(feeShares, epochPpsE12);
  }
  if (fee > s.totalAssets) throw new AccountingError("NEGATIVE", "fee larger than vault assets");
  return {
    gainPpsE12: gain,
    feeAtoms: fee,
    feeShares,
    position: { shares: pos.shares - feeShares, hwmPpsE12: newHwm },
    state: { totalAssets: s.totalAssets - fee, totalShares: s.totalShares - feeShares },
  };
}

export interface WithdrawResult {
  crystallised: CrystalliseResult;
  redeemAtoms: bigint;
  position: Position;
  state: VaultState;
}

export function withdraw(pos: Position, s: VaultState, epochPpsE12: bigint, shares: bigint | "all", feeBps: bigint = DEFAULT_PERF_FEE_BPS): WithdrawResult {
  const c = crystallise(pos, s, epochPpsE12, feeBps);
  const take = shares === "all" ? c.position.shares : shares;
  nonNeg(take, "shares");
  if (take > c.position.shares) throw new AccountingError("INSUFFICIENT_SHARES", "withdrawing more shares than held");
  const redeem = redeemShares(take, c.state);
  if (redeem > c.state.totalAssets) throw new AccountingError("NEGATIVE", "redemption larger than vault assets");
  return {
    crystallised: c,
    redeemAtoms: redeem,
    position: { shares: c.position.shares - take, hwmPpsE12: c.position.hwmPpsE12 },
    state: { totalAssets: c.state.totalAssets - redeem, totalShares: c.state.totalShares - take },
  };
}

export interface NavInputs {
  trackedQuoteAtoms: bigint;
  trackedBaseAtoms: bigint;
  actualQuoteAtoms: bigint;
  actualBaseAtoms: bigint;
  markPriceE12: bigint;
}

export interface NavResult {
  navQuoteAtoms: bigint;
  surplusQuoteAtoms: bigint;
  surplusBaseAtoms: bigint;
  shortfallQuoteAtoms: bigint;
  shortfallBaseAtoms: bigint;
  reconciled: boolean;
}

export function computeNav(n: NavInputs, toleranceAtoms: bigint = ZERO): NavResult {
  for (const [k, v] of Object.entries(n)) nonNeg(v as bigint, k);
  const navQuoteAtoms = n.trackedQuoteAtoms + floorDiv(n.trackedBaseAtoms * n.markPriceE12, PPS_SCALE);
  const dq = n.actualQuoteAtoms - n.trackedQuoteAtoms;
  const db = n.actualBaseAtoms - n.trackedBaseAtoms;
  const shortQ = dq < ZERO ? -dq : ZERO;
  const shortB = db < ZERO ? -db : ZERO;
  return {
    navQuoteAtoms,
    surplusQuoteAtoms: dq > ZERO ? dq : ZERO,
    surplusBaseAtoms: db > ZERO ? db : ZERO,
    shortfallQuoteAtoms: shortQ,
    shortfallBaseAtoms: shortB,
    reconciled: shortQ <= toleranceAtoms && shortB <= toleranceAtoms,
  };
}

export function returnBps(fromPpsE12: bigint, toPpsE12: bigint): number {
  if (fromPpsE12 <= ZERO) return 0;
  return Number(((toPpsE12 - fromPpsE12) * BPS_DENOM) / fromPpsE12);
}