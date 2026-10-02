import type { MobileInteractionResponse, MobileActionType, MobileDataProvenance } from "@ordinconn/contracts";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AgentMessageDto,
  AgentReportDto,
  AppSnapshotDto,
  ApprovalRequestDto,
  ExecutionRecordDto,
  ModelProviderDto,
  MobileResearchBudgetDto,
  MobileGoalDto,
  MobileGoalPlanDto,
  CreateMobileGoalInputDto,
  PlanMobileGoalInputDto,
  MobilePlannerOutcomeDto,
  MobileResearchTaskDto,
  MobileActionInputDto,
  MobileActionResultDto,
  MobileRuntimeSettingsDto,
  MobileWorkspaceDto,
  ProviderCapabilitiesDto,
  RuntimeEventEnvelope,
  TradeProposalDto,
  WarehouseEntryDto,
} from "@ordinconn/contracts";
import { parseRuntimeEventEnvelope } from "@ordinconn/contracts";
import type { PageContext } from "./state";

export interface AgentTaskStarted {
  threadId: string;
  turnId: string;
}

export interface ProviderInput {
  id?: string;
  name: string;
  baseUrl: string;
  apiKey?: string;
  modelId: string;
  temperature: number;
  contextWindow: number;
  enabled: boolean;
  capabilities: ProviderCapabilitiesDto;
}

export interface ConnectionTestResult {
  ok: boolean;
  message: string;
}

export const runtimeClient = {
  runMobileGoal: (goalId: string) => invoke<MobileGoalDto>("run_mobile_goal", { input: { goalId } }),
  getMobileStepApproval: (stepId: string) => invoke<ApprovalRequestDto | null>("get_mobile_step_approval", { stepId }),
  approveMobileGoalStep: (goalId: string, stepId: string, approvalId: string) => invoke<MobileGoalDto>("approve_mobile_goal_step", { input: { goalId, stepId, approvalId } }),
  createMobileGoal: (input: CreateMobileGoalInputDto) => invoke<MobileGoalDto>("create_mobile_goal", { input }),
  getMobileGoal: (goalId: string) => invoke<MobileGoalDto>("get_mobile_goal", { goalId }),
  listMobileGoals: () => invoke<MobileGoalDto[]>("list_mobile_goals"),
  executeMobileGoalStep: (goalId: string) => invoke<import("@ordinconn/contracts").MobileStepExecutionOutcomeDto>("execute_mobile_goal_step", { input: { goalId } }),
  planMobileGoal: (input: PlanMobileGoalInputDto) => invoke<MobilePlannerOutcomeDto>("plan_mobile_goal", { input }),
  getMobileGoalPlan: (goalId: string) => invoke<MobileGoalPlanDto | null>("get_mobile_goal_plan", { goalId }),
  getSnapshot: () => invoke<AppSnapshotDto>("get_snapshot"),
  startAgentTurn: (threadId: string | undefined, question: string, context: PageContext) =>
    invoke<AgentTaskStarted>("start_agent_turn", { threadId, question, context }),
  getThreadItems: (threadId: string) =>
    invoke<AgentMessageDto[]>("get_thread_items", { threadId }),
  cancelAgentTurn: (turnId: string) => invoke<boolean>("cancel_agent_turn", { turnId }),
  createReport: (signalId: string) => invoke<AgentReportDto>("create_report", { signalId }),
  createTradeProposal: (signalId: string) =>
    invoke<TradeProposalDto>("create_trade_proposal", { signalId }),
  requestApproval: (proposalId: string) =>
    invoke<ApprovalRequestDto>("request_approval", { proposalId }),
  approveAndExecutePaper: (approvalId: string) =>
    invoke<ExecutionRecordDto>("approve_and_execute_paper", { approvalId }),
  saveModelProvider: (input: ProviderInput) =>
    invoke<ModelProviderDto>("save_model_provider", { input }),
  testModelProvider: (input: ProviderInput) =>
    invoke<ConnectionTestResult>("test_model_provider", { input }),
  getMobileWorkspace: () => invoke<MobileWorkspaceDto>("get_mobile_workspace"),
  interactMobileDevice: (actionType:MobileActionType, action?:MobileActionInputDto) => invoke<MobileInteractionResponse>("interact_mobile_device", {input:{actionType,action}}),
  extractMobilePage: (observationId:string) => invoke<MobileWorkspaceDto>("extract_mobile_page", {observationId}),
  getMobileDataProvenance: (id:string) => invoke<MobileDataProvenance>("get_mobile_data_provenance", {id}),
  observeMobileDevice: () => invoke<MobileWorkspaceDto>("observe_mobile_device"),
  executeMobileAction: (input: MobileActionInputDto) => invoke<MobileActionResultDto>("execute_mobile_action", { input }),
  stopMobileSession: () => invoke<MobileWorkspaceDto>("stop_mobile_session"),
  startMobileAvd: (name: string) => invoke<MobileWorkspaceDto>("start_mobile_avd", { name }),
  setWarehouseEntry: (itemId: string, favorite: boolean, saved: boolean, tags: string[]) =>
    invoke<WarehouseEntryDto>("set_warehouse_entry", { itemId, favorite, saved, tags }),
  createMobileResearchTask: (query: string, allowedApps: string[], budget: MobileResearchBudgetDto) =>
    invoke<MobileResearchTaskDto>("create_mobile_research_task", { query, allowedApps, budget }),
  saveMobileSettings: (settings: MobileRuntimeSettingsDto) =>
    invoke<void>("save_mobile_settings", { settings }),
  setStrategyEnabled: (strategyId: string, enabled: boolean) =>
    invoke<void>("set_strategy_enabled", { strategyId, enabled }),
};

export async function subscribeRuntimeEvents(
  handler: (event: RuntimeEventEnvelope) => void,
): Promise<UnlistenFn> {
  return listen("ordinconn://runtime-event", ({ payload }) => {
    handler(parseRuntimeEventEnvelope(payload));
  });
}
