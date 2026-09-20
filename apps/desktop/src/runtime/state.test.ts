import { describe, expect, it } from "vitest";
import type { IntelligenceItemDto, SignalDto } from "@ordinconn/contracts";
import { createPageContext, initialRuntimeState, reduceMobileWorkspaceEvent, reduceRuntimeEvent } from "./state";

const signal = {
  id: "signal-1", market: "crypto", category: "exchange", asset: "BTC",
  evidence: [{ id: "evidence-1" }],
} as SignalDto;

describe("runtime UI state", () => {
  it("injects active page, market, asset, signal, and evidence context", () => {
    expect(createPageContext("signals", signal)).toEqual({
      page: "signals", market: "crypto", asset: "BTC", signalId: "signal-1", evidenceIds: ["evidence-1"],
    });
  });

  it("normalizes a completed agent message event", () => {
    const next = reduceRuntimeEvent(initialRuntimeState, {
      id: "event-1", family: "agent", type: "agent.message_completed", aggregateId: "thread-1",
      threadId: "thread-1", turnId: "turn-1", occurredAt: "2026-09-12T03:00:00Z",
      payload: { itemId: "item-2", content: "Mock Model: evidence summary", modelId: "mock-model", mock: true },
    });
    expect(next.agentMessages).toHaveLength(1);
    expect(next.agentMessages[0].content).toBe("Mock Model: evidence summary");
    expect(next.activeTurnId).toBeUndefined();
  });

  it("adds the selected mobile observation and source locator to agent context", () => {
    const item = {
      id: "mobile-1",
      mobileObservationId: "mobile-1",
      sourceLocator: "android://com.x/.Main?snapshot=snapshot-1",
      evidenceIds: [],
      assets: ["BTC"],
    } as unknown as IntelligenceItemDto;
    expect(createPageContext("home", undefined, item)).toEqual({
      page: "home",
      market: undefined,
      asset: "BTC",
      signalId: undefined,
      evidenceIds: [],
      mobileObservationId: "mobile-1",
      mobileSourceLocator: "android://com.x/.Main?snapshot=snapshot-1",
    });
  });

  it("merges a mobile event without replacing feed or settings", () => {
    const workspace = { runtimeStatus: "disconnected", feed: [{ id: "item-1" }], settings: { textScale: 100 } } as unknown as import("@ordinconn/contracts").MobileWorkspaceDto;
    const next = reduceMobileWorkspaceEvent(workspace, {
      id: "event-mobile", family: "mobile", type: "mobile.snapshot", aggregateId: "session-1",
      occurredAt: "2026-09-20T03:00:00Z", payload: { frameHash: "sha256:abc" },
    });
    expect(next.runtimeStatus).toBe("observing");
    expect(next.feed).toBe(workspace.feed);
    expect(next.settings).toBe(workspace.settings);
  });
});
