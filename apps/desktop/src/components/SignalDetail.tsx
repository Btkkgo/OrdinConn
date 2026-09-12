import { ArrowLeft, ExternalLink, ShieldAlert } from "lucide-react";
import type { SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";

interface SignalDetailProps { signal: SignalDto; onBack: () => void; t: Translator }

export function SignalDetail({ signal, onBack, t }: SignalDetailProps) {
  return (
    <section className="detail-page">
      <button className="text-button" onClick={onBack} type="button"><ArrowLeft size={15} />{t("signal.back")}</button>
      <div className="detail-hero">
        <div><span className="eyebrow">{t("signal.detail")}</span><h1>{signal.asset} · {signal.title}</h1><p>{signal.summary}</p></div>
        <span className={`direction large ${signal.direction}`}>{t(`signal.direction.${signal.direction}`)}</span>
      </div>
      <div className="metric-strip">
        <div><span>{t("signal.confidence")}</span><strong>{Math.round(signal.confidence * 100)}%</strong></div>
        <div><span>{t("signal.evidenceQuality")}</span><strong>{Math.round(signal.evidenceQuality * 100)}%</strong></div>
        <div><span>{t("signal.urgency")}</span><strong>{Math.round(signal.urgency * 100)}%</strong></div>
        <div><span>{t("signal.timeHorizon")}</span><strong>{signal.timeHorizon}</strong></div>
      </div>
      <div className="detail-grid">
        <article className="panel evidence-panel">
          <div className="panel-heading"><div><span className="eyebrow">{t("signal.evidence")}</span><h2>{t("signal.timeline")}</h2></div><span className="count-chip">{signal.evidence.length}</span></div>
          <div className="evidence-list">
            {signal.evidence.map((item) => (
              <div className="evidence-item" key={item.id}>
                <div className="timeline-dot" />
                <div>
                  <div className="evidence-meta"><span>{item.relation}</span><span>{item.sourceType}</span></div>
                  <h3>{item.title}</h3><p>{item.content}</p>
                  <div className="evidence-footer"><span>{t("signal.source")}: {item.source}</span><span>{t("signal.factualLevel")}: {item.factualLevel}</span><ExternalLink size={13} /></div>
                </div>
              </div>
            ))}
          </div>
        </article>
        <div className="analysis-stack">
          <article className="panel"><span className="eyebrow">{t("signal.catalysts")}</span><ul>{signal.catalysts.map((item) => <li key={item}>{item}</li>)}</ul></article>
          <article className="panel risk-panel"><span className="eyebrow"><ShieldAlert size={13} />{t("signal.risks")}</span><ul>{signal.risks.map((item) => <li key={item}>{item}</li>)}</ul></article>
          <article className="panel"><span className="eyebrow">{t("signal.invalidation")}</span><ul>{signal.invalidationConditions.map((item) => <li key={item}>{item}</li>)}</ul></article>
        </div>
      </div>
    </section>
  );
}
