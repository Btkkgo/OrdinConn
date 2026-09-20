import { describe, expect, it } from "vitest";
import {
  EVENT_FAMILIES,
  MOBILE_RUNTIME_STATUSES,
  MOBILE_VERIFICATION_RESULTS,
  isApprovalStatus,
  isSignalStatus,
  parseRuntimeEventEnvelope,
} from "./index";
import type { MobileWorkspaceDto } from "./index";

describe("IPC contracts", () => {
  it("rejects unknown signal and approval states", () => {
    expect(isSignalStatus("watching")).toBe(true);
    expect(isSignalStatus("secretly_executed")).toBe(false);
    expect(isApprovalStatus("approved")).toBe(true);
    expect(isApprovalStatus("auto_approved")).toBe(false);
  });

  it("accepts a stable runtime event envelope", () => {
    expect(
      parseRuntimeEventEnvelope({
        id: "evt-1",
        family: "agent",
        type: "agent.message_completed",
        aggregateId: "thread-1",
        occurredAt: "2026-09-12T03:00:00Z",
        payload: { itemId: "item-1" },
      }),
    ).toEqual({
      id: "evt-1",
      family: "agent",
      type: "agent.message_completed",
      aggregateId: "thread-1",
      occurredAt: "2026-09-12T03:00:00Z",
      payload: { itemId: "item-1" },
    });
  });

  it("rejects malformed event envelopes", () => {
    expect(() =>
      parseRuntimeEventEnvelope({
        id: "evt-1",
        family: "private-module-event",
        type: "something",
      }),
    ).toThrow("Invalid runtime event envelope");
  });

  it("defines an observe-only mobile workspace without action capabilities", () => {
    const workspace = {
      runtimeStatus: "observing",
      adbStatus: "ready",
      androidEnvironment: {
        sdkStatus: "detected",
        adbStatus: "ready",
        emulatorStatus: "ready",
        sdkRoot: "~/Library/Android/sdk",
        adbPath: "~/Library/Android/sdk/platform-tools/adb",
        emulatorPath: "~/Library/Android/sdk/emulator/emulator",
        sdkmanagerPath: "~/Library/Android/sdk/cmdline-tools/latest/bin/sdkmanager",
        avdmanagerPath: "~/Library/Android/sdk/cmdline-tools/latest/bin/avdmanager",
        adbVersion: "Android Debug Bridge version 1.0.41",
        availableAvds: [{ name: "Pixel_9_API_36", status: "running", deviceProfile: "pixel_9", architecture: "arm64-v8a", running: true }],
        onlineDevices: [{ id: "emulator-5554", status: "device", model: "sdk_gphone64_arm64", avdName: "Pixel_9_API_36" }],
      },
      session: {
        sessionId: "session-1",
        deviceId: "emulator-5554",
        platform: "android",
        deviceType: "emulator",
        osVersion: "16",
        screenWidth: 1080,
        screenHeight: 2400,
        connectedAt: "2026-09-20T10:00:00Z",
        currentApp: "Example",
        currentActivity: "com.example/.MainActivity",
        status: "connected",
        lastObservationAt: "2026-09-20T10:00:01Z",
      },
      uiSnapshot: {
        snapshotId: "snapshot-1",
        sessionId: "session-1",
        packageName: "com.example",
        activity: "com.example/.MainActivity",
        screenWidth: 1080,
        screenHeight: 2400,
        capturedAt: "2026-09-20T10:00:01Z",
        elements: [{ ref: "@e1", text: "BTC ETF", role: "text", className: "android.widget.TextView", bounds: { x: 20, y: 40, width: 300, height: 80 }, clickable: false, scrollable: false, enabled: true, focused: false, selected: false, resourceId: "com.example:id/title", extractionSource: "accessibility", confidence: 1 }],
      },
      frame: { frameId: "frame-1", sessionId: "session-1", timestamp: "2026-09-20T10:00:01Z", width: 1080, height: 2400, orientation: "portrait", frameHash: "sha256:frame", dataUrl: "data:image/png;base64,AA==" },
      observations: [{ id: "observation-1", deviceSessionId: "session-1", appId: "Example", packageName: "com.example", activity: "com.example/.MainActivity", screenState: "feed", observedAt: "2026-09-20T10:00:01Z", frameHash: "sha256:frame", uiTreeHash: "sha256:tree", sourceLocator: "android://com.example/.MainActivity", visibleFacts: ["BTC ETF"], extractedEntities: ["BTC"], extractionMethod: "accessibility", extractionConfidence: 1, redactions: [], privacyClass: "public", verificationStatus: "VERIFIED", evidenceStatus: "observation_only", metadata: {} }],
      feed: [],
      warehouse: [],
      strategies: [],
      settings: { allowedApps: ["com.example"], screenshotRetention: "memory_only", researchBudget: { maxDurationSeconds: 300, maxSteps: 20, maxScrolls: 10, maxPages: 5, maxObservations: 20, maxModelCalls: 5 }, textScale: 100 },
    } satisfies MobileWorkspaceDto;

    expect(workspace.observations[0].evidenceStatus).toBe("observation_only");
    expect("actions" in workspace).toBe(false);
    expect(MOBILE_RUNTIME_STATUSES).toContain("observing");
    expect(MOBILE_VERIFICATION_RESULTS).toContain("FINANCIAL_ACTION_BLOCKED");
  });

  it("accepts mobile events through the shared event envelope", () => {
    expect(EVENT_FAMILIES).toContain("mobile");
    expect(parseRuntimeEventEnvelope({
      id: "evt-mobile-1",
      family: "mobile",
      type: "mobile.snapshot",
      aggregateId: "session-1",
      occurredAt: "2026-09-20T10:00:01Z",
      payload: { snapshotId: "snapshot-1", elementCount: 1 },
    }).family).toBe("mobile");
  });
});
