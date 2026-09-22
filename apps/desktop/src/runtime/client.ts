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
  observeMobileDevice: () => invoke<MobileWorkspaceDto>("observe_mobile_device"),
  executeMobileAction: (input: MobileActionInputDto) => invoke<MobileActionResultDto>("execute_mobile_action", { input }),
  stopMobileSession: () => invoke<MobileWorkspaceDto>("stop_mobile_session"),
  startMobileAvd: (name: string) => invoke<MobileWorkspaceDto>("start_mobile_avd", { name }),
  setWarehouseEntry: (itemId: string, favorite: boolean, saved: boolean, tags: string[]) =>
    invoke<WarehouseEntryDto>("set_warehouse_entry", { itemId, favorite, saved, tags }),
  createMobileResearchTask: (query: string, allowedApps: string[], budget: MobileResearchBudgetDto) =>
    invoke("create_mobile_research_task", { query, allowedApps, budget }),
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
