import type { ReactNode } from "react";
import { Inbox } from "lucide-react";
import type { Translator } from "../i18n";

export function PageHeader({ title, description, status }: { title: string; description: string; status?: ReactNode }) {
  return <header className="page-header"><div><h1>{title}</h1><p>{description}</p></div>{status}</header>;
}
export function SectionHeader({ title, description }: { title: string; description?: string }) {
  return <header className="section-header"><h2>{title}</h2>{description ? <p>{description}</p> : null}</header>;
}
export function StatusBadge({ children, tone = "neutral" }: { children: ReactNode; tone?: "neutral" | "success" | "danger" }) {
  return <span className={`status-badge tone-${tone}`}><i aria-hidden="true" />{children}</span>;
}
export function EmptyState({ title, description, actions }: { title: string; description: string; actions?: ReactNode }) {
  return <div className="empty-state"><Inbox size={26} aria-hidden="true" /><h3>{title}</h3><p>{description}</p>{actions ? <div className="empty-state-actions">{actions}</div> : null}</div>;
}
export const metricColorMap = { total: "accent", news: "info", stocks: "success", chat: "purple", feedback: "warning", other: "neutral", warning: "warning", error: "danger" } as const;
export function MetricValue({ value, tone }: { value: number; tone: typeof metricColorMap[keyof typeof metricColorMap] }) {
  return <strong className={`metric-value tone-${tone}`}>{value.toLocaleString("en-US")}</strong>;
}
export function Alert({ title, description, detail, success = false, t }: { title: string; description?: string; detail?: string; success?: boolean; t: Translator }) {
  return <div className={`alert ${success ? "alert-success" : "alert-danger"}`} role={success ? "status" : "alert"}><strong>{title}</strong>{description ? <p>{description}</p> : null}{detail ? <details><summary>{t("common.viewDetails")}</summary><pre>{detail}</pre></details> : null}</div>;
}
