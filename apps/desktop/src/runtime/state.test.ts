import { describe, expect, it } from "vitest";
import type { SignalDto } from "@ordinconn/contracts";
import { createPageContext, initialRuntimeState, reduceRuntimeEvent } from "./state";

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
});
