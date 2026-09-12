import { describe, expect, it } from "vitest";
import { en } from "./en";
import { availableLocales, createTranslator, resolveLocale } from "./index";
import { zhCN } from "./zh-CN";

describe("i18n", () => {
  it("uses English as the default complete locale", () => {
    const t = createTranslator();
    expect(availableLocales).toEqual(["en", "zh-CN"]);
    expect(t("nav.overview")).toBe("Overview");
    expect(t("agent.mockBadge")).toBe("Mock Model");
  });

  it("provides a complete Simplified Chinese dictionary", () => {
    expect(Object.keys(zhCN).sort()).toEqual(Object.keys(en).sort());

    const t = createTranslator("zh-CN");
    expect(t("common.settings")).toBe("设置");
    expect(t("signal.evidenceQuality")).toBe("证据质量");
    expect(t("models.capability.toolCalling")).toBe("工具调用");
    expect(t("approvals.paperWarning")).toContain("模拟交易提案");
    expect(t("settings.simplifiedChinese")).toBe("简体中文");
    expect(t("signal.minutesAgo", { count: 8 })).toBe("8 分钟前");
  });

  it("keeps English as the default for empty or unsupported preferences", () => {
    expect(resolveLocale(null)).toBe("en");
    expect(resolveLocale("fr")).toBe("en");
    expect(resolveLocale("zh-CN")).toBe("zh-CN");
  });

  it("returns a visible marker for unknown keys", () => {
    const t = createTranslator();
    expect(t("missing.key")).toBe("[missing.key]");
  });
});
