import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AgentMessageDto,
  AgentReportDto,
  AppSnapshotDto,
  ApprovalRequestDto,
  ExecutionRecordDto,
  ModelProviderDto,
  ProviderCapabilitiesDto,
  RuntimeEventEnvelope,
  TradeProposalDto,
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
};

export async function subscribeRuntimeEvents(
  handler: (event: RuntimeEventEnvelope) => void,
): Promise<UnlistenFn> {
  return listen("ordinconn://runtime-event", ({ payload }) => {
    handler(parseRuntimeEventEnvelope(payload));
  });
}
