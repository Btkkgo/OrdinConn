import { describe, expect, it } from "vitest";
import { availableLocales, createTranslator } from "./index";

describe("i18n", () => {
  it("uses English as the default complete locale", () => {
    const t = createTranslator();
    expect(availableLocales).toEqual(["en", "zh-CN"]);
    expect(t("nav.overview")).toBe("Overview");
    expect(t("agent.mockBadge")).toBe("Mock Model");
  });

  it("falls back to English for untranslated Chinese keys", () => {
    const t = createTranslator("zh-CN");
    expect(t("common.settings")).toBe("设置");
    expect(t("signal.evidenceQuality")).toBe("Evidence quality");
  });

  it("returns a visible marker for unknown keys", () => {
    const t = createTranslator();
    expect(t("missing.key")).toBe("[missing.key]");
  });
});
