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

export function adverseSelection(f: PnlFill, mid0: bigint, midH: bigint): bigint {
  const drift = atMid(f.baseAtoms, midH) - atMid(f.baseAtoms, mid0);
  return f.deskBought ? drift : -drift;
}

export function decomposePnl(
  fills: readonly PnlFill[],
  marksIn: readonly PnlMark[],
  costAtoms: bigint,
  opts: { horizons?: number[]; tolSec?: number; headline?: number } = {},
): PnlBreakdown {
  const horizons = opts.horizons ?? [5, 60, 300];
  const headline = opts.headline ?? 60;
  const tol = opts.tolSec ?? 30;
  const marks = [...marksIn].sort((a, b) => a.at.getTime() - b.at.getTime());
  const hz: Horizon[] = horizons.map((s) => ({ seconds: s, fills: 0, markoutAtoms: ZERO, adverseAtoms: ZERO }));
  let marked = 0;
  let headlineFills = 0;
  let spread = ZERO;
  let adverse = ZERO;
  for (const f of fills) {
    const t = f.at.getTime();
    const m0 = markAtOrBefore(marks, t, tol);
    if (!m0) continue;
    marked++;
    const sc = spreadCapture(f, m0.midE12);
    horizons.forEach((s, i) => {
      const mh = markNear(marks, t + s * 1000, tol);
      if (!mh) return;
      const a = adverseSelection(f, m0.midE12, mh.midE12);
      hz[i]!.fills++;
      hz[i]!.adverseAtoms += a;
      hz[i]!.markoutAtoms += sc + a;
      if (s === headline) {
        headlineFills++;
        spread += sc;
        adverse += a;
      }
    });
  }
  return {
    fills: fills.length,
    marked,
    unmarked: fills.length - marked,
    headlineFills,
    spreadCaptureAtoms: spread,
    adverseSelectionAtoms: adverse,
    costAtoms,
    netAtoms: spread + adverse - costAtoms,
    horizons: hz,
    markToleranceSec: tol,
  };
}

export function lamportsToQuote(lamports: bigint, midE12: bigint): bigint {
  return (lamports * midE12) / E12;
}
