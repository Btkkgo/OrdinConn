export const MARKETS = ["traditional", "crypto"] as const;
export type Market = (typeof MARKETS)[number];

export const SIGNAL_CATEGORIES = [
  "trading",
  "event",
  "demand",
  "onchain",
  "exchange",
  "social",
] as const;
export type SignalCategory = (typeof SIGNAL_CATEGORIES)[number];

export const SIGNAL_DIRECTIONS = ["bullish", "bearish", "neutral", "watch"] as const;
export type SignalDirection = (typeof SIGNAL_DIRECTIONS)[number];

export const SIGNAL_STATUSES = [
  "new",
  "watching",
  "approved",
  "dismissed",
  "expired",
  "invalidated",
] as const;
export type SignalStatus = (typeof SIGNAL_STATUSES)[number];

export const APPROVAL_STATUSES = [
  "requested",
  "approved",
  "rejected",
  "expired",
  "cancelled",
  "revoked",
] as const;
export type ApprovalStatus = (typeof APPROVAL_STATUSES)[number];

export const EVENT_FAMILIES = [
  "agent",
  "model",
  "signal",
  "approval",
  "execution",
  "system",
] as const;
export type RuntimeEventFamily = (typeof EVENT_FAMILIES)[number];

export interface RuntimeEventEnvelope<T = Record<string, unknown>> {
  id: string;
  family: RuntimeEventFamily;
  type: string;
  aggregateId: string;
  threadId?: string;
  turnId?: string;
  occurredAt: string;
  payload: T;
}

export interface CommandError {
  code: string;
  message: string;
  retryable: boolean;
}

export type CommandResult<T> =
  | { ok: true; data: T }
  | { ok: false; error: CommandError };

export interface EvidenceDto {
  id: string;
  source: string;
  sourceType: string;
  market: Market;
  asset: string;
  title: string;
  content: string;
  capturedAt: string;
  freshness: number;
  reliability: number;
  factualLevel: string;
  confidence: number;
  relation: "primary" | "supporting" | "contradicting" | "context";
}

export interface SignalDto {
  id: string;
  market: Market;
  category: SignalCategory;
  asset: string;
  title: string;
  summary: string;
  direction: SignalDirection;
  confidence: number;
  urgency: number;
  timeHorizon: string;
  evidenceQuality: number;
  evidence: EvidenceDto[];
  catalysts: string[];
  risks: string[];
  invalidationConditions: string[];
  createdAt: string;
  updatedAt: string;
  agentId: string;
  modelId: string;
  status: SignalStatus;
}

export interface ConnectorDto {
  id: string;
  name: string;
  market: Market | "cross_market";
  connectorType: string;
  capabilities: string[];
  authType: string;
  status: "ready" | "degraded" | "unavailable";
  reliability: number;
  lastUpdate: string;
  mock: boolean;
}

export interface ProviderCapabilitiesDto {
  chatCompletions: boolean;
  responses: boolean;
  streaming: boolean;
  toolCalling: boolean;
  reasoning: boolean;
  vision: boolean;
  structuredOutput: boolean;
  jsonMode: boolean;
}

export interface ModelProviderDto {
  id: string;
  name: string;
  providerType: string;
  baseUrl: string;
  defaultModel: string;
  enabled: boolean;
  credentialConfigured: boolean;
  temperature: number;
  contextWindow: number;
  capabilities: ProviderCapabilitiesDto;
}

export interface AgentReportDto {
  id: string;
  signalId: string;
  executiveSummary: string;
  whatHappened: string;
  whyItMatters: string;
  evidence: string;
  inference: string;
  marketImpact: string;
  bullCase: string;
  bearCase: string;
  risk: string;
  invalidation: string;
  timeHorizon: string;
  possibleActions: string;
  watchConditions: string;
  conclusion: string;
  createdAt: string;
}

export interface AgentMessageDto {
  id: string;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  createdAt: string;
  modelId?: string;
  mock?: boolean;
}

export interface TradeProposalDto {
  id: string;
  signalId: string;
  version: number;
  market: Market;
  instrument: string;
  action: "buy" | "sell" | "open_long" | "open_short" | "close";
  orderType: "market" | "limit";
  quantity: string;
  price?: string;
  stopLoss?: string;
  takeProfit?: string;
  reason: string;
  riskSummary: string;
  status: "draft" | "waiting_approval" | "approved" | "rejected" | "expired" | "executed" | "failed";
  createdAt: string;
}

export interface ApprovalRequestDto {
  id: string;
  proposalId: string;
  proposalVersion: number;
  proposalHash: string;
  proposalHashVersion: number;
  allowedAction: string;
  status: ApprovalStatus;
  issuedAt: string;
  expiresAt: string;
}

export interface ExecutionRecordDto {
  id: string;
  proposalId: string;
  approvalRequestId: string;
  adapterId: "paper";
  status: "starting" | "completed" | "failed";
  resultSummary?: string;
  createdAt: string;
  completedAt?: string;
}

export interface AppSnapshotDto {
  signals: SignalDto[];
  connectors: ConnectorDto[];
  providers: ModelProviderDto[];
  pendingApprovals: ApprovalRequestDto[];
  recentExecutions: ExecutionRecordDto[];
}

export function isSignalStatus(value: unknown): value is SignalStatus {
  return typeof value === "string" && SIGNAL_STATUSES.includes(value as SignalStatus);
}

export function isApprovalStatus(value: unknown): value is ApprovalStatus {
  return typeof value === "string" && APPROVAL_STATUSES.includes(value as ApprovalStatus);
}

export function parseRuntimeEventEnvelope(value: unknown): RuntimeEventEnvelope {
  if (typeof value !== "object" || value === null) {
    throw new Error("Invalid runtime event envelope");
  }
  const event = value as Record<string, unknown>;
  const valid =
    typeof event.id === "string" &&
    typeof event.family === "string" &&
    EVENT_FAMILIES.includes(event.family as RuntimeEventFamily) &&
    typeof event.type === "string" &&
    typeof event.aggregateId === "string" &&
    typeof event.occurredAt === "string" &&
    typeof event.payload === "object" &&
    event.payload !== null;
  if (!valid) {
    throw new Error("Invalid runtime event envelope");
  }
  return event as unknown as RuntimeEventEnvelope;
}
