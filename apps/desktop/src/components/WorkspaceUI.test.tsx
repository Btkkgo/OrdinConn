import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AppShell } from "./AppShell";
import { Alert, MetricValue, metricColorMap } from "./WorkspaceUI";
import { createTranslator } from "../i18n";

const t = createTranslator("zh-CN");
describe("shared desktop presentation", () => {
  it("keeps the same shell and navigation on every page, with one active item and no logo", () => {
    for (const page of ["home", "warehouse", "settings"] as const) {
      const html = renderToStaticMarkup(<AppShell page={page} onNavigate={() => undefined} workspace={<p>content</p>} t={t} />);
      expect(html).toContain('class="app-shell mobile-shell"');
      expect((html.match(/aria-current="page"/g) ?? [])).toHaveLength(1);
      expect((html.match(/class="nav-item/g) ?? [])).toHaveLength(3);
      expect(html).not.toContain("<img");
      expect(html.indexOf("首页")).toBeLessThan(html.indexOf("资料库"));
      expect(html.indexOf("资料库")).toBeLessThan(html.indexOf("设置"));
    }
  });
  it("preserves exact values including zero and large counts with stable semantic tones", () => {
    expect(metricColorMap).toEqual({ total: "accent", news: "info", stocks: "success", chat: "purple", feedback: "warning", other: "neutral", warning: "warning", error: "danger" });
    for (const value of [0, 1, 10, 999, 10000, 1000000]) {
      const html = renderToStaticMarkup(<MetricValue value={value} tone="info" />);
      expect(html).toContain(`>${value.toLocaleString("en-US")}<`);
      expect(html).toContain("tone-info");
    }
  });
  it("keeps failure details escaped and collapsed behind localized alert copy", () => {
    const html = renderToStaticMarkup(<Alert title={t("models.connectionFailed")} description={t("models.requestRejected")} detail="provider error: <rejected>" t={t} />);
    expect(html).toContain('role="alert"');
    expect(html).toContain("测试连接失败");
    expect(html).toContain("Provider 拒绝了当前请求。");
    expect(html).toContain("<details><summary>查看详情</summary>");
    expect(html).toContain("provider error: &lt;rejected&gt;");
    expect(html).not.toContain("<details open");
  });
});
