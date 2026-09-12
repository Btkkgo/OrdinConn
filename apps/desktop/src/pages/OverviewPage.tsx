import { Activity, ArrowRight, Bot, CheckSquare, Database, Landmark, ShieldCheck } from "lucide-react";
import type { AppSnapshotDto, SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { SignalCard } from "../components/SignalCard";

interface OverviewPageProps {
  snapshot: AppSnapshotDto;
  agentActivity: number;
  reportReady: boolean;
  onOpenSignal: (signal: SignalDto) => void;
  t: Translator;
}

export function OverviewPage({ snapshot, agentActivity, reportReady, onOpenSignal, t }: OverviewPageProps) {
  const traditional = snapshot.signals.filter((signal) => signal.market === "traditional");
  const crypto = snapshot.signals.filter((signal) => signal.market === "crypto");
  const priority = [...snapshot.signals]
    .sort((left, right) => right.urgency - left.urgency || right.confidence - left.confidence)
    .slice(0, 4);

  return (
    <section className="page overview-page">
      <header className="hero-panel">
        <div>
          <span className="eyebrow">{t("overview.eyebrow")}</span>
          <h1>{t("overview.title")}</h1>
          <p>{t("overview.description")}</p>
        </div>
        <div className="hero-badge"><ShieldCheck size={18} /><span>{t("market.evidenceGate")}</span></div>
      </header>

      <div className="stat-grid">
        <article><Activity size={18} /><span>{t("overview.activeSignals")}</span><strong>{snapshot.signals.length}</strong><small>{t("overview.signalsReady")}</small></article>
        <article><Landmark size={18} /><span>{t("nav.traditional")}</span><strong>{traditional.length}</strong><small>{t("market.published")}</small></article>
        <article><Bot size={18} /><span>{t("nav.crypto")}</span><strong>{crypto.length}</strong><small>{t("market.published")}</small></article>
        <article><CheckSquare size={18} /><span>{t("overview.pendingApprovals")}</span><strong>{snapshot.pendingApprovals.length}</strong><small>{t("common.pending")}</small></article>
      </div>

      <div className="dashboard-grid">
        <article className="panel span-two">
          <div className="panel-heading"><div><span className="eyebrow">{t("overview.highPriority")}</span><h2>{t("overview.activeSignals")}</h2></div><ArrowRight size={17} /></div>
          <div className="signal-grid compact-grid">{priority.map((signal) => <SignalCard key={signal.id} signal={signal} onOpen={onOpenSignal} t={t} />)}</div>
        </article>
        <article className="panel status-panel">
          <span className="eyebrow">{t("overview.dataStatus")}</span>
          <h2>{t("common.ready")}</h2>
          <div className="status-list">
            {snapshot.connectors.slice(0, 6).map((connector) => <div key={connector.id}><Database size={14} /><span>{connector.name}</span><i className={connector.status} /></div>)}
          </div>
          <p className="disclosure">{t("overview.mockDisclosure")}</p>
        </article>
      </div>
      <div className="operations-grid">
        <article className="panel"><span className="eyebrow">{t("overview.marketStatus")}</span><strong>{t("common.active")}</strong><small>{traditional.length + crypto.length} {t("overview.signalsReady")}</small></article>
        <article className="panel"><span className="eyebrow">{t("overview.agentActivity")}</span><strong>{agentActivity}</strong><small>{t("overview.runtimeFeed")}</small></article>
        <article className="panel"><span className="eyebrow">{t("overview.modelStatus")}</span><strong>{snapshot.providers.length || t("top.model")}</strong><small>{snapshot.providers.length ? t("overview.providerCount") : t("models.noProviders")}</small></article>
        <article className="panel"><span className="eyebrow">{t("overview.recentReports")}</span><strong>{reportReady ? t("overview.reportReady") : t("common.none")}</strong><small>{reportReady ? t("overview.reportStatus") : t("overview.noReport")}</small></article>
        <article className="panel"><span className="eyebrow">{t("overview.executions")}</span><strong>{snapshot.recentExecutions.length}</strong><small>{t("settings.paperOnly")}</small></article>
      </div>
    </section>
  );
}
