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