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

export function markNear(marks: readonly PnlMark[], t: number, tolSec: number): PnlMark | null {
  let lo = 0;
  let hi = marks.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (marks[mid]!.at.getTime() < t) lo = mid + 1;
    else hi = mid;
  }
  let best: PnlMark | null = null;
  let bestD = Infinity;
  for (const i of [lo - 1, lo]) {
    const m = marks[i];
    if (!m) continue;
    const d = Math.abs(m.at.getTime() - t);
    if (d < bestD) {
      best = m;
      bestD = d;
    }
  }
  return best && bestD <= tolSec * 1000 ? best : null;
}

export function markAtOrBefore(marks: readonly PnlMark[], t: number, tolSec: number): PnlMark | null {
  let best: PnlMark | null = null;
  for (const m of marks) {
    const at = m.at.getTime();
    if (at > t) break;
    best = m;
  }
  return best && t - best.at.getTime() <= tolSec * 1000 ? best : null;
}

const atMid = (base: bigint, midE12: bigint) => (base * midE12) / E12;

export function spreadCapture(f: PnlFill, mid0: bigint): bigint {
  const fair = atMid(f.baseAtoms, mid0);
  return f.deskBought ? fair - f.quoteAtoms : f.quoteAtoms - fair;
}