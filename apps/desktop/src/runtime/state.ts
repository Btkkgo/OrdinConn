import type { AgentMessageDto, RuntimeEventEnvelope, SignalDto } from "@ordinconn/contracts";

export interface PageContext {
  page: string;
  market?: string;
  asset?: string;
  signalId?: string;
  evidenceIds: string[];
}

export interface RuntimeUiState {
  activeThreadId?: string;
  activeTurnId?: string;
  agentMessages: AgentMessageDto[];
  lastError?: string;
}

export const initialRuntimeState: RuntimeUiState = { agentMessages: [] };

export function createPageContext(page: string, signal?: SignalDto): PageContext {
  return {
    page,
    market: signal?.market,
    asset: signal?.asset,
    signalId: signal?.id,
    evidenceIds: signal?.evidence.map((item) => item.id) ?? [],
  };
}

export function reduceRuntimeEvent(state: RuntimeUiState, event: RuntimeEventEnvelope): RuntimeUiState {
  if (event.type === "agent.message_completed") {
    const payload = event.payload as {
      itemId: string;
      content: string;
      modelId: string;
      mock: boolean;
    };
    return {
      ...state,
      activeThreadId: event.threadId ?? state.activeThreadId,
      activeTurnId: undefined,
      agentMessages: [
        ...state.agentMessages,
        {
          id: payload.itemId,
          role: "assistant",
          content: payload.content,
          createdAt: event.occurredAt,
          modelId: payload.modelId,
          mock: payload.mock,
        },
      ],
    };
  }
  if (event.type === "agent.turn_failed") {
    return { ...state, activeTurnId: undefined, lastError: event.type };
  }
  return state;
}
