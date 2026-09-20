import { useEffect, useState } from "react";
import type { MobileRuntimeSettingsDto, MobileWorkspaceDto, ModelProviderDto } from "@ordinconn/contracts";
import { Database, Globe2, MonitorSmartphone, Type } from "lucide-react";
import type { Locale, Translator } from "../i18n";
import type { ConnectionTestResult, ProviderInput } from "../runtime/client";
import type { TextScale } from "../runtime/preferences";
import { ModelsPage } from "./ModelsPage";

interface SettingsPageProps {
  locale: Locale; onLocaleChange: (locale: Locale) => void; textScale: TextScale;
  onTextScaleChange: (scale: TextScale) => void; workspace: MobileWorkspaceDto;
  providers: ModelProviderDto[]; onSaveProvider: (input: ProviderInput) => Promise<void>;
  onTestProvider: (input: ProviderInput) => Promise<ConnectionTestResult>;
  onSaveMobile: (settings: MobileRuntimeSettingsDto) => Promise<void>;
  onStrategyChange: (strategyId: string, enabled: boolean) => Promise<void>; t: Translator;
}

export function SettingsPage({ locale, onLocaleChange, textScale, onTextScaleChange, workspace, providers, onSaveProvider, onTestProvider, onSaveMobile, onStrategyChange, t }: SettingsPageProps) {
  const [sdk, setSdk] = useState(workspace.settings.androidSdk ?? "");
  const [apps, setApps] = useState(workspace.settings.allowedApps.join("\n"));
  useEffect(() => { setSdk(workspace.settings.androidSdk ?? ""); setApps(workspace.settings.allowedApps.join("\n")); }, [workspace.settings]);
  const saveMobile = () => onSaveMobile({ ...workspace.settings, androidSdk: sdk || undefined, allowedApps: apps.split(/\n|,/).map((value) => value.trim()).filter(Boolean), textScale });
  return <section className="page settings-page"><header className="page-header"><span className="eyebrow">{t("settings.runtimeEyebrow")}</span><h1>{t("nav.settings")}</h1><p>{t("settings.mobileDescription")}</p></header><div className="settings-list"><article className="panel setting-row"><div className="setting-icon"><Globe2 size={18}/></div><div><h2>{t("settings.language")}</h2><p>{t("settings.languageDetail")}</p></div><select value={locale} onChange={(event) => onLocaleChange(event.target.value as Locale)}><option value="en">English</option><option value="zh-CN">简体中文</option></select></article><article className="panel setting-row"><div className="setting-icon"><Type size={18}/></div><div><h2>{t("settings.textSize")}</h2><p>{t("settings.textSizeDetail")}</p></div><select value={textScale} onChange={(event) => onTextScaleChange(Number(event.target.value) as TextScale)}>{[90,100,110,120].map((scale) => <option key={scale} value={scale}>{scale}%</option>)}</select></article></div><section className="settings-section"><header><MonitorSmartphone size={18}/><div><h2>{t("settings.mobileRuntime")}</h2><p>{t("settings.adbStatus", { status: workspace.adbStatus })}</p></div></header><div className="panel mobile-settings"><label><span>{t("settings.sdkPath")}</span><input value={sdk} onChange={(event) => setSdk(event.target.value)} placeholder={t("settings.sdkPlaceholder")}/></label><label><span>{t("settings.allowedPackages")}</span><textarea value={apps} onChange={(event) => setApps(event.target.value)} placeholder="com.twitter.android"/></label><div className="budget-grid"><span>Duration {workspace.settings.researchBudget.maxDurationSeconds}s</span><span>Steps {workspace.settings.researchBudget.maxSteps}</span><span>Observations {workspace.settings.researchBudget.maxObservations}</span><span>Model calls {workspace.settings.researchBudget.maxModelCalls}</span></div><button className="gold-button" type="button" onClick={() => void saveMobile()}>{t("settings.saveMobile")}</button></div></section><section className="settings-section"><header><Database size={18}/><div><h2>{t("settings.strategies")}</h2><p>{t("settings.strategyDetail")}</p></div></header><div className="strategy-grid">{workspace.strategies.map((strategy) => <article className="panel strategy-row" key={`${strategy.id}-${strategy.version}`}><div><strong>{strategy.id}</strong><span>{strategy.market} · {strategy.category} · {strategy.readiness}</span></div><label className="switch"><input type="checkbox" checked={strategy.enabled} onChange={(event) => void onStrategyChange(strategy.id, event.target.checked)}/><span>{strategy.enabled ? t("common.enabled") : t("common.pending")}</span></label></article>)}</div></section><section className="settings-section model-settings"><ModelsPage providers={providers} onSave={onSaveProvider} onTest={onTestProvider} t={t}/></section></section>;
}
