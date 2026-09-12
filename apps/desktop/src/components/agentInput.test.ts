import { describe, expect, it } from "vitest";
import { shouldSendOnKeyDown } from "./agentInput";

const ready = {
  key: "Enter",
  shiftKey: false,
  isComposing: false,
  busy: false,
  value: "Explain this signal",
};

describe("Agent input keyboard behavior", () => {
  it("sends a non-empty message on Enter", () => {
    expect(shouldSendOnKeyDown(ready)).toBe(true);
  });

  it("keeps Shift + Enter available for a newline", () => {
    expect(shouldSendOnKeyDown({ ...ready, shiftKey: true })).toBe(false);
  });

  it("does not send while an IME composition is active", () => {
    expect(shouldSendOnKeyDown({ ...ready, isComposing: true })).toBe(false);
  });

  it("does not send empty input", () => {
    expect(shouldSendOnKeyDown({ ...ready, value: "  \n " })).toBe(false);
  });

  it("does not submit again while the Agent is busy", () => {
    expect(shouldSendOnKeyDown({ ...ready, busy: true })).toBe(false);
  });
});
