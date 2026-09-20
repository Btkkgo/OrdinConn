import { describe, expect, it } from "vitest";
import { normalizeTextScale } from "./preferences";

describe("text scale preferences", () => {
  it("accepts only the four supported scales", () => {
    expect([90, 100, 110, 120].map(normalizeTextScale)).toEqual([90, 100, 110, 120]);
    expect(normalizeTextScale(105)).toBe(100);
    expect(normalizeTextScale(Number.NaN)).toBe(100);
  });
});
