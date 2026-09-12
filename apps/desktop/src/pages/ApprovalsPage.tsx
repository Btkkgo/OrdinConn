import { Fingerprint, ShieldCheck } from "lucide-react";
import type { ApprovalRequestDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";

export function ApprovalsPage({ approvals, onApprove, t }: { approvals: ApprovalRequestDto[]; onApprove: (approval: ApprovalRequestDto) => Promise<void>; t: Translator }) {
  return (
    <section className="page">
      <header className="page-header"><span className="eyebrow">{t("nav.approvals")}</span><h1>{t("approvals.title")}</h1><p>{t("approvals.description")}</p></header>
      <div className="safety-banner"><ShieldCheck size={18} /><p>{t("approvals.paperWarning")}</p></div>
      {approvals.length === 0 ? <div className="empty-panel"><Fingerprint size={24} /><p>{t("approvals.none")}</p></div> : <div className="approval-list">{approvals.map((approval) => <article className="panel approval-row" key={approval.id}><div><span className="eyebrow">{approval.allowedAction}</span><h2>{approval.proposalId}</h2><p>{t("approvals.hash")}: <code>{approval.proposalHash.slice(0, 24)}</code></p><small>{t("approvals.expires")}: {new Date(approval.expiresAt).toLocaleString()}</small></div><button className="gold-button" onClick={() => onApprove(approval)} type="button">{t("common.approve")}</button></article>)}</div>}
    </section>
  );
}
