import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import type { AgentMessageDto, AppSnapshotDto, IntelligenceItemDto, MobileActionInputDto, MobileGoalDto, MobileGoalPlanDto, MobileWorkspaceDto } from "@ordinconn/contracts";
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
import { Alert } from "./components/WorkspaceUI";

const emptySnapshot: AppSnapshotDto = { signals: [], connectors: [], providers: [], pendingApprovals: [], recentExecutions: [] };
const emptyMobile: MobileWorkspaceDto = { runtimeStatus: "unavailable", adbStatus: "missing", androidEnvironment: { sdkStatus: "missing", adbStatus: "missing", emulatorStatus: "missing", availableAvds: [], onlineDevices: [] }, observations: [], feed: [], warehouse: [], strategies: [], settings: { allowedApps: [], screenshotRetention: "memory_only", textScale: 100, researchBudget: { maxDurationSeconds: 300, maxSteps: 40, maxScrolls: 12, maxPages: 20, maxObservations: 50, maxModelCalls: 10 } } };
function errorMessage(error: unknown): string { return typeof error === "string" ? error : error && typeof error === "object" && "message" in error && typeof error.message === "string" ? error.message : ""; }

export default function App() {
  const [locale, setLocale] = useState<Locale>(() => { try { return resolveLocale(localStorage.getItem(localeStorageKey)); } catch { return "zh-CN"; } });
  const [textScale, setTextScale] = useState<TextScale>(() => { try { return normalizeTextScale(Number(localStorage.getItem(textScaleStorageKey))); } catch { return 100; } });
  const t = useMemo(() => createTranslator(locale), [locale]);
  const [page, setPage] = useState<PageId>("home");
  const [snapshot, setSnapshot] = useState(emptySnapshot);
  const [mobile, setMobile] = useState(emptyMobile);
  const [goals, setGoals] = useState<MobileGoalDto[]>([]);
  const [goalPlans, setGoalPlans] = useState<Record<string, MobileGoalPlanDto | null>>({});
  const [selectedItemId, setSelectedItemId] = useState<string>();
  const [detail, setDetail] = useState<IntelligenceItemDto>();
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [runtimeState, dispatchRuntimeEvent] = useReducer(reduceRuntimeEvent, initialRuntimeState);
  const [localMessages, setLocalMessages] = useState<AgentMessageDto[]>([]);
  const [threadId, setThreadId] = useState<string>();
  const goalRefreshVersion = useRef(0);
  const refreshGoals = useCallback(async () => { const version = ++goalRefreshVersion.current; const nextGoals = await runtimeClient.listMobileGoals(); const plans = await Promise.all(nextGoals.filter(goal => goal.activePlanId).map(async goal => [goal.id, await runtimeClient.getMobileGoalPlan(goal.id)] as const)); if (version === goalRefreshVersion.current) { setGoals(nextGoals); setGoalPlans(Object.fromEntries(plans)); } }, []);
  const refresh = useCallback(async () => { const [nextSnapshot, nextMobile] = await Promise.all([runtimeClient.getSnapshot(), runtimeClient.getMobileWorkspace(), refreshGoals()]); setSnapshot(nextSnapshot); setMobile(nextMobile); }, [refreshGoals]);
  const run = async <T,>(operation: () => Promise<T>): Promise<T | undefined> => { setError(""); try { return await operation(); } catch (cause) { setError(errorMessage(cause) || t("common.error")); } };
  useEffect(() => { document.documentElement.lang = locale; applyTextScale(textScale); }, [locale, textScale]);
  useEffect(() => { void refresh().catch((cause) => setError(errorMessage(cause) || t("common.error"))).finally(() => setLoading(false)); let disposed = false; let unlisten: (() => void) | undefined; void subscribeRuntimeEvents((event) => { dispatchRuntimeEvent(event); setMobile((current) => reduceMobileWorkspaceEvent(current, event)); if (event.family === "mobile" && /^mobile\.(goal_|plan_|step_)/.test(event.type)) void refreshGoals().catch(() => setError(t("common.error"))); if (event.type === "mobile.data_updated") void runtimeClient.getMobileWorkspace().then(setMobile).catch(() => setError(t("common.error"))); }).then((stop) => { if (disposed) stop(); else unlisten = stop; }); return () => { disposed = true; unlisten?.(); }; }, [refresh, refreshGoals, t]);
  useEffect(() => {
    let disposed = false; let pending = false;
    const timer = setInterval(() => {
      if (pending) return;
      pending = true;
      void runtimeClient.getMobileWorkspace().then(next => { if (!disposed) setMobile(current => ({ ...current, adbStatus: next.adbStatus, androidEnvironment: next.androidEnvironment })); }).catch(() => { if (!disposed) setMobile(current => ({ ...current, adbStatus: "error" })); }).finally(() => { pending = false; });
    }, 5000);
    return () => { disposed = true; clearInterval(timer); };
  }, []);
  const changeLocale = (next: Locale) => { setLocale(next); try { localStorage.setItem(localeStorageKey, next); } catch { /* in-memory preference remains active */ } };
  const changeTextScale = (next: TextScale) => { const normalized = normalizeTextScale(next); setTextScale(normalized); try { localStorage.setItem(textScaleStorageKey, String(normalized)); } catch { /* in-memory preference remains active */ } };
  const observe = async () => { const next = await run(runtimeClient.observeMobileDevice); if (next) { setMobile(next); setSelectedItemId(next.feed[0]?.id); } };
  const stopMobile = async () => { const next = await run(runtimeClient.stopMobileSession); if (next) setMobile(next); };
  const actMobile = async (input: MobileActionInputDto) => { const result = await run(() => runtimeClient.executeMobileAction(input)); if (result) { setMobile(result.workspace); setSelectedItemId(result.workspace.feed[0]?.id); } };
  const warehouseChange = async (favorite: boolean, saved: boolean, tags: string[]) => { if (!detail) return; if (await run(() => runtimeClient.setWarehouseEntry(detail.id, favorite, saved, tags))) { await refresh(); setDetail((current) => current ? { ...current, favorite, saved } : current); } };
  const discuss = async (question: string) => { if (!detail) return; const message: AgentMessageDto = { id: crypto.randomUUID(), role: "user", content: question, createdAt: new Date().toISOString() }; setLocalMessages((current) => [...current, message]); const started = await run(() => runtimeClient.startAgentTurn(threadId, question, createPageContext(page, undefined, detail))); if (started) setThreadId(started.threadId); };
  const research = async () => { if (!detail) return; await run(() => runtimeClient.createMobileResearchTask(`Research: ${detail.title}`, mobile.settings.allowedApps.length ? mobile.settings.allowedApps : [detail.sourceApp], mobile.settings.researchBudget)); };
  const saveProvider = async (input: ProviderInput) => { if (await run(() => runtimeClient.saveModelProvider(input))) await refresh(); };
  const content = page === "home" ? <MobileHomePage workspace={mobile} goals={goals} goalPlans={goalPlans} signals={snapshot.signals} runtime={{
    observe: async () => { const next = await runtimeClient.observeMobileDevice(); setMobile(next); setSelectedItemId(next.feed[0]?.id); return next; },
    action: async input => { const result = await runtimeClient.executeMobileAction(input); setMobile(result.workspace); setSelectedItemId(result.workspace.feed[0]?.id); return result; },
    stop: async () => { setMobile(await runtimeClient.stopMobileSession()); },
    createGoal: async objective => { const goal = await runtimeClient.createMobileGoal({ objective }); setGoals(current => [goal, ...current.filter(item => item.id !== goal.id)]); try { return await runtimeClient.runMobileGoal(goal.id); } finally { await refreshGoals(); } },
  }} selectedItemId={selectedItemId} onSelectItem={setSelectedItemId} onObserve={() => void observe()} onStop={() => void stopMobile()} onAction={actMobile} onOpenDetail={setDetail} t={t}/> : page === "warehouse" ? <WarehousePage onGoHome={() => setPage("home")} workspace={mobile} onOpenDetail={setDetail} t={t}/> : <SettingsPage locale={locale} onLocaleChange={changeLocale} textScale={textScale} onTextScaleChange={changeTextScale} workspace={mobile} providers={snapshot.providers} onSaveProvider={saveProvider} onTestProvider={runtimeClient.testModelProvider} onSaveMobile={async (settings) => { await run(() => runtimeClient.saveMobileSettings(settings)); await refresh(); }} onStartAvd={async (name) => { const next = await run(() => runtimeClient.startMobileAvd(name)); if (next) setMobile(next); }} onStrategyChange={async (strategyId, enabled) => { await run(() => runtimeClient.setStrategyEnabled(strategyId, enabled)); await refresh(); }} t={t}/>;
  if (loading) return <div className="startup-state"><p>{t("common.loading")}</p></div>;
  return <>{error ? <div className="error-banner" role="alert"><Alert title={t("common.error")} detail={error} t={t} /><button type="button" onClick={() => setError("")}>{t("common.close")}</button></div> : null}<AppShell page={page} onNavigate={setPage} workspace={content} t={t}/>{detail ? <DataDetailModal item={mobile.feed.find((item) => item.id === detail.id) ?? detail} signals={snapshot.signals.filter((signal) => detail.relatedSignalIds.includes(signal.id) || detail.assets.includes(signal.asset))} messages={[...localMessages, ...runtimeState.agentMessages]} onClose={() => setDetail(undefined)} onWarehouseChange={(favorite, saved, tags) => void warehouseChange(favorite, saved, tags)} onDiscuss={(question) => void discuss(question)} onResearch={() => void research()} t={t}/> : null}<div className="sr-only" aria-live="polite">{runtimeState.agentMessages.length + localMessages.length} agent messages</div></>;
}
