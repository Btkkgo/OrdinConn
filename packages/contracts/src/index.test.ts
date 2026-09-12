import { describe, expect, it } from "vitest";
import {
  isApprovalStatus,
  isSignalStatus,
  parseRuntimeEventEnvelope,
} from "./index";

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
});
