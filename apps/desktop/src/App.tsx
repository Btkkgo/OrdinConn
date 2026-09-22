import { useCallback, useEffect, useMemo, useReducer, useState } from "react";
import type { AgentMessageDto, AppSnapshotDto, IntelligenceItemDto, MobileActionInputDto, MobileWorkspaceDto } from "@ordinconn/contracts";
import { AppShell } from "./components/AppShell";
import { DataDetailModal } from "./components/DataDetailModal";
import type { PageId } from "./components/Navigation";
import { createTranslator, localeStorageKey, resolveLocale, type Locale } from "./i18n";
import { runtimeClient, subscribeRuntimeEvents, type ProviderInput } from "./runtime/client";
import { applyTextScale, normalizeTextScale, textScaleStorageKey, type TextScale } from "./runtime/preferences";
import { createPageContext, initialRuntimeState, reduceMobileWorkspaceEvent, reduceRuntimeEvent } from "./runtime/state";
import { MobileHomePage } from "./pages/MobileHomePage";
import { SettingsPage } from "./pages/SettingsPage";
import { WarehousePage } from "./pages/WarehousePage";
import iconUrl from "./assets/ordinconn-icon-source.png";

const emptySnapshot: AppSnapshotDto = { signals: [], connectors: [], providers: [], pendingApprovals: [], recentExecutions: [] };
const emptyMobile: MobileWorkspaceDto = { runtimeStatus: "unavailable", adbStatus: "missing", androidEnvironment: { sdkStatus: "missing", adbStatus: "missing", emulatorStatus: "missing", availableAvds: [], onlineDevices: [] }, observations: [], feed: [], warehouse: [], strategies: [], settings: { allowedApps: [], screenshotRetention: "memory_only", textScale: 100, researchBudget: { maxDurationSeconds: 300, maxSteps: 40, maxScrolls: 12, maxPages: 20, maxObservations: 50, maxModelCalls: 10 } } };
function errorMessage(error: unknown): string { return typeof error === "string" ? error : error && typeof error === "object" && "message" in error && typeof error.message === "string" ? error.message : ""; }

