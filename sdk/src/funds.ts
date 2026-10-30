export type FundClass = "CUSTOMER" | "SERVICE" | "CREATOR_FEE" | "TEAM";

export type FlowPurpose =
  | "DEPOSIT"
  | "WITHDRAWAL"
  | "PERF_FEE"
  | "CREATOR_FEE_CLAIM"
  | "TEAM_FUNDING"
  | "STAKER_REWARD"
  | "BUYBACK"
  | "OPEX"
  | "TEAM_RETAINED";

export class FundClassViolation extends Error {
  constructor(message: string) {
    super(message);
    this.name = "FundClassViolation";
  }
}

export interface FlowShape {
  fundClass: FundClass;
  toClass: FundClass | null;
  purpose: FlowPurpose;
  amountAtoms: bigint;
}

type Rule = { from: readonly FundClass[]; to: FundClass | null };

export const FLOW_RULES: Readonly<Record<FlowPurpose, Rule>> = {
  DEPOSIT: { from: ["CUSTOMER"], to: null },
  WITHDRAWAL: { from: ["CUSTOMER"], to: null },
  PERF_FEE: { from: ["CUSTOMER"], to: "SERVICE" },
  CREATOR_FEE_CLAIM: { from: ["CREATOR_FEE"], to: null },
  TEAM_FUNDING: { from: ["TEAM"], to: null },
  STAKER_REWARD: { from: ["SERVICE"], to: null },
  BUYBACK: { from: ["SERVICE", "CREATOR_FEE"], to: null },
  OPEX: { from: ["SERVICE", "CREATOR_FEE", "TEAM"], to: null },
  TEAM_RETAINED: { from: ["SERVICE", "CREATOR_FEE"], to: "TEAM" },
};

export function assertFundFlow(f: FlowShape): void {
  if (f.amountAtoms <= BigInt(0)) throw new FundClassViolation(`${f.purpose}: amount must be positive`);
  if (f.toClass === "CUSTOMER") throw new FundClassViolation(`${f.purpose}: nothing flows into customer assets except a deposit`);
  const rule = FLOW_RULES[f.purpose];
  if (!rule) throw new FundClassViolation(`unknown purpose ${String(f.purpose)}`);
  if (!rule.from.includes(f.fundClass)) {
    throw new FundClassViolation(`${f.purpose} cannot be funded from ${f.fundClass}`);
  }
  if (rule.to !== f.toClass) {
    throw new FundClassViolation(`${f.purpose} from ${f.fundClass} must go to ${rule.to ?? "outside"}, not ${f.toClass ?? "outside"}`);
  }
}

export function buybackFundClass(source: "CREATOR_FEE" | "SERVICE_FEE"): FundClass {
  return source === "CREATOR_FEE" ? "CREATOR_FEE" : "SERVICE";
}
