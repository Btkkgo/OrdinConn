import { Bitcoin, CalendarClock, Landmark, WalletCards } from "lucide-react";
import type { Translator } from "../i18n";

const automationItems = [
  ["automations.btc", Bitcoin],
  ["automations.wallet", WalletCards],
  ["automations.market", Landmark],
] as const;

export function AutomationsPage({ t }: { t: Translator }) {
  return (
    <section className="page">
      <header className="page-header"><span className="eyebrow">{t("nav.automations")}</span><h1>{t("automations.title")}</h1><p>{t("automations.description")}</p></header>
      <div className="automation-grid">{automationItems.map(([key, Icon]) => <article className="panel automation-card" key={key}><div><Icon size={19} /><span className="state-chip">{t("automations.disabled")}</span></div><h2>{t(key)}</h2><p>{t("automations.description")}</p><button type="button" disabled><CalendarClock size={15} />{t("common.unavailable")}</button></article>)}</div>
    </section>
  );
}
