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
      <MobileHomePage workspace={workspace} signals={signals} selectedItemId="mobile-1" onSelectItem={() => undefined} onObserve={() => undefined} onStop={() => undefined} onOpenDetail={() => undefined} t={createTranslator("en")} />,
    );
    expect(html).toContain("aria-label=\"Intelligence feed\"");
    expect(html).toContain("aria-label=\"Mobile live view\"");
    expect(html).toContain("aria-label=\"Related signals\"");
    expect(html).toContain("ADB unavailable");
    expect(html).toContain("BTC linked signal");
    expect(html).toContain("Stop session");
  });
});
