import { renderToStaticMarkup } from "react-dom/server";
import { expect, it, vi } from "vitest";
import { submitCommandDraft } from "./AgentCommandPanel";
import { WorkbenchCommandRunner } from "./commands";
import { AgentPlanPanel } from "./AgentPlanPanel";
import { createTranslator } from "../i18n";
import { workbenchRuntimeError } from "./useWorkbenchCommands";

it("model configuration blockers use existing localized Settings guidance", () => {
  const key = workbenchRuntimeError({ code: "MODEL_NOT_CONFIGURED", message: "backend message" });
  expect(createTranslator("en")(key)).toContain("Settings");
  expect(createTranslator("zh-CN")(key)).toContain("设置");
  expect(workbenchRuntimeError({ code: "USER_STOPPED" })).toBe("workbench.stopped");
});

it("Enter preserves the original objective instead of normalizing it", () => {
  const submit = vi.fn();
  expect(submitCommandDraft("  向下查看并提取新增信息  ", submit)).toBe(true);
  expect(submit).toHaveBeenCalledWith("  向下查看并提取新增信息  ");
});

it("submission returns the canonical pending goal without observation, action, or a fabricated plan", async () => {
  const goal = { id: "mobile_goal_019a0123-4567-7000-8000-000000000001", objective: " original ", status: "PENDING", createdAt: "2026-09-27T00:00:00Z", updatedAt: "2026-09-27T00:00:00Z", stepBudget: { maxSteps: 8, maxRuntimeMs: 120000, maxConsecutiveFailures: 2, maxIdenticalObservations: 3 }, consecutiveFailureCount: 0, identicalObservationCount: 0 };
  const runtime = { createGoal: vi.fn(async () => goal), createTask: vi.fn(async () => ({ id: "old-task", query: "original", status: "pending" })), observe: vi.fn(), action: vi.fn(), stop: vi.fn() };
  const result = await new WorkbenchCommandRunner(runtime as never).submitGoal(" original ");
  expect(result).toEqual(goal);
  expect(runtime.createGoal).toHaveBeenCalledWith(" original ");
  expect(runtime.createTask).not.toHaveBeenCalled();
  expect(runtime.observe).not.toHaveBeenCalled();
  expect(runtime.action).not.toHaveBeenCalled();
});

it("a saved goal without a plan displays waiting for planning in the existing panel", () => {
  const html = renderToStaticMarkup(<AgentPlanPanel plans={[]} goals={[{ id: "goal-1", objective: "Read public page", status: "PENDING" } as never]} t={createTranslator("en")} />);
  expect(html).toContain("Read public page");
  expect(html).toContain("Waiting for planning");
  expect(html).not.toContain('data-plan-status="proposed"');
});

it("contract metadata events do not claim that a device is being observed", async () => {
  const { reduceMobileWorkspaceEvent } = await import("../runtime/state");
  const workspace = { runtimeStatus: "disconnected" } as never;
  expect(reduceMobileWorkspaceEvent(workspace, { id: "event-1", family: "mobile", type: "mobile.goal_created", aggregateId: "goal-1", occurredAt: "2026-09-27T00:00:00Z", payload: { goalId: "goal-1" } })).toBe(workspace);
});

it("a planning goal with a persisted plan reports waiting for execution without a new button", () => {
  const html = renderToStaticMarkup(<AgentPlanPanel plans={[]} goals={[{ id: "goal-ready", objective: "Read public page", status: "PLANNING", activePlanId: "plan-1" } as never]} t={createTranslator("en")} />);
  expect(html).toContain("Plan ready. Waiting for execution.");
  expect(html).not.toContain("<button");
});

it("existing plan cards display persisted execution states without adding controls", () => {
  const cases = [["EXECUTING", "Executing step"], ["VERIFIED", "Step verified"], ["FAILED", "Step failed"], ["WAITING_APPROVAL", "Waiting for approval"]] as const;
  for (const [status, label] of cases) {
    const html = renderToStaticMarkup(<AgentPlanPanel plans={[]} goals={[{ id: "goal-executor", objective: "Read public page", status: "RUNNING", activePlanId: "plan-1" } as never]} goalPlans={{ "goal-executor": { plan: { id: "plan-1" }, steps: [{ id: "step-1", status }] } as never }} t={createTranslator("en")} />);
    expect(html).toContain(label);
    expect(html).not.toContain("<button");
  }
});

it("executor audit metadata does not pretend a device observation occurred", async () => {
  const { reduceMobileWorkspaceEvent } = await import("../runtime/state");
  const workspace = { runtimeStatus: "disconnected" } as never;
  for (const type of ["mobile.executor_step_claimed", "mobile.executor_verify_started", "mobile.executor_step_failed", "mobile.model_call_reserved", "mobile.approval_requested", "mobile.completion_verified", "mobile.planner_started"]) {
    expect(reduceMobileWorkspaceEvent(workspace, { id: "executor-event", family: "mobile", type, aggregateId: "goal-1", occurredAt: "2026-09-27T00:00:00Z", payload: { goalId: "goal-1" } })).toBe(workspace);
  }
});
