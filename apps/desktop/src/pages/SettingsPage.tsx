import { Database, Globe2, MonitorCog, ShieldAlert } from "lucide-react";
import type { Locale, Translator } from "../i18n";

interface SettingsPageProps {
  locale: Locale;
  onLocaleChange: (locale: Locale) => void;
  t: Translator;
}

export function SettingsPage({ locale, onLocaleChange, t }: SettingsPageProps) {
  const rows = [
    [ShieldAlert, "settings.execution", "settings.safetyDetail", "settings.paperOnly"],
    [MonitorCog, "settings.computerUse", "settings.computerUseDetail", "common.unavailable"],
    [Database, "settings.data", "settings.dataDetail", "top.localRuntime"],
  ] as const;
  return (
    <section className="page">
      <header className="page-header"><span className="eyebrow">{t("nav.settings")}</span><h1>{t("settings.title")}</h1><p>{t("settings.description")}</p></header>
      <div className="settings-list">
        <article className="panel setting-row">
          <div className="setting-icon"><Globe2 size={18} /></div>
          <div><h2>{t("settings.language")}</h2><p>{t("settings.languageDetail")}</p></div>
          <select className="language-select" aria-label={t("settings.language")} value={locale} onChange={(event) => onLocaleChange(event.target.value as Locale)}>
            <option value="en">{t("settings.english")}</option>
            <option value="zh-CN">{t("settings.simplifiedChinese")}</option>
          </select>
        </article>
        {rows.map(([Icon, title, description, value]) => <article className="panel setting-row" key={title}><div className="setting-icon"><Icon size={18} /></div><div><h2>{t(title)}</h2><p>{t(description)}</p></div><span className="setting-value">{t(value)}</span></article>)}
      </div>
    </section>
  );
}
