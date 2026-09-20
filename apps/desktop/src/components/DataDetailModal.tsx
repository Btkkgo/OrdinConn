import { useState, type FormEvent } from "react";
import type { AgentMessageDto, IntelligenceItemDto, SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { sourceLabel } from "../pages/mobileIntelligence";

interface DataDetailModalProps {
  item: IntelligenceItemDto;
  signals: SignalDto[];
  messages: AgentMessageDto[];
  onClose: () => void;
  onWarehouseChange: (favorite: boolean, saved: boolean, tags: string[]) => void;
  onDiscuss: (question: string) => void;
  onResearch: () => void;
  t: Translator;
}

export function DataDetailModal({ item, signals, messages, onClose, onWarehouseChange, onDiscuss, onResearch, t }: DataDetailModalProps) {
  const [question, setQuestion] = useState("");
  const [tags, setTags] = useState("");
  const submit = (event: FormEvent) => { event.preventDefault(); if (question.trim()) onDiscuss(question.trim()); };
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="data-detail panel" role="dialog" aria-modal="true" aria-labelledby="data-detail-title" onMouseDown={(event) => event.stopPropagation()}>
        <header><div><span className="source-chip">{sourceLabel(item.sourceMethod)} · {item.evidenceStatus}</span><h2 id="data-detail-title">{item.title}</h2></div><button className="icon-button" type="button" onClick={onClose} aria-label={t("common.close")}>×</button></header>
        <p className="detail-summary">{item.summary}</p>
        <dl className="lineage-grid"><div><dt>{t("detail.source")}</dt><dd>{item.sourceApp}</dd></div><div><dt>{t("detail.observed")}</dt><dd>{new Date(item.observedAt).toLocaleString()}</dd></div><div><dt>{t("detail.confidence")}</dt><dd>{Math.round(item.confidence * 100)}%</dd></div><div><dt>{t("detail.lineage")}</dt><dd>{item.sourceLocator ?? (item.evidenceIds.join(", ") || t("detail.observationOnly"))}</dd></div></dl>
        <div className="tag-list">{item.assets.map((asset) => <span key={asset}>{asset}</span>)}</div>
        <div className="detail-actions"><button type="button" className={item.favorite ? "gold-button" : "secondary-button"} onClick={() => onWarehouseChange(!item.favorite, item.saved, tags.split(",").filter(Boolean))}>{t("detail.favorite")}</button><button type="button" className={item.saved ? "gold-button" : "secondary-button"} onClick={() => onWarehouseChange(item.favorite, !item.saved, tags.split(",").filter(Boolean))}>{t("detail.save")}</button><button type="button" className="secondary-button" onClick={onResearch}>{t("detail.research")}</button></div>
        <label><span>{t("detail.tags")}</span><input value={tags} onChange={(event) => setTags(event.target.value)} placeholder="macro, exchange" /></label>
        <div className="detail-conversation">{messages.slice(-6).map((message) => <article className={`message ${message.role}`} key={message.id}><span className="message-label">{message.role}</span><p>{message.content}</p></article>)}</div>
        <form className="detail-discussion" onSubmit={submit}><label><span>{t("detail.discuss")}</span><textarea value={question} onChange={(event) => setQuestion(event.target.value)} placeholder={t("detail.discussPlaceholder")} /></label><button className="gold-button" type="submit">{t("detail.discuss")}</button></form>
        <div className="related-mini"><strong>{t("detail.relatedSignals")}</strong><span>{signals.length ? signals.map((signal) => signal.title).join(" · ") : t("mobile.noSignal")}</span></div>
      </section>
    </div>
  );
}
