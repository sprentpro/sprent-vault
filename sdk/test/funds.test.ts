import { describe, expect, it } from "vitest";
import { assertFundFlow, buybackFundClass, FLOW_RULES, FundClassViolation, type FlowShape } from "../src/funds";

const one = BigInt(1_000_000);
const f = (fundClass: FlowShape["fundClass"], purpose: FlowShape["purpose"], toClass: FlowShape["toClass"] = null): FlowShape => ({ fundClass, purpose, toClass, amountAtoms: one });

describe("assertFundFlow", () => {
  it.each([
    ["customer buyback", f("CUSTOMER", "BUYBACK")],
    ["customer staker reward", f("CUSTOMER", "STAKER_REWARD")],
    ["customer opex", f("CUSTOMER", "OPEX")],
    ["customer to team", f("CUSTOMER", "TEAM_RETAINED", "TEAM")],
    ["customer perf fee to team (skipping S)", f("CUSTOMER", "PERF_FEE", "TEAM")],
    ["team buyback dressed as income", f("TEAM", "BUYBACK")],
    ["creator fee as staker reward", f("CREATOR_FEE", "STAKER_REWARD")],
    ["top-up into customer assets", f("TEAM", "TEAM_FUNDING", "CUSTOMER")],
    ["service into customer", f("SERVICE", "OPEX", "CUSTOMER")],
  ])("rejects %s", (_name, flow) => {
    expect(() => assertFundFlow(flow)).toThrow(FundClassViolation);
  });

  it.each([
    ["deposit", f("CUSTOMER", "DEPOSIT")],
    ["withdrawal", f("CUSTOMER", "WITHDRAWAL")],
    ["perf fee to service", f("CUSTOMER", "PERF_FEE", "SERVICE")],
    ["service buyback", f("SERVICE", "BUYBACK")],
    ["creator fee buyback", f("CREATOR_FEE", "BUYBACK")],
    ["service staker reward", f("SERVICE", "STAKER_REWARD")],
    ["team opex", f("TEAM", "OPEX")],
    ["service retained to team", f("SERVICE", "TEAM_RETAINED", "TEAM")],
  ])("allows %s", (_name, flow) => {
    expect(() => assertFundFlow(flow)).not.toThrow();
  });

  it("rejects zero and negative amounts", () => {
    expect(() => assertFundFlow({ ...f("SERVICE", "BUYBACK"), amountAtoms: BigInt(0) })).toThrow(FundClassViolation);
    expect(() => assertFundFlow({ ...f("SERVICE", "BUYBACK"), amountAtoms: BigInt(-1) })).toThrow(FundClassViolation);
  });

  it("no purpose lists CUSTOMER as a source except deposit/withdrawal/perf fee", () => {
    const customerPurposes = Object.entries(FLOW_RULES).filter(([, r]) => r.from.includes("CUSTOMER")).map(([p]) => p).sort();
    expect(customerPurposes).toEqual(["DEPOSIT", "PERF_FEE", "WITHDRAWAL"]);
  });
});
