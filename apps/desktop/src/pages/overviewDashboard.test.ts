import { describe, expect, it } from "vitest";
import type { AppSnapshotDto, ConnectorDto, EvidenceDto, SignalDto } from "@ordinconn/contracts";
import {
  buildOverviewDashboard,
  filterSignals,
  resolveOverviewDestination,
} from "./overviewDashboard";

const evidence = (id: string, relation: EvidenceDto["relation"]): EvidenceDto => ({
  id,
  source: "Test source",
  sourceType: "OFFICIAL_API",
  market: "traditional",
  asset: "TEST",
  title: "Observed test fact",
  content: "Controlled evidence fixture",
  capturedAt: "2026-09-12T10:00:00Z",
  freshness: 0.9,
  reliability: 0.9,
  factualLevel: "observed",
  confidence: 0.9,
  relation,
});

const signal = (
  id: string,
  market: SignalDto["market"],
  category: SignalDto["category"],
  direction: SignalDto["direction"],
  confidence: number,
  urgency: number,
  updatedAt: string,
  relation: EvidenceDto["relation"] = "primary",
): SignalDto => ({
  id,
  market,
  category,
  asset: id.toUpperCase(),
  title: `Signal ${id}`,
  summary: "Controlled signal fixture",
  direction,
  confidence,
  urgency,
  timeHorizon: "1 day",
  evidenceQuality: 0.8,
  evidence: [{ ...evidence(`evidence-${id}`, relation), market, asset: id.toUpperCase() }],
  catalysts: [],
  risks: [],
  invalidationConditions: [],
  createdAt: updatedAt,
  updatedAt,
  agentId: "test-agent",
  modelId: "test-model",
  status: "new",
});

const connectors: ConnectorDto[] = [
  { id: "mock", name: "Mock", market: "cross_market", connectorType: "mock", capabilities: [], authType: "none", status: "ready", reliability: 1, lastUpdate: "2026-09-12T10:00:00Z", mock: true },
  { id: "real", name: "Real", market: "crypto", connectorType: "api", capabilities: [], authType: "none", status: "degraded", reliability: 0.8, lastUpdate: "2026-09-12T10:00:00Z", mock: false },
];

const snapshot: AppSnapshotDto = {
  signals: [
    signal("trad-high", "traditional", "trading", "bullish", 0.9, 0.8, "2026-09-12T10:04:00Z", "contradicting"),
    signal("trad-watch", "traditional", "event", "watch", 0.6, 0.4, "2026-09-12T10:03:00Z"),
    signal("crypto-high", "crypto", "onchain", "bullish", 0.8, 0.7, "2026-09-12T10:02:00Z"),
    signal("crypto-low", "crypto", "exchange", "neutral", 0.5, 0.2, "2026-09-12T10:01:00Z"),
  ],
  connectors,
  providers: [],
  pendingApprovals: [{ id: "approval", proposalId: "proposal", proposalVersion: 1, proposalHash: "hash", proposalHashVersion: 1, allowedAction: "paper_execute", status: "requested", issuedAt: "2026-09-12T10:00:00Z", expiresAt: "2026-09-12T10:05:00Z" }],
  recentExecutions: [{ id: "execution", proposalId: "proposal", approvalRequestId: "approval", adapterId: "paper", status: "completed", createdAt: "2026-09-12T09:00:00Z", completedAt: "2026-09-12T09:01:00Z" }],
};

describe("Overview dashboard", () => {
  it("derives every metric from the supplied runtime snapshot", () => {
    const dashboard = buildOverviewDashboard(snapshot, { agentActivity: 3, reportsGenerated: 2 });

    expect(dashboard.kpis).toEqual({
      activeSignals: 4,
      traditionalPublished: 2,
      cryptoPublished: 2,
      pendingApprovals: 1,
      reportsGenerated: 2,
      readyConnectors: 1,
    });
    expect(dashboard.connectorHealth).toEqual({ total: 2, ready: 1, degraded: 1, unavailable: 0, mock: 1, real: 1 });
    expect(dashboard.snapshot).toEqual({ highConfidence: 2, watch: 1, contradicting: 1, averageConfidence: 0.7, averageUrgency: 0.525 });
    expect(dashboard.recentSignals.map((item) => item.id)).toEqual(["trad-high", "trad-watch", "crypto-high", "crypto-low"]);
    expect(dashboard.completedExecutions).toBe(1);
  });

  it("filters the Signals page for dashboard drill-down targets", () => {
    expect(filterSignals(snapshot.signals, { kind: "category", category: "event" }).map((item) => item.id)).toEqual(["trad-watch"]);
    expect(filterSignals(snapshot.signals, { kind: "highConfidence" }).map((item) => item.id)).toEqual(["trad-high", "crypto-high"]);
    expect(filterSignals(snapshot.signals, { kind: "contradicting" }).map((item) => item.id)).toEqual(["trad-high"]);
  });

  it("maps dashboard controls to existing pages and preserved filters", () => {
    expect(resolveOverviewDestination({ kind: "activeSignals" })).toEqual({ page: "signals", filter: { kind: "all" } });
    expect(resolveOverviewDestination({ kind: "traditionalPublished" })).toEqual({ page: "traditional" });
    expect(resolveOverviewDestination({ kind: "cryptoPublished" })).toEqual({ page: "crypto" });
    expect(resolveOverviewDestination({ kind: "category", category: "event" })).toEqual({ page: "signals", filter: { kind: "category", category: "event" } });
    expect(resolveOverviewDestination({ kind: "connectorHealth" })).toEqual({ page: "dataSources" });
    expect(resolveOverviewDestination({ kind: "pendingApprovals" })).toEqual({ page: "approvals" });
    expect(resolveOverviewDestination({ kind: "agentActivity" })).toEqual({ page: "agent" });
  });
});
