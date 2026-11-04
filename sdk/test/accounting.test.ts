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

  it("V2 profit epoch: fee 10% of gain above hwm, paid by burning own shares, pps unchanged", () => {
    s = { ...s, totalAssets: s.totalAssets + usd(12_000) };
    const epoch = ppsOf(s);
    expect(epoch).toBe(pps(1.1));
    const c = crystallise(A, s, epoch);
    expect(c.feeAtoms).toBe(B(1_199_999_990));
    near(c.feeAtoms, 1_200);
    expect(c.feeShares).toBe(B(1_090_909_082));
    near(c.feeShares, 1_091);
    near(c.position.shares, 118_909);
    expect(c.position.hwmPpsE12).toBe(pps(1.1));
    near(c.state.totalAssets, 130_800);
    ppsHeld(epoch, ppsOf(c.state), c.state.totalShares);
    A = c.position;
    s = c.state;
  });

  it("V3 deposit at the high: shares at the epoch price, own hwm = entry pps, pps unchanged", () => {
    const before = ppsOf(s);
    const m = mintShares(usd(55_000), s);
    near(m.sharesMinted, 50_000);
    s = m.state;
    Bp = { shares: m.sharesMinted, hwmPpsE12: before };
    near(s.totalShares, 168_909);
    near(s.totalAssets, 185_800);
    ppsHeld(before, ppsOf(s), s.totalShares);
  });

  it("V4 loss epoch: no fee, hwm unchanged", () => {
    s = { ...s, totalAssets: s.totalAssets - usd(18_580) };
    const epoch = ppsOf(s);
    expect(Number(epoch) / 1e12).toBeCloseTo(0.99, 5);
    const ca = crystallise(A, s, epoch);
    const cb = crystallise(Bp, s, epoch);
    expect(ca.feeAtoms).toBe(B(0));
    expect(cb.feeAtoms).toBe(B(0));
    expect(ca.position.hwmPpsE12).toBe(A.hwmPpsE12);
    expect(cb.position.hwmPpsE12).toBe(Bp.hwmPpsE12);
  });

  it("V5 recovery epoch: no fee on recovered loss", () => {
    s = { ...s, totalAssets: s.totalAssets + usd(18_580) };
    const epoch = ppsOf(s);
    expect(Number(epoch) / 1e12).toBeCloseTo(1.1, 9);
    const ca = crystallise(A, s, epoch);
    const cb = crystallise(Bp, s, epoch);
    expect(ca.feeAtoms).toBe(B(0));
    expect(cb.feeAtoms).toBe(B(0));
    A = ca.position;
    Bp = cb.position;
  });

  it("V6 new profit above hwm: each position pays only on its own gain above 1.10", () => {
    s = { ...s, totalAssets: s.totalAssets + usd(16_890.9) };
    const epoch = ppsOf(s);
    expect(Number(epoch) / 1e12).toBeCloseTo(1.2, 6);
    beforeV6Crank = { s, B: Bp, epochPps: epoch };
    const ca = crystallise(A, s, epoch);
    near(ca.feeAtoms, 1_189.09);
    const cb = crystallise(Bp, ca.state, epoch);
    near(cb.feeAtoms, 500);
    feeBInCrank = cb.feeAtoms;
    ppsHeld(epoch, ppsOf(cb.state), cb.state.totalShares);
    A = ca.position;
    Bp = cb.position;
    s = cb.state;
  });

  it("V7 full withdrawal after crystallisation: no extra fee", () => {
    const epoch = ppsOf(s);
    const w = withdraw(A, s, epoch, "all");
    expect(w.crystallised.feeAtoms).toBe(B(0));
    expect(w.redeemAtoms).toBe(redeemShares(A.shares, s));
    expect(w.redeemAtoms <= redeemValue(A.shares, epoch) + B(1)).toBe(true);
    expect(w.position.shares).toBe(B(0));
    near(w.redeemAtoms, 118_909 * 1.2 - 1_189.09, B(2) * U);
  });

  it("V8 withdrawal before the crank: forced crystallisation, same fee (no escape)", () => {
    const { s: s0, B: b0, epochPps } = beforeV6Crank;
    const w = withdraw(b0, s0, epochPps, "all");
    expect(w.crystallised.feeAtoms).toBe(feeBInCrank);
    near(w.crystallised.feeAtoms, 500);
    expect(w.redeemAtoms).toBe(redeemShares(w.crystallised.position.shares, w.crystallised.state));
  });
});

