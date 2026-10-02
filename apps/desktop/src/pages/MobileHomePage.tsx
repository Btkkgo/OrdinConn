import { MobileDataStream, MobileObservationContext } from "../components/MobileCollectionPanels";
import type { MobileDataProvenance } from "@ordinconn/contracts";
import { useMemo, useState } from "react";
import type { ReactNode } from "react";
import type { IntelligenceItemDto, MobileGoalDto, MobileGoalPlanDto, MobileActionInputDto, MobileWorkspaceDto, SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { MobileDeviceView } from "../components/MobileDeviceView";
import { WorkbenchDataBrowser } from "../workbench/WorkbenchDataBrowser";
import { useWorkbenchCommands } from "../workbench/useWorkbenchCommands";
import { type WorkbenchRuntimePort } from "../workbench/commands";
import { buildWorkbenchModel, type WorkbenchViewModel } from "../workbench/model";
import { WorkbenchHeader, WorkbenchLegend } from "../workbench/WorkbenchHeader";
import { RealtimeDataPanel } from "../workbench/RealtimeDataPanel";
import { AgentPlanPanel } from "../workbench/AgentPlanPanel";
import { AgentCommandPanel } from "../workbench/AgentCommandPanel";

export interface MobileHomePageProps {
  manualBusy?: boolean;
  workspace: MobileWorkspaceDto; signals: SignalDto[]; selectedItemId?: string;
  onSelectItem: (id: string) => void; onObserve: () => void; onStop: () => void;
  onAction: (input: MobileActionInputDto) => Promise<void>;
  onOpenDetail: (item: IntelligenceItemDto) => void; t: Translator;
  onExtract?: () => Promise<void>;
  onProvenance?: (id:string) => Promise<MobileDataProvenance>;
  goals?: MobileGoalDto[];
  goalPlans?: Record<string, MobileGoalPlanDto | null>;
  runtime?: WorkbenchRuntimePort;
  visualModel?: WorkbenchViewModel; fixtureScreen?: ReactNode;
}
export function MobileHomePage({ manualBusy = false, workspace, onSelectItem, onObserve, onStop, onAction, onOpenDetail, t, visualModel, fixtureScreen, runtime, onExtract, onProvenance, goals = [], goalPlans = {} }: MobileHomePageProps) {
  const controller = useWorkbenchCommands(runtime);
  const [dismissed, setDismissed] = useState<string[]>([]);
  const [browse, setBrowse] = useState(false);
  const [inspect, setInspect] = useState(!!workspace.collection);
  const [selectedMetric, setSelectedMetric] = useState("other");
  const model = useMemo(() => {
    if (visualModel) return { ...visualModel, selectedMetric };
    const plans = [...new Map(controller.plans.map(plan => [plan.id, plan])).values()]
      .filter(plan => !dismissed.includes(plan.id)).map(plan => ({ ...plan, title: plan.title.startsWith("workbench.") ? t(plan.title) : plan.title }));
    const live = buildWorkbenchModel(workspace, selectedMetric, plans);
    return { ...live, agentState: controller.phase && live.device.connected ? controller.phase : live.agentState };
  }, [workspace, selectedMetric, visualModel, controller.plans, controller.phase, dismissed, t]);
  const categoryItems = workspace.feed.filter(item => model.objects.some(object => object.id === item.id && object.category === model.selectedMetric));
  const openData = () => setBrowse(true);
  if(workspace.collection && !visualModel) {
    const connected=workspace.runtimeStatus==="observing" && !!workspace.session;
    const panels={collection:workspace.collection,t,connected,onExtract:onExtract??(async()=>{throw new Error("UNAVAILABLE");}),onProvenance:onProvenance??(async()=>{throw new Error("UNAVAILABLE");})};
    return <section className="realtime-workbench mobile-acquisition-workbench" data-data-mode="live"><WorkbenchHeader model={model} t={t}/><div className="workbench-columns">
      <MobileDataStream {...panels}/>
      <section className="workbench-panel mobile-operation-panel" aria-label={t("workbench.mobile")}><header className="workbench-panel-header"><div><h2>{t("workbench.mobile")}</h2><p>{workspace.session?.deviceId??t("workbench.noDevice")}</p></div></header>
        <MobileDeviceView manualBusy={manualBusy} compact observeBeforeAction frame={workspace.frame} snapshot={workspace.uiSnapshot} session={workspace.session} allowedApps={workspace.settings.allowedApps} latestActionReceipt={workspace.latestActionReceipt} adbStatus={workspace.adbStatus} inspect={inspect} onInspectChange={setInspect} onObserve={onObserve} onStop={onStop} onAction={onAction} t={t}>
          <div className="mobile-action-row"><button className="secondary-button" type="button" disabled={manualBusy} aria-busy={manualBusy} onClick={onObserve}>{t("mobile.observeNow")}</button><button className="secondary-button" type="button" onClick={onStop}>{t("collection.action.stop")}</button><button className="secondary-button" type="button" disabled={!connected || !workspace.collection.observations.length} onClick={()=>void panels.onExtract().catch(()=>undefined)}>{t("collection.extractPage")}</button></div>
        </MobileDeviceView>
      </section><MobileObservationContext {...panels}/>
    </div></section>;
  }
  return <section className="realtime-workbench" data-data-mode={model.mode}>
    {model.mode === "visual_fixture" ? <span className="fixture-badge">{t("workbench.fixture")}</span> : null}
    <WorkbenchHeader model={model} t={t}/>
    <div className="workbench-columns">
      <RealtimeDataPanel model={model} onSelect={setSelectedMetric} onOpenData={openData} t={t}/>
      <section className="workbench-panel mobile-operation-panel" aria-label={t("workbench.mobile")}>
        <header className="workbench-panel-header"><div><h2>{t("workbench.mobile")}</h2><p>{model.device.name ?? t("workbench.noDevice")} · {t("workbench.capabilities")}</p></div><span className="workbench-agent-state">{t(`workbench.agent.${model.agentState}`)}</span></header>
        <MobileDeviceView compact fixtureScreen={fixtureScreen} frame={workspace.frame} snapshot={workspace.uiSnapshot} session={workspace.session} allowedApps={workspace.settings.allowedApps} latestActionReceipt={workspace.latestActionReceipt} adbStatus={workspace.adbStatus} inspect={inspect} onInspectChange={setInspect} onObserve={onObserve} onStop={onStop} onAction={onAction} t={t}>
          <AgentCommandPanel disabled={!model.device.connected || model.mode === "visual_fixture"} busy={controller.busy} onCommand={command => { if(runtime) void controller.command(command); else { if(command === "observe") onObserve(); if(command === "stop") onStop(); } }} onSubmit={goal => void controller.submit(goal)} t={t}/>
        </MobileDeviceView>
      </section>
      <AgentPlanPanel goalPlans={goalPlans} fixtureHandledNote={model.mode === "visual_fixture"} plans={model.plans} goals={visualModel ? [] : [...new Map([...controller.goals, ...goals].map(goal => [goal.id, goal])).values()]} onDismiss={model.mode === "live" ? id => setDismissed(current => [...current, id]) : undefined} t={t}/>
    </div>
    {controller.notice ? <p className={["workbench.extracted", "workbench.collected", "workbench.stopped"].includes(controller.notice) ? "sr-only" : "workbench-notice"} role="status">{controller.notice.startsWith("workbench.") ? t(controller.notice) : controller.notice}</p> : null}
    <WorkbenchLegend t={t}/>
    {browse ? <WorkbenchDataBrowser items={categoryItems} onClose={() => setBrowse(false)} onSelect={item => { setBrowse(false); onSelectItem(item.id); onOpenDetail(item); }} t={t}/> : null}
  </section>;
}
