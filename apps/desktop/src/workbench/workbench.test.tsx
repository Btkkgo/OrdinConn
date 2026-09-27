import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { IntelligenceItemDto, MobileActionResultDto, MobileGoalDto, MobileWorkspaceDto } from "@ordinconn/contracts";
import { buildWorkbenchModel, selectMetric, type AgentPlan, type PlanStatus } from "./model";
import { createVisualFixture, visualFixtureEnabled } from "./visualFixture";
import { WorkbenchCommandRunner, type WorkbenchRuntimePort } from "./commands";
import { AgentPlanPanel } from "./AgentPlanPanel";
import { submitCommandDraft } from "./AgentCommandPanel";
import { MobileHomePage } from "../pages/MobileHomePage";
import { createTranslator } from "../i18n";
import { frameSurfaceStyle } from "../components/MobileDeviceView";
import type { MobileFrameDto } from "@ordinconn/contracts";

const now = new Date("2026-09-27T12:00:00+08:00");
const empty: MobileWorkspaceDto = {
  runtimeStatus: "disconnected", adbStatus: "ready", androidEnvironment: { sdkStatus: "detected", adbStatus: "ready", emulatorStatus: "ready", availableAvds: [], onlineDevices: [] },
  observations: [], feed: [], warehouse: [], strategies: [], settings: { allowedApps: ["com.android.settings"], screenshotRetention: "memory_only", textScale: 100, researchBudget: { maxDurationSeconds: 300, maxSteps: 40, maxScrolls: 12, maxPages: 20, maxObservations: 50, maxModelCalls: 10 } },
};
function item(id: string, dataType: string, date = now.toISOString()): IntelligenceItemDto {
  return { id, sourceMethod: "MOBILE", sourceApp: "com.android.settings", title: "Visible fact", summary: "Public data", dataType, observedAt: date, evidenceStatus: "observation_only", confidence: 1, assets: [], favorite: false, saved: false, officialSource: false, hasContradiction: false, evidenceIds: ["evidence-1"], relatedSignalIds: [] };
}
function active(): MobileWorkspaceDto {
  return { ...empty, androidEnvironment: { ...empty.androidEnvironment, onlineDevices: [{ id: "emulator-5570", status: "device" }] },
    session: { sessionId: "s", deviceId: "emulator-5570", platform: "android", deviceType: "emulator", osVersion: "16", screenWidth: 1080, screenHeight: 2400, status: "connected", currentApp: "com.android.settings", connectedAt: new Date().toISOString() },
    uiSnapshot: { snapshotId: "fresh", sessionId: "s", packageName: "com.android.settings", activity: ".Settings", capturedAt: new Date().toISOString(), screenWidth: 1080, screenHeight: 2400, elements: [] },
    observations: [{ id: "observation-1" } as MobileWorkspaceDto["observations"][number]],
  };
}
const t = createTranslator("en");
function render(workspace: MobileWorkspaceDto, visual = false) {
  return renderToStaticMarkup(<MobileHomePage workspace={workspace} signals={[]} onSelectItem={() => undefined} onObserve={() => undefined} onStop={() => undefined} onAction={async () => undefined} onOpenDetail={() => undefined} t={t} visualModel={visual ? createVisualFixture() : undefined}/>);
}
function port(workspace = active()): WorkbenchRuntimePort {
  return { observe: vi.fn(async () => workspace), stop: vi.fn(async () => undefined), createGoal: vi.fn(async objective => ({ id: "goal-1", objective, status: "PENDING", createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(), stepBudget: { maxSteps: 8, maxRuntimeMs: 120000, maxConsecutiveFailures: 2, maxIdenticalObservations: 3 }, consecutiveFailureCount: 0, identicalObservationCount: 0 } as MobileGoalDto)),
    action: vi.fn(async () => ({ workspace, receipt: { status: "executed", verification: "VERIFIED" } } as MobileActionResultDto)) };
}
describe("reference workbench", () => {
  it("fits real portrait and landscape frames with the inspector on the exact image surface", () => {
    expect(frameSurfaceStyle({ width: 1080, height: 2400 } as MobileFrameDto)).toEqual({ width: "80%", height: "100%" });
    expect(frameSurfaceStyle({ width: 1920, height: 1080 } as MobileFrameDto)).toEqual({ width: "100%", height: "31.640625%" });
    expect(frameSurfaceStyle(undefined)).toEqual({ width: "100%", height: "100%" });
  });
  it("renders all regions with no icons or static production demonstration", () => {
    const html = render(empty); expect(html).toContain("Realtime Intelligence Workbench"); expect(html).toContain("Agent commands"); expect(html).toContain("Approval: user confirms key actions"); expect(html).not.toContain("<svg"); expect(html).not.toContain("1,284"); expect(html).not.toContain("18 repeated");
  });
  it("selects a metric and rejects unknown selections", () => {
    const model = buildWorkbenchModel(empty); expect(selectMetric(model, "news").selectedMetric).toBe("news"); expect(selectMetric(model, "missing")).toBe(model);
  });
  it("does not trust an old session when the device is disconnected", () => {
    const workspace = active(); workspace.androidEnvironment.onlineDevices = [];
    expect(buildWorkbenchModel(workspace).device.connected).toBe(false); expect(render(workspace)).toContain("Android disconnected");
  });
  it("reads connected state and device identity from fresh diagnostics", () => {
    const html = render(active()); expect(html).toContain("Android connected"); expect(html).toContain("emulator-5570"); expect(html).not.toContain("emulator-5554");
  });
  it("aggregates dates and explicit categories, deduplicates ids, preserves evidence", () => {
    const a = item("1", "news"); const workspace = { ...empty, feed: [a, a, item("2", "news"), item("3", "news", "2026-09-26T12:00:00+08:00"), item("4", "files")] };
    const model = buildWorkbenchModel(workspace, "news", [], now); expect(model.total.count).toBe(3); expect(model.metrics.find(m => m.id === "news")?.deltaPercent).toBe(100); expect(model.metrics.find(m => m.id === "files")?.count).toBe(1); expect(model.objects[0].evidenceIds).toEqual(["evidence-1"]);
  });
  it("deduplicates repeated visible mobile content while retaining the newest evidence locator", () => {
    const first = { ...item("a", "mobile_observation"), mobileObservationId: "a" };
    const second = { ...item("b", "mobile_observation", new Date(now.getTime() + 1000).toISOString()), mobileObservationId: "b" };
    const observation = { packageName: "com.android.settings", visibleFacts: ["Public setting"] };
    const model = buildWorkbenchModel({ ...empty, feed: [first, second], observations: [{ ...observation, id: "a" }, { ...observation, id: "b" }] as MobileWorkspaceDto["observations"] }, "other", [], now);
    expect(model.total.count).toBe(1); expect(model.objects[0].observationId).toBe("b");
  });
  it("refreshes metrics for a persisted extraction object without merging it into its source observation", () => {
    const observation = { id: "observed", packageName: "com.android.settings", visibleFacts: ["Display"] } as MobileWorkspaceDto["observations"][number];
    const raw = { ...item("raw", "mobile_observation"), mobileObservationId: "observed" };
    const extracted = { ...item("object", "mobile_observation_object"), mobileObservationId: "observed", evidenceIds: ["actual-extraction-evidence"] };
    const before = buildWorkbenchModel({ ...empty, feed: [raw], observations: [observation] }, "other", [], now);
    const after = buildWorkbenchModel({ ...empty, feed: [raw, extracted, extracted], observations: [observation] }, "other", [], now);
    expect(after.total.count).toBe(before.total.count + 1);
    expect(after.metrics.find(metric => metric.id === "other")?.count).toBe(2);
    expect(after.metrics.find(metric => metric.id === "news")?.count).toBe(0);
    expect(after.objects.find(object => object.id === "object")?.evidenceIds).toEqual(["actual-extraction-evidence"]);
  });
  it("does not invent a percentage when yesterday's baseline is missing", () => {
    expect(buildWorkbenchModel({ ...empty, feed: [item("1", "news")] }, "news", [], now).total.deltaPercent).toBeUndefined();
  });
  it("renders the empty plan state", () => { expect(render(empty)).toContain("All current suggestions have been handled"); });
  it.each(["discovered", "proposed", "waiting_approval", "executing", "completed", "failed", "dismissed"] as PlanStatus[])("renders or hides plan state %s without claiming a different outcome", status => {
    const plan: AgentPlan = { id: "p", title: "Evidence-linked plan", summary: "Real result", priority: "high", sourceObjectIds: ["1"], status, createdAt: now.toISOString() };
    const html = renderToStaticMarkup(<AgentPlanPanel plans={[plan]} t={t}/>);
    expect(html.includes(`data-plan-status="${status}"`)).toBe(status !== "dismissed");
  });
  it("isolates visual fixtures behind explicit development opt-in", () => {
    expect(visualFixtureEnabled(false, "?workbenchFixture=1")).toBe(false); expect(visualFixtureEnabled(true, "")).toBe(false); expect(visualFixtureEnabled(true, "?workbenchFixture=1")).toBe(true);
    const html = render(empty, true); expect(html).toContain("VISUAL FIXTURE"); expect(html).toContain("1,284"); expect(render(empty)).not.toContain("VISUAL FIXTURE");
  });
  it("submits original custom input from the same form path used by Enter and rejects blanks", () => {
    const submit = vi.fn(); expect(submitCommandDraft("  collect public data  ", submit)).toBe(true); expect(submit).toHaveBeenCalledWith("  collect public data  "); expect(submitCommandDraft("  ", submit)).toBe(false); expect(submit).toHaveBeenCalledTimes(1);
  });
});
describe("existing runtime command adapter", () => {
  it("Observe calls the real typed runtime port and returns its workspace", async () => {
    const runtime = port(); const result = await new WorkbenchCommandRunner(runtime).execute("observe"); expect(runtime.observe).toHaveBeenCalledTimes(1); expect(result.workspace.session?.deviceId).toBe("emulator-5570"); expect(runtime.action).not.toHaveBeenCalled();
  });
  it("extracts only a real observation and rejects missing extraction", async () => {
    expect((await new WorkbenchCommandRunner(port()).execute("extract")).summaryKey).toBe("workbench.extracted"); await expect(new WorkbenchCommandRunner(port(empty)).execute("extract")).rejects.toThrow("NOT_IMPLEMENTED");
  });
  it("collects through snapshot-bound swipe and requires post-action verification", async () => {
    const runtime = port(); await new WorkbenchCommandRunner(runtime).execute("collect"); expect(runtime.action).toHaveBeenCalledWith({ sessionId: "s", snapshotId: "fresh", expectedPackage: "com.android.settings", target: { kind: "swipe", direction: "up" } });
    vi.mocked(runtime.action).mockResolvedValue({ workspace: active(), receipt: { status: "executed", verification: "NO_CHANGE" } } as MobileActionResultDto); await expect(new WorkbenchCommandRunner(runtime).execute("collect")).rejects.toThrow("NO_CHANGE");
  });
  it("fails closed before swipe on a sensitive screen", async () => {
    const workspace = active(); workspace.uiSnapshot!.sensitiveState = "SENSITIVE_FIELD_BLOCKED"; const runtime = port(workspace);
    await expect(new WorkbenchCommandRunner(runtime).execute("collect")).rejects.toThrow("BLOCKED"); expect(runtime.action).not.toHaveBeenCalled();
  });
  it("Stop interrupts a pending Observe before any later action without killing a device", async () => {
    let finish!: (workspace: MobileWorkspaceDto) => void; const runtime = port(); runtime.observe = vi.fn(() => new Promise<MobileWorkspaceDto>(resolve => { finish = resolve; })); const runner = new WorkbenchCommandRunner(runtime);
    const collecting = runner.execute("collect"); const assertion = expect(collecting).rejects.toThrow("INTERRUPTED"); await runner.stop(); finish(active()); await assertion; expect(runtime.stop).toHaveBeenCalledTimes(1); expect(runtime.action).not.toHaveBeenCalled();
  });
  it("stores a canonical pending goal without claiming execution", async () => {
    const runtime = port(); const goal = await new WorkbenchCommandRunner(runtime).submitGoal("  inspect public screen  "); expect(runtime.createGoal).toHaveBeenCalledWith("  inspect public screen  "); expect(goal.status).toBe("PENDING"); expect(goal.objective).toBe("  inspect public screen  "); expect(runtime.action).not.toHaveBeenCalled();
  });
});
