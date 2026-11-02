import { describe, expect, it } from "vitest";
import {
  AccountingError,
  allocateCreatorFee,
  allocateService,
  BOOTSTRAP_SHARES,
  computeNav,
  crystallise,
  mintShares,
  ppsOf,
  redeemShares,
  redeemValue,
  sharesAtPps,
  splitRewards,
  withdraw,
  type Position,
  type VaultState,
} from "../src/accounting";

const B = (n: number | string) => BigInt(n);
const U = B(1_000_000);
const usd = (x: number) => B(Math.round(x * 1e6));
const pps = (x: number) => B(Math.round(x * 1e6)) * B(1_000_000);
const near = (got: bigint, wantUsd: number, tolAtoms: bigint = U) => {
  const d = got - usd(wantUsd);
  expect(d < 0 ? -d : d, `got ${got} want ~${wantUsd} USDC`).toBeLessThanOrEqual(tolAtoms);
};

const ppsHeld = (before: bigint, after: bigint, totalShares: bigint) => {
  const slack = (B(3) * B("1000000000000")) / totalShares + B(2);
  expect(after >= before - B(1), `pps fell ${before} -> ${after}`).toBe(true);
  expect(after - before <= slack, `pps rose ${before} -> ${after} beyond ${slack}`).toBe(true);
};

function singleHwmFee(shares: bigint, epochPps: bigint, vaultHwm: bigint): bigint {
  const gain = epochPps > vaultHwm ? epochPps - vaultHwm : B(0);
  const num = B(1000) * gain * shares;
  const den = B(10000) * B("1000000000000");
  return num === B(0) ? B(0) : (num - B(1)) / den + B(1);
}

describe("V1-V8 one vault, in sequence", () => {
  let s: VaultState = { totalAssets: B(0), totalShares: B(0) };
  let A: Position;
  let Bp: Position;
  let beforeV6Crank: { s: VaultState; B: Position; epochPps: bigint };
  let feeBInCrank: bigint;

  it("V1 first deposit: shares = deposit - BOOTSTRAP, pps 1.0, hwm 1.0", () => {
    const m = mintShares(usd(120_000), s);
    expect(m.sharesMinted).toBe(usd(120_000) - BOOTSTRAP_SHARES);
    expect(m.deadShares).toBe(BOOTSTRAP_SHARES);
    s = m.state;
    expect(s.totalAssets).toBe(usd(120_000));
    expect(ppsOf(s)).toBe(pps(1));
    A = { shares: m.sharesMinted, hwmPpsE12: ppsOf(s) };
    expect(A.hwmPpsE12).toBe(pps(1));
  });
});

describe("V9-V15 independent", () => {
});

describe("service / creator / staker allocation (ECONOMICS rev2 examples)", () => {
});

