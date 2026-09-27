import type { IntelligenceItemDto, MobileWorkspaceDto } from "@ordinconn/contracts";

export const categoryIds = ["news", "stocks", "chat", "feedback", "other"] as const;
export type CategoryId = typeof categoryIds[number];
export type PlanStatus = "discovered" | "proposed" | "waiting_approval" | "executing" | "completed" | "failed" | "dismissed";
export interface IntelligenceDataObject {
  id: string; sourceType: string; category: string; observedAt: string;
  confidence?: number; evidenceIds: string[]; observationId?: string;
}
export interface RealtimeMetric {
  id: string; count: number; yesterdayCount: number; deltaCount: number; deltaPercent?: number;
  direction: "up" | "down" | "flat";
}
export interface AgentPlan {
  id: string; title: string; summary: string; priority: "low" | "medium" | "high";
  sourceObjectIds: string[]; status: PlanStatus; createdAt: string;
  summaryKey?: string; taskId?: string;
}
export interface WorkbenchViewModel {
  device: { connected: boolean; name?: string }; observation: MobileWorkspaceDto["observations"][number] | undefined;
  objects: IntelligenceDataObject[]; metrics: RealtimeMetric[]; total: RealtimeMetric; selectedMetric: string;
  agentState: "waiting" | "observing" | "executing" | "paused" | "disconnected" | "error";
  plans: AgentPlan[]; approval: "required_for_key_actions"; mode: "live" | "visual_fixture";
}
const typeCategories: Record<string, string> = {
  news: "news", rss: "news", article: "news", stock: "stocks", stocks: "stocks", stock_data: "stocks",
  chat: "chat", chat_record: "chat", customer_feedback: "feedback", feedback: "feedback", mobile_observation: "other", mobile_observation_object: "other", evidence: "other",
};
export function toDataObjects(items: IntelligenceItemDto[], observations: MobileWorkspaceDto["observations"] = []): IntelligenceDataObject[] {
  const byObservation = new Map(observations.map(observation => [observation.id, observation]));
  const unique = new Map<string, IntelligenceItemDto>();
  for (const item of [...items].sort((a, b) => b.observedAt.localeCompare(a.observedAt))) {
    const observation = item.mobileObservationId ? byObservation.get(item.mobileObservationId) : undefined;
    const contentKey = item.dataType === "mobile_observation" && observation?.visibleFacts?.length ? `${observation.packageName}:${dayKey(new Date(item.observedAt))}:${JSON.stringify(observation.visibleFacts)}` : item.id;
    if (!unique.has(contentKey)) unique.set(contentKey, item);
  }
  return [...unique.values()].filter(item => item.evidenceStatus !== "rejected").map(item => ({
    id: item.id, sourceType: item.sourceMethod, category: typeCategories[item.dataType] ?? item.dataType ?? "other",
    observedAt: item.observedAt, confidence: item.confidence, evidenceIds: item.evidenceIds, observationId: item.mobileObservationId,
  }));
}
function dayKey(date: Date): string { return `${date.getFullYear()}-${date.getMonth()}-${date.getDate()}`; }
function metric(id: string, count: number, yesterdayCount: number): RealtimeMetric {
  const deltaCount = count - yesterdayCount;
  return { id, count, yesterdayCount, deltaCount, deltaPercent: yesterdayCount ? deltaCount / yesterdayCount * 100 : undefined,
    direction: deltaCount > 0 ? "up" : deltaCount < 0 ? "down" : "flat" };
}
export function buildWorkbenchModel(workspace: MobileWorkspaceDto, selectedMetric = "other", plans: AgentPlan[] = [], now = new Date()): WorkbenchViewModel {
  const objects = toDataObjects(workspace.feed, workspace.observations);
  const yesterday = new Date(now); yesterday.setDate(yesterday.getDate() - 1);
  const todayKey = dayKey(now); const yesterdayKey = dayKey(yesterday);
  const today = objects.filter(item => dayKey(new Date(item.observedAt)) === todayKey);
  const prior = objects.filter(item => dayKey(new Date(item.observedAt)) === yesterdayKey);
  const categories = [...new Set([...categoryIds, ...objects.map(item => item.category)])];
  const metrics = categories.map(id => metric(id, today.filter(item => item.category === id).length, prior.filter(item => item.category === id).length));
  const online = workspace.androidEnvironment.onlineDevices.find(device => device.status === "device");
  const connected = workspace.adbStatus === "ready" && !!online;
  const activeDevice = workspace.session && workspace.androidEnvironment.onlineDevices.find(device => device.id === workspace.session?.deviceId && device.status === "device");
  return {
    device: { connected, name: activeDevice?.id ?? online?.id }, observation: workspace.observations[0], objects, metrics,
    total: metric("total", today.length, prior.length), selectedMetric: metrics.some(item => item.id === selectedMetric) ? selectedMetric : "other",
    agentState: !connected ? "disconnected" : workspace.runtimeStatus === "paused" || workspace.session?.status === "ended" ? "paused" : workspace.runtimeStatus === "error" ? "error" : "waiting",
    plans, approval: "required_for_key_actions", mode: "live",
  };
}
export function selectMetric(model: WorkbenchViewModel, id: string): WorkbenchViewModel {
  return model.metrics.some(metric => metric.id === id) ? { ...model, selectedMetric: id } : model;
}
