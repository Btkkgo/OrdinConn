import { Bot, ChevronLeft, ChevronRight, FileText, Send, ShieldCheck, Sparkles, X } from "lucide-react";
import { useState, type FormEvent } from "react";
import type { AgentMessageDto, AgentReportDto, ApprovalRequestDto, ExecutionRecordDto, SignalDto, TradeProposalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";

interface AgentDockProps {
  collapsed: boolean;
  signal?: SignalDto;
  messages: AgentMessageDto[];
  busy: boolean;
  report?: AgentReportDto;
  proposal?: TradeProposalDto;
  approval?: ApprovalRequestDto;
  execution?: ExecutionRecordDto;
  onToggle: () => void;
  onSend: (question: string) => Promise<void>;
  onReport: () => Promise<void>;
  onProposal: () => Promise<void>;
  onRequestApproval: () => Promise<void>;
  onApprove: () => Promise<void>;
  t: Translator;
}

export function AgentDock(props: AgentDockProps) {
  const { collapsed, signal, messages, busy, report, proposal, approval, execution, onToggle, t } = props;
  const [question, setQuestion] = useState("");
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const value = question.trim();
    if (!value || busy) return;
    setQuestion("");
    await props.onSend(value);
  };
  if (collapsed) {
    return <button className="dock-expand" onClick={onToggle} aria-label={t("agent.expand")} type="button"><ChevronLeft size={18} /><Bot size={18} /></button>;
  }
  return (
    <div className="agent-dock">
      <header className="dock-header">
        <div className="agent-identity"><div className="agent-icon"><Bot size={18} /></div><div><strong>{t("agent.title")}</strong><span><i />{t("agent.contextReady")}</span></div></div>
        <button className="icon-button" onClick={onToggle} aria-label={t("agent.collapse")} type="button"><ChevronRight size={18} /></button>
      </header>
      <div className="context-card">
        <span>{signal?.market ?? t("top.context")}</span>
        <strong>{signal ? `${signal.asset} · ${signal.category}` : t("agent.noSignal")}</strong>
        {signal ? <small>{signal.evidence.length} {t("signal.evidence").toLowerCase()}</small> : null}
      </div>
      <div className="quick-actions">
        <button disabled={!signal || busy} onClick={() => props.onSend(t("agent.explain"))} type="button"><Sparkles size={14} />{t("agent.explain")}</button>
        <button disabled={!signal || busy} onClick={() => props.onSend(t("agent.risk"))} type="button"><ShieldCheck size={14} />{t("agent.risk")}</button>
        <button disabled={!signal || busy} onClick={() => props.onSend(t("agent.compare"))} type="button">{t("agent.compare")}</button>
        <button disabled={!signal || busy} onClick={() => props.onSend(t("agent.investigate"))} type="button">{t("agent.investigate")}</button>
        <button disabled={!signal || busy} onClick={() => props.onSend(t("agent.monitor"))} type="button">{t("agent.monitor")}</button>
        <button disabled={!signal || busy} onClick={() => props.onSend(t("agent.viewEvidence"))} type="button">{t("agent.viewEvidence")}</button>
      </div>
      <div className="conversation">
        {messages.length === 0 ? <div className="empty-conversation"><Bot size={24} /><p>{t("agent.noSignal")}</p></div> : null}
        {messages.map((message) => (
          <div className={`message ${message.role}`} key={message.id}>
            <div className="message-label">{message.role === "assistant" ? t("agent.title") : t("top.context")}{message.mock ? <span>{t("agent.mockBadge")}</span> : null}</div>
            <p>{message.content}</p>
          </div>
        ))}
        {busy ? <div className="message assistant typing"><div className="message-label">{t("agent.title")}</div><p>{t("agent.processing")}</p></div> : null}
        {report ? <div className="artifact"><FileText size={15} /><div><strong>{t("agent.reportCreated")}</strong><span>{t("agent.evidenceVsInference")}</span></div></div> : null}
        {proposal ? <div className="artifact"><ShieldCheck size={15} /><div><strong>{t("agent.proposalCreated")}</strong><span>{t("agent.paperOnly")}</span></div></div> : null}
        {execution ? <div className="artifact success"><ShieldCheck size={15} /><div><strong>{t("agent.executionCompleted")}</strong><span>{execution.resultSummary}</span></div></div> : null}
      </div>
      <div className="workflow-actions">
        <button disabled={!signal || busy} onClick={props.onReport} type="button">{t("agent.report")}</button>
        <button disabled={!signal || busy || Boolean(proposal)} onClick={props.onProposal} type="button">{t("agent.proposal")}</button>
        {proposal && !approval ? <button className="gold-button" onClick={props.onRequestApproval} type="button">{t("agent.requestApproval")}</button> : null}
        {approval && !execution ? <button className="gold-button" onClick={props.onApprove} type="button">{t("common.approve")}</button> : null}
      </div>
      <form className="agent-input" onSubmit={submit}>
        <textarea value={question} onChange={(event) => setQuestion(event.target.value)} placeholder={t("agent.placeholder")} disabled={!signal || busy} rows={2} />
        <button type="submit" disabled={!signal || busy || !question.trim()} aria-label={t("agent.send")}><Send size={16} /></button>
      </form>
    </div>
  );
}
