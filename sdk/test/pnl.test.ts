import { describe, expect, it } from "vitest";
import { adverseSelection, decomposePnl, lamportsToQuote, markAtOrBefore, spreadCapture, type PnlFill, type PnlMark } from "../src/pnl";

const B = (n: number | string) => BigInt(n);
const t0 = Date.UTC(2026, 10, 4, 0, 0, 0);
const at = (s: number) => new Date(t0 + s * 1000);
const mid = (usd: number) => B(Math.round(usd * 1e9));
const SOL = B(1_000_000_000);

describe("single fill parts", () => {
  const bought: PnlFill = { at: at(0), deskBought: true, baseAtoms: SOL, quoteAtoms: B(149_900_000) };
  it("spread capture: bought 10 cents under a 150.00 mid = +0.10 USDC", () => {
    expect(spreadCapture(bought, mid(150))).toBe(B(100_000));
  });
  it("adverse selection: mid falls to 149.50 after the buy = -0.50 USDC; mirror sale = same size", () => {
    expect(adverseSelection(bought, mid(150), mid(149.5))).toBe(B(-500_000));
    const sold: PnlFill = { at: at(0), deskBought: false, baseAtoms: SOL, quoteAtoms: B(150_100_000) };
    expect(spreadCapture(sold, mid(150))).toBe(B(100_000));
    expect(adverseSelection(sold, mid(150), mid(150.5))).toBe(B(-500_000));
    expect(adverseSelection(sold, mid(150), mid(149.5))).toBe(B(500_000));
  });
});

describe("decomposePnl", () => {
  const marks: PnlMark[] = [
    { at: at(-10), midE12: mid(150) },
    { at: at(5), midE12: mid(150.2) },
    { at: at(60), midE12: mid(149.5) },
    { at: at(300), midE12: mid(149) },
  ];
});
