import {
  Activity,
  ArrowRight,
  Bot,
  CheckSquare,
  Database,
  FileText,
  Landmark,
  ShieldCheck,
} from "lucide-react";
import type { AppSnapshotDto, SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { buildOverviewDashboard, type OverviewTarget } from "./overviewDashboard";

interface OverviewPageProps {
  snapshot: AppSnapshotDto;
  agentActivity: number;
  reportsGenerated: number;
  onOpenSignal: (signal: SignalDto) => void;
  onNavigate: (target: OverviewTarget) => void;
  t: Translator;
}

function percentage(value: number): string {
  return `${Math.round(value * 100)}%`;
}

export function OverviewPage({ snapshot, agentActivity, reportsGenerated, onOpenSignal, onNavigate, t }: OverviewPageProps) {
  const dashboard = buildOverviewDashboard(snapshot, { agentActivity, reportsGenerated });
  const marketTotal = Math.max(1, dashboard.marketDistribution.total);
  const lastExecution = snapshot.recentExecutions[0];
  const kpis = [
    { label: t("overview.activeSignals"), value: dashboard.kpis.activeSignals, detail: t("market.published"), icon: Activity, target: { kind: "activeSignals" } as const },
    { label: t("nav.traditional"), value: dashboard.kpis.traditionalPublished, detail: t("overview.signalDistribution"), icon: Landmark, target: { kind: "traditionalPublished" } as const },
    { label: t("nav.crypto"), value: dashboard.kpis.cryptoPublished, detail: t("overview.signalDistribution"), icon: Bot, target: { kind: "cryptoPublished" } as const },
    { label: t("overview.pendingApprovals"), value: dashboard.kpis.pendingApprovals, detail: t("common.pending"), icon: CheckSquare, target: { kind: "pendingApprovals" } as const },
    { label: t("overview.reportsGenerated"), value: dashboard.kpis.reportsGenerated, detail: t("overview.session"), icon: FileText, target: { kind: "reportsGenerated" } as const },
    { label: t("overview.readyConnectors"), value: dashboard.kpis.readyConnectors, detail: `${dashboard.connectorHealth.total} ${t("common.total").toLocaleLowerCase()}`, icon: Database, target: { kind: "connectorHealth" } as const },
  ];

  return (
    <section className="page overview-page">
      <div className="overview-kpi-grid" aria-label={t("overview.kpiSummary")}>
        {kpis.map(({ label, value, detail, icon: Icon, target }) => (
          <button className="overview-kpi" key={label} onClick={() => onNavigate(target)} type="button">
            <span className="overview-kpi-icon"><Icon size={18} /></span>
            <span className="overview-kpi-copy"><span>{label}</span><small>{detail}</small></span>
            <strong>{value}</strong>
          </button>
        ))}
      </div>

      <div className="overview-layout">
        <article className="panel overview-market-panel">
          <button className="panel-link-heading" onClick={() => onNavigate({ kind: "activeSignals" })} type="button">
            <span><small>{t("overview.marketDistribution")}</small><strong>{t("overview.signalDistribution")}</strong></span><ArrowRight size={17} />
          </button>
          <div className="market-distribution" aria-label={t("overview.marketDistribution")}>
            <div className="distribution-bar">
              <span className="distribution-traditional" style={{ width: `${(dashboard.marketDistribution.traditional / marketTotal) * 100}%` }} />
              <span className="distribution-crypto" style={{ width: `${(dashboard.marketDistribution.crypto / marketTotal) * 100}%` }} />
            </div>
            <div className="distribution-legend">
              <button onClick={() => onNavigate({ kind: "traditionalPublished" })} type="button"><i className="traditional" /><span>{t("nav.traditional")}</span><strong>{dashboard.marketDistribution.traditional}</strong></button>
              <button onClick={() => onNavigate({ kind: "cryptoPublished" })} type="button"><i className="crypto" /><span>{t("nav.crypto")}</span><strong>{dashboard.marketDistribution.crypto}</strong></button>
            </div>
          </div>
          <div className="category-list">
            {dashboard.categoryDistribution.map((item) => (
              <button key={item.category} onClick={() => onNavigate({ kind: "category", category: item.category })} type="button">
                <span>{t(`signal.category.${item.category}`)}</span><span className="category-track"><i style={{ width: `${item.share * 100}%` }} /></span><strong>{item.count}</strong>
              </button>
            ))}
          </div>
        </article>

        <article className="panel overview-snapshot-panel">
          <div className="panel-static-heading"><span><small>{t("overview.signalHealth")}</small><strong>{t("overview.confidenceSnapshot")}</strong></span><ShieldCheck size={17} /></div>
          <div className="snapshot-actions">
            <button onClick={() => onNavigate({ kind: "highConfidence" })} type="button"><span>{t("overview.highConfidence")}</span><strong>{dashboard.snapshot.highConfidence}</strong></button>
            <button onClick={() => onNavigate({ kind: "watchSignals" })} type="button"><span>{t("overview.watchSignals")}</span><strong>{dashboard.snapshot.watch}</strong></button>
            <button onClick={() => onNavigate({ kind: "contradictingEvidence" })} type="button"><span>{t("overview.contradictingEvidence")}</span><strong>{dashboard.snapshot.contradicting}</strong></button>
          </div>
          <div className="gauge-grid">
            <div><span>{t("overview.averageConfidence")}</span><strong>{percentage(dashboard.snapshot.averageConfidence)}</strong><i><b style={{ width: percentage(dashboard.snapshot.averageConfidence) }} /></i></div>
            <div><span>{t("overview.averageUrgency")}</span><strong>{percentage(dashboard.snapshot.averageUrgency)}</strong><i><b style={{ width: percentage(dashboard.snapshot.averageUrgency) }} /></i></div>
          </div>
        </article>

        <article className="panel overview-recent-panel">
          <div className="panel-static-heading"><span><small>{t("overview.latestUpdates")}</small><strong>{t("overview.recentSignals")}</strong></span><Activity size={17} /></div>
          <div className="recent-signal-list">
            {dashboard.recentSignals.length ? dashboard.recentSignals.map((signal) => (
              <button key={signal.id} onClick={() => onOpenSignal(signal)} type="button">
                <span className="recent-asset">{signal.asset}</span><span className="recent-copy"><strong>{signal.title}</strong><small>{t(`signal.category.${signal.category}`)} · {percentage(signal.confidence)}</small></span><ArrowRight size={16} />
              </button>
            )) : <div className="dashboard-empty">{t("signal.noResults")}</div>}
          </div>
        </article>

        <article className="panel overview-connector-panel">
          <button className="panel-link-heading" onClick={() => onNavigate({ kind: "connectorHealth" })} type="button">
            <span><small>{t("overview.sourceMix")}</small><strong>{t("overview.connectorHealth")}</strong></span><ArrowRight size={17} />
          </button>
          <div className="connector-health-number"><strong>{dashboard.connectorHealth.ready}</strong><span>/ {dashboard.connectorHealth.total} {t("common.ready").toLocaleLowerCase()}</span></div>
          <div className="health-track" aria-hidden="true"><i style={{ width: `${dashboard.connectorHealth.total ? (dashboard.connectorHealth.ready / dashboard.connectorHealth.total) * 100 : 0}%` }} /></div>
          <dl className="health-breakdown">
            <div><dt>{t("common.ready")}</dt><dd>{dashboard.connectorHealth.ready}</dd></div>
            <div><dt>{t("common.degraded")}</dt><dd>{dashboard.connectorHealth.degraded}</dd></div>
            <div><dt>{t("common.unavailable")}</dt><dd>{dashboard.connectorHealth.unavailable}</dd></div>
            <div><dt>{t("overview.mockConnectors")}</dt><dd>{dashboard.connectorHealth.mock}</dd></div>
            <div><dt>{t("overview.realConnectors")}</dt><dd>{dashboard.connectorHealth.real}</dd></div>
          </dl>
        </article>

        <button className="panel overview-operation-card" onClick={() => onNavigate({ kind: "completedExecutions" })} type="button">
          <span><small>{t("settings.paperOnly")}</small><strong>{t("overview.approvalExecution")}</strong></span>
          <span className="operation-metric"><b>{dashboard.completedExecutions}</b><small>{t("overview.completedExecutions")}</small></span>
          <span className="operation-detail">{lastExecution ? (lastExecution.resultSummary ?? lastExecution.status) : t("overview.noExecution")}</span><ArrowRight size={17} />
        </button>

        <button className="panel overview-operation-card" onClick={() => onNavigate({ kind: "agentActivity" })} type="button">
          <span><small>{t("overview.session")}</small><strong>{t("overview.agentActivity")}</strong></span>
          <span className="operation-metric"><b>{dashboard.agentActivity}</b><small>{t("overview.messages")}</small></span>
          <span className="operation-detail">{dashboard.reportsGenerated} {t("overview.reportsGenerated").toLocaleLowerCase()}</span><ArrowRight size={17} />
        </button>
      </div>
    </section>
  );
}
