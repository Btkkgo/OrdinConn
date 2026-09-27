import { expect, it } from "vitest";
import { isMobileGoalStatus, isMobileStepStatus, isMobileGoalErrorCode } from "./index";
import type { MobileGoalDto, ExpectedStepResultDto, MobileExecutionReferencesDto } from "./index";

it("rejects arbitrary goal and step states at the contract boundary", () => {
  expect(isMobileGoalStatus("PENDING")).toBe(true);
  expect(isMobileGoalStatus("auto_running")).toBe(false);
  expect(isMobileStepStatus("VERIFIED")).toBe(true);
  expect(isMobileStepStatus("done")).toBe(false);
});
it("uses typed error codes rather than parsing UI text", () => {
  expect(isMobileGoalErrorCode("MODEL_NOT_CONFIGURED")).toBe(true);
  expect(isMobileGoalErrorCode("INTERRUPTED_BY_RESTART")).toBe(true);
  expect(isMobileGoalErrorCode("provider missing")).toBe(false);
});
it("preserves original objective and nullable references in DTO serialization", () => {
  const goal: MobileGoalDto = { id: "mobile_goal_019a0123-4567-7000-8000-000000000001", objective: " original ", status: "PENDING", createdAt: "2026-09-27T00:00:00Z", updatedAt: "2026-09-27T00:00:00Z", activePlanId: null, stepBudget: { maxSteps: 8, maxRuntimeMs: 120000, maxConsecutiveFailures: 2, maxIdenticalObservations: 3 }, consecutiveFailureCount: 0, identicalObservationCount: 0 };
  const wire = JSON.parse(JSON.stringify(goal));
  expect(wire.objective).toBe(" original ");
  expect(wire.activePlanId).toBeNull();
  expect(wire.status).toBe("PENDING");
});
it("keeps verification rules typed and trace references independent of large captures", () => {
  const expected: ExpectedStepResultDto = { kind: "TEXT_EQUALS", elementRef: "@e1", value: "wifi" };
  const refs: MobileExecutionReferencesDto = { goalId: "g", planId: "p", stepId: "s", observationId: "o", actionId: "a", evidenceId: "e" };
  expect(JSON.parse(JSON.stringify(expected))).toEqual({ kind: "TEXT_EQUALS", elementRef: "@e1", value: "wifi" });
  expect(Object.keys(refs).sort()).toEqual(["actionId", "evidenceId", "goalId", "observationId", "planId", "stepId"]);
});
