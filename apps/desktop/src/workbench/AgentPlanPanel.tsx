import type { MobileGoalDto, MobileGoalPlanDto } from "@ordinconn/contracts";
import { EmptyState } from "../components/WorkspaceUI";
import type { Translator } from "../i18n";
import type { AgentPlan } from "./model";
export function AgentPlanPanel({ plans, goals = [], goalPlans = {}, onDismiss, fixtureHandledNote = false, t }: { fixtureHandledNote?: boolean; plans: AgentPlan[]; goals?: MobileGoalDto[]; goalPlans?: Record<string, MobileGoalPlanDto | null>; onDismiss?: (id: string) => void; t: Translator }) {
  const visible = plans.filter(plan => plan.status !== "dismissed");
  return <section className="workbench-panel plan-panel" aria-label={t("workbench.plans")}><header className="workbench-panel-header"><div><h2>{t("workbench.plans")}</h2><p>{t("workbench.plansSubtitle")}</p></div></header>
    {goals.map(goal => { const steps = goalPlans[goal.id]?.steps ?? []; const step = steps.find(item => ["EXECUTING", "WAITING_APPROVAL", "PENDING"].includes(item.status)) ?? steps.at(-1); return <article className="workbench-plan" key={goal.id} data-goal-status={goal.status} data-goal-id={goal.id}><header><span>{t(`workbench.goalStatus.${goal.status}`)}</span><strong>{t("workbench.priority.medium")}</strong></header><h3>{goal.objective}</h3><p>{t(step ? `workbench.stepStatus.${step.status}` : goal.status === "PENDING" && !goal.activePlanId ? "workbench.waitingPlanning" : goal.status === "PLANNING" && goal.activePlanId ? "workbench.waitingExecutor" : "workbench.goalSaved")}</p></article>; })}
    {visible.map(plan => <article className="workbench-plan" key={plan.id} data-plan-status={plan.status}><header><span>{t(`workbench.planStatus.${plan.status}`)}</span><strong>{t(`workbench.priority.${plan.priority}`)}</strong></header><h3>{plan.title}</h3><p>{plan.summaryKey ? t(plan.summaryKey) : plan.summary}</p>{plan.sourceObjectIds.length ? <small>{t("workbench.planSources", { count: plan.sourceObjectIds.length })}</small> : null}{onDismiss ? <button type="button" onClick={() => onDismiss(plan.id)}>{t("workbench.dismiss")}</button> : null}</article>)}
    {fixtureHandledNote || (!goals.length && (!visible.length || visible.every(plan => plan.status === "completed"))) ? <EmptyState title={t("workbench.plansEmpty")} description={t("workbench.plansEmptyDescription")} /> : null}
  </section>;
}
