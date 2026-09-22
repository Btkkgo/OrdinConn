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
  "mobile",
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

export const MOBILE_RUNTIME_STATUSES = [
  "unavailable",
  "disconnected",
  "observing",
  "paused",
  "error",
] as const;
export type MobileRuntimeStatus = (typeof MOBILE_RUNTIME_STATUSES)[number];

export const MOBILE_VERIFICATION_RESULTS = [
  "VERIFIED",
  "NO_CHANGE",
  "UNEXPECTED_STATE",
  "TARGET_MISSING",
  "PERMISSION_REQUIRED",
  "LOGIN_REQUIRED",
  "CAPTCHA_BLOCKED",
  "SENSITIVE_FIELD_BLOCKED",
  "FINANCIAL_ACTION_BLOCKED",
  "STALE_OBSERVATION",
  "APP_CRASHED",
  "DEVICE_OFFLINE",
  "INTERRUPTED",
] as const;
export type MobileVerificationResult = (typeof MOBILE_VERIFICATION_RESULTS)[number];

export interface MobileBoundsDto {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface MobileDeviceSessionDto {
  sessionId: string;
  deviceId: string;
  platform: "android" | "ios";
  deviceType: "emulator" | "physical";
  osVersion: string;
  screenWidth: number;
  screenHeight: number;
  connectedAt: string;
  currentApp?: string;
  currentActivity?: string;
  status: "connected" | "offline" | "blocked" | "ended";
  lastObservationAt?: string;
}

export interface MobileElementDto {
  ref: string;
  text?: string;
  role: string;
  className: string;
  contentDescription?: string;
  bounds: MobileBoundsDto;
  clickable: boolean;
  scrollable: boolean;
  enabled: boolean;
  focused: boolean;
  selected: boolean;
  resourceId?: string;
  extractionSource: "accessibility" | "ocr" | "vision" | "hybrid";
  confidence: number;
}

export interface MobileUiSnapshotDto {
  snapshotId: string;
  sessionId: string;
  packageName: string;
  activity: string;
  screenWidth: number;
  screenHeight: number;
  capturedAt: string;
  elements: MobileElementDto[];
  redactions?: string[];
  sensitiveState?: MobileVerificationResult | null;
}

export type MobileSwipeDirectionDto = "up" | "down" | "left" | "right";
export type MobileActionTargetDto =
  | { kind: "tap"; elementRef: string }
  | { kind: "swipe"; direction: MobileSwipeDirectionDto }
  | { kind: "type"; elementRef: string }
  | { kind: "back" }
  | { kind: "home" }
  | { kind: "open_app"; packageName: string };

export interface MobileActionInputDto {
  sessionId: string;
  snapshotId: string;
  expectedPackage: string;
  target: MobileActionTargetDto;
  text?: string;
}

export type MobileActionDenyReasonDto =
  | "INACTIVE_SESSION" | "PHYSICAL_DEVICE" | "BUDGET_EXCEEDED" | "PACKAGE_NOT_ALLOWED"
  | "FOREGROUND_CHANGED" | "STALE_SNAPSHOT" | "SENSITIVE_SCREEN" | "TARGET_MISSING"
  | "SENSITIVE_TARGET" | "TARGET_DISABLED" | "TARGET_NOT_CLICKABLE" | "TARGET_NOT_EDITABLE"
  | "TARGET_NOT_FOCUSED" | "INVALID_BOUNDS" | "UNSAFE_TEXT" | "INVALID_ACTION";

export type MobileActionDecisionDto =
  | { outcome: "allowed" }
  | { outcome: "denied"; reason: MobileActionDenyReasonDto };

export interface MobileActionReceiptDto {
  actionId: string;
  sessionId: string;
  snapshotId: string;
  target: MobileActionTargetDto;
  decision: MobileActionDecisionDto;
  status: "blocked" | "executed" | "failed";
  requestedAt: string;
  completedAt: string;
  prePackage: string;
  preActivity: string;
  preFrameHash: string;
  preUiTreeHash: string;
  postPackage?: string | null;
  postSnapshotId?: string | null;
  postActivity?: string | null;
  postFrameHash?: string | null;
  postUiTreeHash?: string | null;
  verification?: MobileVerificationResult | null;
  textLength?: number | null;
  textSha256?: string | null;
  commandSent: boolean;
}

export interface MobileActionResultDto {
  receipt: MobileActionReceiptDto;
  workspace: MobileWorkspaceDto;
}

export interface MobileFrameDto {
  frameId: string;
  sessionId: string;
  timestamp: string;
  width: number;
  height: number;
  orientation: "portrait" | "landscape" | "unknown";
  frameHash: string;
  dataUrl: string;
}

export interface MobileObservationDto {
  id: string;
  taskId?: string;
  deviceSessionId: string;
  appId: string;
  packageName: string;
  activity: string;
  screenState: string;
  observedAt: string;
  frameHash: string;
  uiTreeHash: string;
  sourceLocator: string;
  visibleFacts: string[];
  extractedEntities: string[];
  author?: string;
  publishedAt?: string;
  extractionMethod: "accessibility" | "ocr" | "vision" | "hybrid";
  extractionConfidence: number;
  redactions: string[];
  privacyClass: "public" | "user_allowed" | "sensitive";
  verificationStatus: MobileVerificationResult;
  evidenceStatus: "observation_only" | "validated" | "rejected";
  metadata: Record<string, unknown>;
}

export interface IntelligenceItemDto {
  id: string;
  sourceMethod: "MOBILE" | "API" | "WEBSOCKET" | "RSS" | "HTML" | "WEB" | "DESKTOP";
  sourceApp: string;
  sourceAccount?: string;
  title: string;
  summary: string;
  observedAt: string;
  dataType: string;
  evidenceStatus: "observation_only" | "validated" | "published" | "rejected";
  evidenceQuality?: number;
  confidence: number;
  assets: string[];
  favorite: boolean;
  saved: boolean;
  officialSource: boolean;
  hasContradiction: boolean;
  mobileObservationId?: string;
  evidenceIds: string[];
  relatedSignalIds: string[];
  sourceLocator?: string;
}

export interface WarehouseEntryDto {
  id: string;
  itemId: string;
  favorite: boolean;
  saved: boolean;
  tags: string[];
  note?: string;
  collection?: string;
  createdAt: string;
  updatedAt: string;
}

export interface StrategyDefinitionDto {
  id: string;
  version: string;
  market: string;
  category: string;
  enabled: boolean;
  requiredInputs: string[];
  readiness: string;
  parameters: Record<string, unknown>;
}

export interface MobileResearchBudgetDto {
  maxDurationSeconds: number;
  maxSteps: number;
  maxScrolls: number;
  maxPages: number;
  maxObservations: number;
  maxModelCalls: number;
}

export interface MobileRuntimeSettingsDto {
  androidSdk?: string;
  allowedApps: string[];
  screenshotRetention: "memory_only";
  researchBudget: MobileResearchBudgetDto;
  textScale: 90 | 100 | 110 | 120;
}

export interface AndroidDeviceInfoDto {
  id: string;
  status: string;
  model?: string;
  avdName?: string;
}

export interface AndroidAvdInfoDto {
  name: string;
  status: string;
  deviceProfile?: string;
  architecture?: string;
  running: boolean;
}

export interface AndroidEnvironmentDiagnosticsDto {
  sdkStatus: "detected" | "missing";
  adbStatus: "ready" | "missing" | "error";
  emulatorStatus: "ready" | "missing" | "error";
  sdkRoot?: string;
  adbPath?: string;
  emulatorPath?: string;
  sdkmanagerPath?: string;
  avdmanagerPath?: string;
  adbVersion?: string;
  availableAvds: AndroidAvdInfoDto[];
  onlineDevices: AndroidDeviceInfoDto[];
}

export interface MobileWorkspaceDto {
  runtimeStatus: MobileRuntimeStatus;
  adbStatus: "ready" | "missing" | "offline" | "error";
  androidEnvironment: AndroidEnvironmentDiagnosticsDto;
  session?: MobileDeviceSessionDto;
  uiSnapshot?: MobileUiSnapshotDto;
  frame?: MobileFrameDto;
  latestActionReceipt?: MobileActionReceiptDto | null;
  observations: MobileObservationDto[];
  feed: IntelligenceItemDto[];
  warehouse: WarehouseEntryDto[];
  strategies: StrategyDefinitionDto[];
  settings: MobileRuntimeSettingsDto;
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
