import {
  Activity,
  Bitcoin,
  Bot,
  BrainCircuit,
  CalendarClock,
  CheckSquare,
  Database,
  Landmark,
  LayoutDashboard,
  Settings,
} from "lucide-react";
import type { Translator } from "../i18n";
import logoUrl from "../assets/ordinconn-logo-source.jpg";

export type PageId =
  | "overview"
  | "traditional"
  | "crypto"
  | "signals"
  | "agent"
  | "automations"
  | "models"
  | "dataSources"
  | "approvals"
  | "settings";

const navigation = [
  ["overview", "nav.overview", LayoutDashboard],
  ["traditional", "nav.traditional", Landmark],
  ["crypto", "nav.crypto", Bitcoin],
  ["signals", "nav.signals", Activity],
  ["agent", "nav.agent", Bot],
  ["automations", "nav.automations", CalendarClock],
  ["models", "nav.models", BrainCircuit],
  ["dataSources", "nav.dataSources", Database],
  ["approvals", "nav.approvals", CheckSquare],
  ["settings", "nav.settings", Settings],
] as const;

interface NavigationProps {
  page: PageId;
  onNavigate: (page: PageId) => void;
  t: Translator;
}

export function Navigation({ page, onNavigate, t }: NavigationProps) {
  return (
    <aside className="navigation">
      <div className="brand-block">
        <div className="brand-mark" aria-hidden="true"><img src={logoUrl} alt="" /></div>
        <div>
          <strong>{t("app.name")}</strong>
          <span>{t("app.version")}</span>
        </div>
      </div>
      <nav aria-label={t("app.name")}>
        {navigation.map(([id, label, Icon]) => (
          <button
            className={page === id ? "nav-item active" : "nav-item"}
            key={id}
            onClick={() => onNavigate(id)}
            type="button"
          >
            <Icon aria-hidden="true" size={17} strokeWidth={1.8} />
            <span>{t(label)}</span>
          </button>
        ))}
      </nav>
      <div className="navigation-footer">
        <span className="status-dot" />
        <div><strong>{t("top.localRuntime")}</strong><small>{t("common.ready")}</small></div>
      </div>
    </aside>
  );
}