describe("V9-V15 independent", () => {
  it("V9 entrant after a loss pays on its recovery; earlier holder does not (control: single HWM lets it ride free)", () => {
    let s: VaultState = { totalAssets: usd(80_000), totalShares: usd(100_000) };
    const E: Position = { shares: usd(100_000), hwmPpsE12: pps(1) };
    expect(ppsOf(s)).toBe(pps(0.8));
    const m = mintShares(usd(40_000), s);
    expect(m.sharesMinted).toBe(usd(50_000));
    const D: Position = { shares: m.sharesMinted, hwmPpsE12: ppsOf(s) };
    s = { ...m.state, totalAssets: usd(150_000) };
    const epoch = ppsOf(s);
    expect(epoch).toBe(pps(1));
    const cd = crystallise(D, s, epoch);
    expect(cd.feeAtoms).toBe(usd(1_000));
    const ce = crystallise(E, cd.state, epoch);
    expect(ce.feeAtoms).toBe(B(0));
    expect(singleHwmFee(D.shares, epoch, pps(1))).toBe(B(0));
  });

  it("V10 entry at the high then flat: no fee (control: a stale vault HWM double-charges)", () => {
    const s: VaultState = { totalAssets: usd(120_000), totalShares: usd(100_000) };
    const F: Position = { shares: usd(10_000), hwmPpsE12: pps(1.2) };
    const epoch = ppsOf(s);
    expect(epoch).toBe(pps(1.2));
    expect(crystallise(F, s, epoch).feeAtoms).toBe(B(0));
    expect(singleHwmFee(F.shares, epoch, pps(1.1)) > B(0)).toBe(true);
  });

  it("V11 withdraw and redeposit cannot dodge the fee", () => {
    const s: VaultState = { totalAssets: usd(130_000), totalShares: usd(100_000) };
    const G: Position = { shares: usd(100_000), hwmPpsE12: pps(1) };
    const epoch = ppsOf(s);
    const out = withdraw(G, s, epoch, "all");
    expect(out.crystallised.feeAtoms).toBe(usd(3_000));
    const stay = crystallise(G, s, epoch);
    expect(stay.feeAtoms).toBe(out.crystallised.feeAtoms);
    const back = mintShares(out.redeemAtoms, out.state);
    const G2: Position = { shares: back.sharesMinted, hwmPpsE12: epoch };
    expect(G2.hwmPpsE12).toBe(pps(1.3));
    expect(crystallise(G2, back.state, ppsOf(back.state)).feeAtoms).toBe(B(0));
  });

  it("V12 a direct transfer into the vault is surplus, not NAV: pps unchanged", () => {
    const shares = usd(100_000);
    const before = computeNav({ trackedQuoteAtoms: usd(100_000), trackedBaseAtoms: B(0), actualQuoteAtoms: usd(100_000), actualBaseAtoms: B(0), markPriceE12: B(0) });
    const after = computeNav({ trackedQuoteAtoms: usd(100_000), trackedBaseAtoms: B(0), actualQuoteAtoms: usd(1_100_000), actualBaseAtoms: B(0), markPriceE12: B(0) });
    expect(after.navQuoteAtoms).toBe(before.navQuoteAtoms);
    expect(after.surplusQuoteAtoms).toBe(usd(1_000_000));
    expect(after.reconciled).toBe(true);
    expect(ppsOf({ totalAssets: after.navQuoteAtoms, totalShares: shares })).toBe(ppsOf({ totalAssets: before.navQuoteAtoms, totalShares: shares }));
    const short = computeNav({ trackedQuoteAtoms: usd(100_000), trackedBaseAtoms: B(0), actualQuoteAtoms: usd(99_999), actualBaseAtoms: B(0), markPriceE12: B(0) });
    expect(short.reconciled).toBe(false);
    expect(short.shortfallQuoteAtoms).toBe(usd(1));
  });

  it("V13 a deposit that buys zero shares is refused", () => {
    const s: VaultState = { totalAssets: B("1000000000000000"), totalShares: B(1) };
    expect(() => mintShares(B(1), s)).toThrow(AccountingError);
    try {
      mintShares(B(1), s);
    } catch (e) {
      expect((e as AccountingError).code).toBe("ZERO_SHARES");
    }
    expect(() => sharesAtPps(B(1), B("1000000000000000000000000"))).toThrow(AccountingError);
  });

  it("V14 round trip rounds for the vault: 100 in -> 90 shares -> 99 out", () => {
    const s: VaultState = { totalAssets: B(11_000_000_000), totalShares: B(10_000_000_000) };
    expect(ppsOf(s)).toBe(pps(1.1));
    const m = mintShares(B(100), s);
    expect(m.sharesMinted).toBe(B(90));
    expect(sharesAtPps(B(100), pps(1.1))).toBe(B(90));
    expect(redeemValue(B(90), pps(1.1))).toBe(B(99));
    expect(redeemShares(B(90), m.state)).toBe(B(99));
  });

  it("V15 fee 1234.5 atoms rounds up to 1235, fee shares round up", () => {
    const pos: Position = { shares: B(12_345_000), hwmPpsE12: pps(1) };
    const s: VaultState = { totalAssets: B(100_000_000), totalShares: B(90_000_000) };
    const epoch = pps(1.001);
    const c = crystallise(pos, s, epoch);
    expect(c.feeAtoms).toBe(B(1235));
    expect(c.feeShares).toBe(B(1234));
  });
});

