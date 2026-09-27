import { createRoot } from "react-dom/client";
import { AppShell } from "../components/AppShell";
import { MobileHomePage } from "../pages/MobileHomePage";
import { createTranslator } from "../i18n";
import { type WorkbenchViewModel } from "./model";
import type { MobileWorkspaceDto } from "@ordinconn/contracts";

// Explicit development-only entry point, dynamically imported behind import.meta.env.DEV.
// This module never invokes IPC or writes any production objects.
export function visualFixtureEnabled(development: boolean, search: string): boolean {
  return development && new URLSearchParams(search).get("workbenchFixture") === "1";
}
export function createVisualFixture(): WorkbenchViewModel {
  return {
    mode: "visual_fixture", device: { connected: true, name: "emulator-5554" }, observation: undefined,
    objects: [], selectedMetric: "other", agentState: "executing", approval: "required_for_key_actions",
    total: { id: "total", count: 1284, yesterdayCount: 1083, deltaCount: 201, deltaPercent: 18.6, direction: "up" },
    metrics: [
      { id: "news", count: 326, deltaPercent: 12.4, deltaCount: 36, yesterdayCount: 290, direction: "up" },
      { id: "stocks", count: 412, deltaPercent: 23.1, deltaCount: 77, yesterdayCount: 335, direction: "up" },
      { id: "chat", count: 288, deltaPercent: -4.8, deltaCount: -15, yesterdayCount: 303, direction: "down" },
      { id: "feedback", count: 143, deltaPercent: 31.2, deltaCount: 34, yesterdayCount: 109, direction: "up" },
      { id: "other", count: 115, deltaPercent: 6, deltaCount: 7, yesterdayCount: 108, direction: "up" },
    ],
    plans: [{ id: "visual-plan", title: "客户反馈中出现 18 条重复投诉", summary: "建议聚类并生成问题摘要，关联对应产品与客户。", status: "discovered", priority: "high", sourceObjectIds: [], createdAt: "2026-09-26T10:22:00Z" }],
  };
}
function FixtureScreen() {
  return <div className="fixture-phone-screen"><header><span>18:22</span><span>5G · 91% ▪</span></header><div className="fixture-phone-content"><span>市场资讯</span><p>Agent 正在观察当前页面</p><article><strong>BTC 市场交易量出现明显增长</strong><p>刚刚 · 已捕获</p></article><article><strong>某科技公司发布最新季度指引</strong><p>1 分钟前 · 已结构化</p></article><aside><small>Agent</small><strong>向下滚动并继续采集</strong></aside></div><i/></div>;
}
export function mountVisualFixture() {
  const workspace: MobileWorkspaceDto = {
    runtimeStatus: "observing", adbStatus: "ready", androidEnvironment: { sdkStatus: "detected", adbStatus: "ready", emulatorStatus: "ready", onlineDevices: [], availableAvds: [] },
    observations: [], feed: [], warehouse: [], strategies: [], settings: { allowedApps: [], screenshotRetention: "memory_only", textScale: 100, researchBudget: { maxDurationSeconds: 300, maxSteps: 40, maxScrolls: 12, maxPages: 20, maxObservations: 50, maxModelCalls: 10 } },
  };
  const t = createTranslator("zh-CN");
  createRoot(document.getElementById("root")!).render(<AppShell page="home" onNavigate={() => undefined} t={t} workspace={<MobileHomePage workspace={workspace} signals={[]} onSelectItem={() => undefined} onObserve={() => undefined} onStop={() => undefined} onAction={async () => undefined} onOpenDetail={() => undefined} t={t} visualModel={createVisualFixture()} fixtureScreen={<FixtureScreen/>}/>}/>);
}
