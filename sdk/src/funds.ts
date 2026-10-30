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