export default function App() {
  const [locale, setLocale] = useState<Locale>(() => { try { return resolveLocale(localStorage.getItem(localeStorageKey)); } catch { return "en"; } });
  const [textScale, setTextScale] = useState<TextScale>(() => { try { return normalizeTextScale(Number(localStorage.getItem(textScaleStorageKey))); } catch { return 100; } });
  const t = useMemo(() => createTranslator(locale), [locale]);
  const [page, setPage] = useState<PageId>("home");
  const [snapshot, setSnapshot] = useState(emptySnapshot);
  const [mobile, setMobile] = useState(emptyMobile);
  const [selectedItemId, setSelectedItemId] = useState<string>();
  const [detail, setDetail] = useState<IntelligenceItemDto>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [runtimeState, dispatchRuntimeEvent] = useReducer(reduceRuntimeEvent, initialRuntimeState);
  const [localMessages, setLocalMessages] = useState<AgentMessageDto[]>([]);
  const [threadId, setThreadId] = useState<string>();
  const refresh = useCallback(async () => { const [nextSnapshot, nextMobile] = await Promise.all([runtimeClient.getSnapshot(), runtimeClient.getMobileWorkspace()]); setSnapshot(nextSnapshot); setMobile(nextMobile); }, []);
  const run = async <T,>(operation: () => Promise<T>): Promise<T | undefined> => { setError(""); try { return await operation(); } catch (cause) { setError(errorMessage(cause) || t("common.error")); } };
  useEffect(() => { document.documentElement.lang = locale; applyTextScale(textScale); }, [locale, textScale]);
  useEffect(() => { void refresh().catch((cause) => setError(errorMessage(cause) || t("common.error"))).finally(() => setLoading(false)); let disposed = false; let unlisten: (() => void) | undefined; void subscribeRuntimeEvents((event) => { dispatchRuntimeEvent(event); setMobile((current) => reduceMobileWorkspaceEvent(current, event)); }).then((stop) => { if (disposed) stop(); else unlisten = stop; }); return () => { disposed = true; unlisten?.(); }; }, [refresh, t]);
  const changeLocale = (next: Locale) => { setLocale(next); try { localStorage.setItem(localeStorageKey, next); } catch { /* in-memory preference remains active */ } };
  const changeTextScale = (next: TextScale) => { const normalized = normalizeTextScale(next); setTextScale(normalized); try { localStorage.setItem(textScaleStorageKey, String(normalized)); } catch { /* in-memory preference remains active */ } };
  const observe = async () => { const next = await run(runtimeClient.observeMobileDevice); if (next) { setMobile(next); setSelectedItemId(next.feed[0]?.id); } };
  const stopMobile = async () => { const next = await run(runtimeClient.stopMobileSession); if (next) setMobile(next); };
  const actMobile = async (input: MobileActionInputDto) => { const result = await run(() => runtimeClient.executeMobileAction(input)); if (result) { setMobile(result.workspace); setSelectedItemId(result.workspace.feed[0]?.id); } };
  const warehouseChange = async (favorite: boolean, saved: boolean, tags: string[]) => { if (!detail) return; if (await run(() => runtimeClient.setWarehouseEntry(detail.id, favorite, saved, tags))) { await refresh(); setDetail((current) => current ? { ...current, favorite, saved } : current); } };
  const discuss = async (question: string) => { if (!detail) return; const message: AgentMessageDto = { id: crypto.randomUUID(), role: "user", content: question, createdAt: new Date().toISOString() }; setLocalMessages((current) => [...current, message]); const started = await run(() => runtimeClient.startAgentTurn(threadId, question, createPageContext(page, undefined, detail))); if (started) setThreadId(started.threadId); };
  const research = async () => { if (!detail) return; await run(() => runtimeClient.createMobileResearchTask(`Research: ${detail.title}`, mobile.settings.allowedApps.length ? mobile.settings.allowedApps : [detail.sourceApp], mobile.settings.researchBudget)); };
  const saveProvider = async (input: ProviderInput) => { if (await run(() => runtimeClient.saveModelProvider(input))) await refresh(); };
  const content = page === "home" ? <MobileHomePage workspace={mobile} signals={snapshot.signals} selectedItemId={selectedItemId} onSelectItem={setSelectedItemId} onObserve={() => void observe()} onStop={() => void stopMobile()} onAction={actMobile} onOpenDetail={setDetail} t={t}/> : page === "warehouse" ? <WarehousePage workspace={mobile} onOpenDetail={setDetail} t={t}/> : <SettingsPage locale={locale} onLocaleChange={changeLocale} textScale={textScale} onTextScaleChange={changeTextScale} workspace={mobile} providers={snapshot.providers} onSaveProvider={saveProvider} onTestProvider={runtimeClient.testModelProvider} onSaveMobile={async (settings) => { await run(() => runtimeClient.saveMobileSettings(settings)); await refresh(); }} onStartAvd={async (name) => { const next = await run(() => runtimeClient.startMobileAvd(name)); if (next) setMobile(next); }} onStrategyChange={async (strategyId, enabled) => { await run(() => runtimeClient.setStrategyEnabled(strategyId, enabled)); await refresh(); }} t={t}/>;
  if (loading) return <div className="startup-state"><div className="startup-mark"><img src={iconUrl} alt=""/></div><p>{t("common.loading")}</p></div>;
  return <>{error ? <div className="error-banner" role="alert"><span>{error}</span><button type="button" onClick={() => setError("")}>{t("common.close")}</button></div> : null}<AppShell page={page} onNavigate={setPage} workspace={content} t={t}/>{detail ? <DataDetailModal item={mobile.feed.find((item) => item.id === detail.id) ?? detail} signals={snapshot.signals.filter((signal) => detail.relatedSignalIds.includes(signal.id) || detail.assets.includes(signal.asset))} messages={[...localMessages, ...runtimeState.agentMessages]} onClose={() => setDetail(undefined)} onWarehouseChange={(favorite, saved, tags) => void warehouseChange(favorite, saved, tags)} onDiscuss={(question) => void discuss(question)} onResearch={() => void research()} t={t}/> : null}<div className="sr-only" aria-live="polite">{runtimeState.agentMessages.length + localMessages.length} agent messages</div></>;
}
