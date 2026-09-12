import { Database, Globe2, MonitorCog, ShieldAlert } from "lucide-react";
import type { Translator } from "../i18n";

export function SettingsPage({ t }: { t: Translator }) {
  const rows = [
    [Globe2, "settings.language", "settings.englishOnly", "settings.english"],
    [ShieldAlert, "settings.execution", "settings.safetyDetail", "settings.paperOnly"],
    [MonitorCog, "settings.computerUse", "settings.computerUseDetail", "common.unavailable"],
    [Database, "settings.data", "settings.dataDetail", "top.localRuntime"],
  ] as const;
  return (
    <section className="page">
      <header className="page-header"><span className="eyebrow">{t("nav.settings")}</span><h1>{t("settings.title")}</h1><p>{t("settings.description")}</p></header>
      <div className="settings-list">{rows.map(([Icon, title, description, value]) => <article className="panel setting-row" key={title}><div className="setting-icon"><Icon size={18} /></div><div><h2>{t(title)}</h2><p>{t(description)}</p></div><span className="setting-value">{t(value)}</span></article>)}</div>
    </section>
  );
}
