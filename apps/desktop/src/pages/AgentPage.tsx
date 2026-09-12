import { Bot, Braces, CircleDot, Network } from "lucide-react";
import type { AgentMessageDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";

const eventKeys = ["agent", "model", "signal", "approval", "execution", "system"] as const;

export function AgentPage({ messages, t }: { messages: AgentMessageDto[]; t: Translator }) {
  return (
    <section className="page">
      <header className="page-header"><span className="eyebrow">{t("nav.agent")}</span><h1>{t("agentPage.title")}</h1><p>{t("agentPage.description")}</p></header>
      <div className="agent-page-grid">
        <article className="panel protocol-card"><Network size={20} /><span className="eyebrow">{t("agentPage.protocol")}</span><h2>{t("top.localRuntime")}</h2><p>{t("agentPage.protocolDetail")}</p><div className="protocol-nodes">{eventKeys.map((key) => <span key={key}><CircleDot size={12} />{t(`agentPage.event.${key}`)}</span>)}</div></article>
        <article className="panel activity-card"><div className="panel-heading"><div><span className="eyebrow">{t("overview.agentActivity")}</span><h2>{t("agent.title")}</h2></div><Bot size={18} /></div>{messages.length === 0 ? <div className="empty-copy"><Braces size={22} /><p>{t("agentPage.empty")}</p></div> : messages.map((message) => <div className="activity-message" key={message.id}><span>{message.role}</span><p>{message.content}</p></div>)}</article>
      </div>
    </section>
  );
}
