import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

const styles = readFileSync(new URL("./styles.css", import.meta.url), "utf8");

describe("readable typography contract", () => {
  it("defines the approved cross-platform font and type-scale tokens", () => {
    expect(styles).toContain("--font-ui:");
    expect(styles).toContain("\"PingFang SC\"");
    expect(styles).toContain("\"Microsoft YaHei\"");
    expect(styles).toContain("--font-mono:");
    expect(styles).toContain("--text-xs: 12px");
    expect(styles).toContain("--text-3xl: 32px");
  });

  it("keeps rendered interface text at or above the 12px readability floor", () => {
    const sizes = [...styles.matchAll(/font-size:\s*([\d.]+)px/g)].map((match) => Number(match[1]));
    expect(Math.min(...sizes)).toBeGreaterThanOrEqual(12);
  });

  it("does not depend on absent display fonts or blur text containers", () => {
    expect(styles).not.toMatch(/font(?:-family|:)\s*[^;]*(Inter|Manrope)/);
    expect(styles).not.toContain("text-rendering: geometricPrecision");
    expect(styles).not.toMatch(/\.signal-card:hover\s*{[^}]*transform/);
    expect(styles).not.toMatch(/\.message\.typing\s*{[^}]*opacity/);
  });
});