describe("service / creator / staker allocation (ECONOMICS rev2 examples)", () => {
  it("net S 700 -> holders 210 / buyback 70 / team 420", () => {
    const a = allocateService({ grossFeeAtoms: usd(1_000), externalCostAtoms: usd(300), rebateAtoms: B(0), opexReserveAtoms: B(0) });
    expect(a.netDistributableAtoms).toBe(usd(700));
    expect([a.holderAtoms, a.buybackAtoms, a.teamAtoms]).toEqual([usd(210), usd(70), usd(420)]);
    expect(a.holderAtoms + a.buybackAtoms + a.teamAtoms).toBe(a.netDistributableAtoms);
  });

  it("S <= 0 allocates nothing and records the shortfall for the team book", () => {
    const a = allocateService({ grossFeeAtoms: usd(100), externalCostAtoms: usd(220), rebateAtoms: B(0), opexReserveAtoms: B(0) });
    expect([a.netDistributableAtoms, a.holderAtoms, a.buybackAtoms, a.teamAtoms]).toEqual([B(0), B(0), B(0), B(0)]);
    expect(a.shortfallAtoms).toBe(usd(120));
  });

  it("odd amounts: floors to holders and buyback, remainder to team, sum exact", () => {
    const a = allocateService({ grossFeeAtoms: B(1_000_007), externalCostAtoms: B(0), rebateAtoms: B(0), opexReserveAtoms: B(0) });
    expect(a.holderAtoms + a.buybackAtoms + a.teamAtoms).toBe(B(1_000_007));
  });

  it("creator fee 1000 - 20 cost -> buyback 196 / team 784", () => {
    expect(allocateCreatorFee(usd(1_000), usd(20))).toEqual({ netAtoms: usd(980), buybackAtoms: usd(196), teamAtoms: usd(784) });
    expect(() => allocateCreatorFee(usd(1), B(0), B(5001))).toThrow(AccountingError);
  });

  it("time-weighted rewards never exceed the pool; remainder carries", () => {
    const r = splitRewards(B(100), [B(1), B(1), B(1)]);
    expect(r.rewards).toEqual([B(33), B(33), B(33)]);
    expect(r.carryAtoms).toBe(B(1));
  });
});

