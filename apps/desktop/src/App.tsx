import { useCallback, useEffect, useMemo, useReducer, useState } from "react";
import type { AgentMessageDto, AgentReportDto, AppSnapshotDto, ApprovalRequestDto, ExecutionRecordDto, SignalDto, TradeProposalDto } from "@ordinconn/contracts";
import { AppShell } from "./components/AppShell";
import { AgentDock } from "./components/AgentDock";
import { SignalDetail } from "./components/SignalDetail";
import type { PageId } from "./components/Navigation";
import { createTranslator } from "./i18n";
import { runtimeClient, subscribeRuntimeEvents, type ProviderInput } from "./runtime/client";
import { createPageContext, initialRuntimeState, reduceRuntimeEvent } from "./runtime/state";
import { AgentPage } from "./pages/AgentPage";
import { ApprovalsPage } from "./pages/ApprovalsPage";
import { AutomationsPage } from "./pages/AutomationsPage";
import { CryptoPage } from "./pages/CryptoPage";
import { DataSourcesPage } from "./pages/DataSourcesPage";
import { ModelsPage } from "./pages/ModelsPage";
import { OverviewPage } from "./pages/OverviewPage";
import { SettingsPage } from "./pages/SettingsPage";
import { SignalsPage } from "./pages/SignalsPage";
import { TraditionalFinancePage } from "./pages/TraditionalFinancePage";
import logoUrl from "./assets/ordinconn-logo-source.jpg";

const emptySnapshot: AppSnapshotDto = { signals: [], connectors: [], providers: [], pendingApprovals: [], recentExecutions: [] };

function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error && typeof error.message === "string") return error.message;
  return "";
}

export default function App() {
  const t = useMemo(() => createTranslator("en"), []);
  const [page, setPage] = useState<PageId>("overview");
  const [snapshot, setSnapshot] = useState<AppSnapshotDto>(emptySnapshot);
  const [selectedSignal, setSelectedSignal] = useState<SignalDto>();
  const [dockCollapsed, setDockCollapsed] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [runtimeState, dispatchRuntimeEvent] = useReducer(reduceRuntimeEvent, initialRuntimeState);
  const [localMessages, setLocalMessages] = useState<AgentMessageDto[]>([]);
  const [threadId, setThreadId] = useState<string>();
  const [turnId, setTurnId] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<AgentReportDto>();
  const [proposal, setProposal] = useState<TradeProposalDto>();
  const [approval, setApproval] = useState<ApprovalRequestDto>();
  const [execution, setExecution] = useState<ExecutionRecordDto>();

  const refresh = useCallback(async () => setSnapshot(await runtimeClient.getSnapshot()), []);
  useEffect(() => {
    refresh().catch((cause) => setError(errorMessage(cause) || t("common.error"))).finally(() => setLoading(false));
    let disposed = false;
    let unlisten: (() => void) | undefined;
    subscribeRuntimeEvents((event) => {
      dispatchRuntimeEvent(event);
      if (event.type === "agent.message_completed" || event.type === "agent.turn_failed") setBusy(false);
      if (event.family === "approval" || event.family === "execution") void refresh();
    }).then((stop) => { if (disposed) stop(); else unlisten = stop; }).catch((cause) => setError(errorMessage(cause) || t("common.error")));
    return () => { disposed = true; unlisten?.(); };
  }, [refresh, t]);

  const messages = [...localMessages, ...runtimeState.agentMessages];
  const navigate = (next: PageId) => { setPage(next); setSelectedSignal(undefined); };
  const openSignal = (signal: SignalDto) => {
    setSelectedSignal(signal);
    setReport(undefined); setProposal(undefined); setApproval(undefined); setExecution(undefined);
  };
  const run = async <T,>(operation: () => Promise<T>): Promise<T | undefined> => {
    setError("");
    try { return await operation(); } catch (cause) { setError(errorMessage(cause) || t("common.error")); return undefined; }
  };
  const send = async (question: string) => {
    const message: AgentMessageDto = { id: crypto.randomUUID(), role: "user", content: question, createdAt: new Date().toISOString() };
    setLocalMessages((current) => [...current, message]); setBusy(true);
    const started = await run(() => runtimeClient.startAgentTurn(threadId, question, createPageContext(page, selectedSignal)));
    if (started) { setThreadId(started.threadId); setTurnId(started.turnId); } else setBusy(false);
  };
  const createReport = async () => { if (!selectedSignal) return; const next = await run(() => runtimeClient.createReport(selectedSignal.id)); if (next) setReport(next); };
  const createProposal = async () => { if (!selectedSignal) return; const next = await run(() => runtimeClient.createTradeProposal(selectedSignal.id)); if (next) setProposal(next); };
  const requestApproval = async () => { if (!proposal) return; const next = await run(() => runtimeClient.requestApproval(proposal.id)); if (next) { setApproval(next); await refresh(); } };
  const approve = async (target = approval) => { if (!target) return; const next = await run(() => runtimeClient.approveAndExecutePaper(target.id)); if (next) { setExecution(next); await refresh(); } };
  const saveProvider = async (input: ProviderInput) => { const saved = await run(() => runtimeClient.saveModelProvider(input)); if (saved) await refresh(); };

  let workspace;
  if (selectedSignal) workspace = <SignalDetail signal={selectedSignal} onBack={() => setSelectedSignal(undefined)} t={t} />;
  else if (page === "overview") workspace = <OverviewPage snapshot={snapshot} agentActivity={messages.length} reportReady={Boolean(report)} onOpenSignal={openSignal} t={t} />;
  else if (page === "traditional") workspace = <TraditionalFinancePage signals={snapshot.signals} onOpenSignal={openSignal} t={t} />;
  else if (page === "crypto") workspace = <CryptoPage signals={snapshot.signals} onOpenSignal={openSignal} t={t} />;
  else if (page === "signals") workspace = <SignalsPage signals={snapshot.signals} onOpenSignal={openSignal} t={t} />;
  else if (page === "agent") workspace = <AgentPage messages={messages} t={t} />;
  else if (page === "automations") workspace = <AutomationsPage t={t} />;
  else if (page === "models") workspace = <ModelsPage providers={snapshot.providers} onSave={saveProvider} onTest={runtimeClient.testModelProvider} t={t} />;
  else if (page === "dataSources") workspace = <DataSourcesPage connectors={snapshot.connectors} t={t} />;
  else if (page === "approvals") workspace = <ApprovalsPage approvals={snapshot.pendingApprovals} onApprove={approve} t={t} />;
  else workspace = <SettingsPage t={t} />;

  if (loading) return <div className="startup-state"><div className="startup-mark"><img src={logoUrl} alt="" /></div><p>{t("common.loading")}</p></div>;
  return (
    <>
      {error ? <div className="error-banner" role="alert"><span>{error}</span><button onClick={() => setError("")} type="button">{t("common.close")}</button></div> : null}
      <AppShell page={page} signal={selectedSignal} onNavigate={navigate} workspace={workspace} dockCollapsed={dockCollapsed} t={t} dock={<AgentDock collapsed={dockCollapsed} signal={selectedSignal} messages={messages} busy={busy} report={report} proposal={proposal} approval={approval} execution={execution} onToggle={() => setDockCollapsed((current) => !current)} onSend={send} onReport={createReport} onProposal={createProposal} onRequestApproval={requestApproval} onApprove={() => approve()} t={t} />} />
      {turnId && busy ? <button className="cancel-task" onClick={() => { void runtimeClient.cancelAgentTurn(turnId); setBusy(false); }} type="button">{t("common.cancel")}</button> : null}
    </>
  );
}
