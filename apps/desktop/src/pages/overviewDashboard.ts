import type {
  AppSnapshotDto,
  SignalCategory,
  SignalDto,
} from "@ordinconn/contracts";
import type { PageId } from "../components/Navigation";

const dashboardCategories: SignalCategory[] = [
  "trading",
  "event",
  "demand",
  "onchain",
  "exchange",
];

const activeStatuses = new Set<SignalDto["status"]>(["new", "watching", "approved"]);

export type SignalFilter =
  | { kind: "all" }
  | { kind: "category"; category: SignalCategory }
  | { kind: "highConfidence" }
  | { kind: "watch" }
  | { kind: "contradicting" };

export type OverviewTarget =
  | { kind: "activeSignals" }
  | { kind: "traditionalPublished" }
  | { kind: "cryptoPublished" }
  | { kind: "pendingApprovals" }
  | { kind: "reportsGenerated" }
  | { kind: "connectorHealth" }
  | { kind: "highConfidence" }
  | { kind: "watchSignals" }
  | { kind: "contradictingEvidence" }
  | { kind: "completedExecutions" }
  | { kind: "agentActivity" }
  | { kind: "category"; category: SignalCategory };

export interface OverviewDestination {
  page: PageId;
  filter?: SignalFilter;
}

export interface OverviewDashboardModel {
  kpis: {
    activeSignals: number;
    traditionalPublished: number;
    cryptoPublished: number;
    pendingApprovals: number;
    reportsGenerated: number;
    readyConnectors: number;
  };
  marketDistribution: {
    traditional: number;
    crypto: number;
    total: number;
  };
  categoryDistribution: Array<{
    category: SignalCategory;
    count: number;
    share: number;
  }>;
  snapshot: {
    highConfidence: number;
    watch: number;
    contradicting: number;
    averageConfidence: number;
    averageUrgency: number;
  };
  connectorHealth: {
    total: number;
    ready: number;
    degraded: number;
    unavailable: number;
    mock: number;
    real: number;
  };
  recentSignals: SignalDto[];
  completedExecutions: number;
  agentActivity: number;
  reportsGenerated: number;
}

function rounded(value: number): number {
  return Math.round(value * 1_000) / 1_000;
}

export function buildOverviewDashboard(
  snapshot: AppSnapshotDto,
  activity: { agentActivity: number; reportsGenerated: number },
): OverviewDashboardModel {
  const activeSignals = snapshot.signals.filter((signal) => activeStatuses.has(signal.status));
  const traditionalPublished = activeSignals.filter((signal) => signal.market === "traditional").length;
  const cryptoPublished = activeSignals.filter((signal) => signal.market === "crypto").length;
  const readyConnectors = snapshot.connectors.filter((connector) => connector.status === "ready").length;
  const totalConfidence = activeSignals.reduce((sum, signal) => sum + signal.confidence, 0);
  const totalUrgency = activeSignals.reduce((sum, signal) => sum + signal.urgency, 0);
  const total = activeSignals.length;

  return {
    kpis: {
      activeSignals: total,
      traditionalPublished,
      cryptoPublished,
      pendingApprovals: snapshot.pendingApprovals.length,
      reportsGenerated: activity.reportsGenerated,
      readyConnectors,
    },
    marketDistribution: { traditional: traditionalPublished, crypto: cryptoPublished, total },
    categoryDistribution: dashboardCategories.map((category) => {
      const count = activeSignals.filter((signal) => signal.category === category).length;
      return { category, count, share: total === 0 ? 0 : count / total };
    }),
    snapshot: {
      highConfidence: activeSignals.filter((signal) => signal.confidence >= 0.75).length,
      watch: activeSignals.filter((signal) => signal.direction === "watch" || signal.status === "watching").length,
      contradicting: activeSignals.filter((signal) => signal.evidence.some((item) => item.relation === "contradicting")).length,
      averageConfidence: total === 0 ? 0 : rounded(totalConfidence / total),
      averageUrgency: total === 0 ? 0 : rounded(totalUrgency / total),
    },
    connectorHealth: {
      total: snapshot.connectors.length,
      ready: readyConnectors,
      degraded: snapshot.connectors.filter((connector) => connector.status === "degraded").length,
      unavailable: snapshot.connectors.filter((connector) => connector.status === "unavailable").length,
      mock: snapshot.connectors.filter((connector) => connector.mock).length,
      real: snapshot.connectors.filter((connector) => !connector.mock).length,
    },
    recentSignals: [...activeSignals]
      .sort((left, right) => Date.parse(right.updatedAt) - Date.parse(left.updatedAt))
      .slice(0, 5),
    completedExecutions: snapshot.recentExecutions.filter((item) => item.status === "completed").length,
    agentActivity: activity.agentActivity,
    reportsGenerated: activity.reportsGenerated,
  };
}

export function filterSignals(signals: SignalDto[], filter: SignalFilter): SignalDto[] {
  if (filter.kind === "all") return signals;
  if (filter.kind === "category") return signals.filter((signal) => signal.category === filter.category);
  if (filter.kind === "highConfidence") return signals.filter((signal) => signal.confidence >= 0.75);
  if (filter.kind === "watch") return signals.filter((signal) => signal.direction === "watch" || signal.status === "watching");
  return signals.filter((signal) => signal.evidence.some((item) => item.relation === "contradicting"));
}

export function resolveOverviewDestination(target: OverviewTarget): OverviewDestination {
  if (target.kind === "traditionalPublished") return { page: "traditional" };
  if (target.kind === "cryptoPublished") return { page: "crypto" };
  if (target.kind === "pendingApprovals" || target.kind === "completedExecutions") return { page: "approvals" };
  if (target.kind === "connectorHealth") return { page: "dataSources" };
  if (target.kind === "reportsGenerated" || target.kind === "agentActivity") return { page: "agent" };
  if (target.kind === "category") return { page: "signals", filter: { kind: "category", category: target.category } };
  if (target.kind === "highConfidence") return { page: "signals", filter: { kind: "highConfidence" } };
  if (target.kind === "watchSignals") return { page: "signals", filter: { kind: "watch" } };
  if (target.kind === "contradictingEvidence") return { page: "signals", filter: { kind: "contradicting" } };
  return { page: "signals", filter: { kind: "all" } };
}
