import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { MobileWorkspaceDto, SignalDto } from "@ordinconn/contracts";
import { createTranslator } from "../i18n";
import { MobileHomePage } from "./MobileHomePage";

const workspace: MobileWorkspaceDto = {
  runtimeStatus: "unavailable", adbStatus: "missing", androidEnvironment: { sdkStatus: "missing", adbStatus: "missing", emulatorStatus: "missing", availableAvds: [], onlineDevices: [] }, observations: [], warehouse: [], strategies: [],
  settings: { allowedApps: [], screenshotRetention: "memory_only", textScale: 100, researchBudget: { maxDurationSeconds: 300, maxSteps: 40, maxScrolls: 12, maxPages: 20, maxObservations: 50, maxModelCalls: 10 } },
  feed: [{
    id: "mobile-1", sourceMethod: "MOBILE", sourceApp: "X", title: "BTC mobile observation", summary: "ETF flow",
    observedAt: "2026-09-20T02:00:00Z", dataType: "mobile_observation", evidenceStatus: "observation_only",
    confidence: 1, assets: ["BTC"], favorite: false, saved: false, officialSource: false,
    hasContradiction: false, mobileObservationId: "mobile-1", evidenceIds: [], relatedSignalIds: ["signal-1"],
  }],
};
const signals = [{ id: "signal-1", title: "BTC linked signal", asset: "BTC" }] as SignalDto[];

describe("mobile home workspace", () => {
  it("renders feed, live mobile state, and related signals as linked columns", () => {
    const html = renderToStaticMarkup(
      <MobileHomePage workspace={workspace} signals={signals} selectedItemId="mobile-1" onSelectItem={() => undefined} onObserve={() => undefined} onStop={() => undefined} onAction={async () => undefined} onOpenDetail={() => undefined} t={createTranslator("en")} />,
    );
    expect(html).toContain("aria-label=\"Intelligence feed\"");
    expect(html).toContain("aria-label=\"Mobile live view\"");
    expect(html).toContain("aria-label=\"Related signals\"");
    expect(html).toContain("ADB unavailable");
    expect(html).toContain("BTC linked signal");
    expect(html).toContain("Stop session");
  });

  it("renders only manual M2 controls and a sanitized receipt summary", () => {
    const active = {
      ...workspace,
      runtimeStatus: "observing" as const,
      adbStatus: "ready" as const,
      session: {
        sessionId: "session-1", deviceId: "emulator-5554", platform: "android" as const,
        deviceType: "emulator" as const, osVersion: "16", screenWidth: 1080, screenHeight: 2400,
        connectedAt: new Date().toISOString(), currentApp: "com.android.settings", currentActivity: ".Settings",
        status: "connected" as const, lastObservationAt: new Date().toISOString(),
      },
      uiSnapshot: {
        snapshotId: "snapshot-1", sessionId: "session-1", packageName: "com.android.settings", activity: ".Settings",
        screenWidth: 1080, screenHeight: 2400, capturedAt: new Date().toISOString(), elements: [], redactions: [],
      },
      settings: { ...workspace.settings, allowedApps: ["com.android.settings"] },
      latestActionReceipt: {
        actionId: "action-1", sessionId: "session-1", snapshotId: "snapshot-1", target: { kind: "back" as const },
        decision: { outcome: "allowed" as const }, status: "executed" as const,
        requestedAt: new Date().toISOString(), completedAt: new Date().toISOString(), prePackage: "com.android.settings",
        preActivity: ".Settings", preFrameHash: "sha256:before", preUiTreeHash: "sha256:before",
        postSnapshotId: "snapshot-2", postPackage: "com.android.settings", postActivity: ".Settings", postFrameHash: "sha256:after",
        postUiTreeHash: "sha256:after", verification: "VERIFIED" as const, commandSent: true,
      },
    } satisfies MobileWorkspaceDto;
    const html = renderToStaticMarkup(
      <MobileHomePage workspace={active} signals={signals} onSelectItem={() => undefined} onObserve={() => undefined} onStop={() => undefined} onOpenDetail={() => undefined} onAction={async () => undefined} t={createTranslator("en")} />,
    );
    expect(html).toContain("Tap selected");
    expect(html).toContain("Swipe up");
    expect(html).toContain("Open allowed app");
    expect(html).toContain("Last action");
    expect(html).toContain("Action");
    expect(html).toContain("Target");
    expect(html).toContain("Policy");
    expect(html).toContain("Executed");
    expect(html).toContain("Pre snapshot");
    expect(html).toContain("Post snapshot");
    expect(html).toContain("snapshot-2");
    expect(html).toContain("Timestamp");
    expect(html).not.toContain("Auto navigate");
    const pending = {
      ...active,
      latestActionReceipt: {
        ...active.latestActionReceipt,
        decision: { outcome: "pending" as const },
        status: "pending" as const,
        commandSent: false,
        verification: null,
      },
    } satisfies MobileWorkspaceDto;
    const pendingHtml = renderToStaticMarkup(
      <MobileHomePage workspace={pending} signals={signals} onSelectItem={() => undefined} onObserve={() => undefined} onStop={() => undefined} onOpenDetail={() => undefined} onAction={async () => undefined} t={createTranslator("en")} />,
    );
    expect(pendingHtml).toContain("Pending — result unknown");
    expect(pendingHtml).toContain("Unknown · pending");
    expect(pendingHtml).not.toContain("undefined");
  });
});
