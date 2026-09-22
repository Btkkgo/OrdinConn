import { describe, expect, it } from "vitest";
import type { MobileDeviceSessionDto, MobileUiSnapshotDto } from "@ordinconn/contracts";
import { mobileActionAvailability } from "./MobileActionControls";

const now = Date.parse("2026-09-22T06:00:00.000Z");
const activeSession: MobileDeviceSessionDto = {
  sessionId: "session-1", deviceId: "emulator-5554", platform: "android", deviceType: "emulator",
  osVersion: "16", screenWidth: 1080, screenHeight: 2400, connectedAt: new Date(now).toISOString(),
  currentApp: "com.android.settings", currentActivity: ".Settings", status: "connected",
  lastObservationAt: new Date(now).toISOString(),
};
const currentSnapshot: MobileUiSnapshotDto = {
  snapshotId: "snapshot-1", sessionId: "session-1", packageName: "com.android.settings",
  activity: ".Settings", screenWidth: 1080, screenHeight: 2400,
  capturedAt: new Date(now - 1_000).toISOString(), elements: [], redactions: [],
};

describe("manual mobile action availability", () => {
  it("allows current allowlisted emulator navigation and escape", () => {
    expect(mobileActionAvailability(activeSession, currentSnapshot, ["com.android.settings"], now))
      .toEqual({ escape: true, navigate: true });
  });

  it("disables controls on stale, physical, mismatched, and unallowlisted states", () => {
    const allowed = ["com.android.settings"];
    expect(mobileActionAvailability(activeSession, { ...currentSnapshot, capturedAt: new Date(now - 11_000).toISOString() }, allowed, now).navigate).toBe(false);
    expect(mobileActionAvailability({ ...activeSession, deviceType: "physical" }, currentSnapshot, allowed, now).escape).toBe(false);
    expect(mobileActionAvailability(activeSession, { ...currentSnapshot, sessionId: "other" }, allowed, now).navigate).toBe(false);
    expect(mobileActionAvailability(activeSession, currentSnapshot, [], now).navigate).toBe(false);
  });

  it("leaves only Back and Home available on a sensitive screen", () => {
    expect(mobileActionAvailability(activeSession, { ...currentSnapshot, sensitiveState: "FINANCIAL_ACTION_BLOCKED" }, ["com.android.settings"], now))
      .toEqual({ escape: true, navigate: false });
  });
});
