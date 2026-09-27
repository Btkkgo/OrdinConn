import type { MobileActionInputDto, MobileActionResultDto, MobileGoalDto, MobileWorkspaceDto } from "@ordinconn/contracts";
import { mobileActionAvailability } from "../components/MobileActionControls";
import { toDataObjects } from "./model";
import type { WorkbenchCommand } from "./AgentCommandPanel";

export interface WorkbenchRuntimePort {
  observe: () => Promise<MobileWorkspaceDto>;
  action: (input: MobileActionInputDto) => Promise<MobileActionResultDto>;
  stop: () => Promise<void>;
  createGoal: (objective: string) => Promise<MobileGoalDto>;
}
export class WorkbenchCommandRunner {
  private generation = 0;
  constructor(private readonly runtime: WorkbenchRuntimePort) {}
  async stop(): Promise<void> { this.generation++; await this.runtime.stop(); }
  async execute(command: Exclude<WorkbenchCommand, "stop">): Promise<{ workspace: MobileWorkspaceDto; summaryKey: string }> {
    const generation = this.generation;
    const before = await this.runtime.observe();
    if (generation !== this.generation) throw new Error("INTERRUPTED");
    if (command === "observe") return { workspace: before, summaryKey: "workbench.extracted" };
    if (command === "extract") {
      if (!before.uiSnapshot || !before.observations.length) throw new Error("NOT_IMPLEMENTED");
      return { workspace: before, summaryKey: "workbench.extracted" };
    }
    if (!mobileActionAvailability(before.session, before.uiSnapshot, before.settings.allowedApps).navigate || !before.session || !before.uiSnapshot) throw new Error("MOBILE_ACTION_BLOCKED");
    const result = await this.runtime.action({ sessionId: before.session.sessionId, snapshotId: before.uiSnapshot.snapshotId, expectedPackage: before.uiSnapshot.packageName,
      // Finger moves upward so content continues downward. Runtime performs post-observation and verification.
      target: { kind: "swipe", direction: "up" } });
    if (generation !== this.generation) throw new Error("INTERRUPTED");
    if (result.receipt.status !== "executed" || result.receipt.verification !== "VERIFIED") throw new Error(`MOBILE_ACTION_${result.receipt.verification ?? result.receipt.status}`);
    return { workspace: result.workspace, summaryKey: "workbench.collected" };
  }
  async submitGoal(objective: string): Promise<MobileGoalDto> {
    if (!objective.trim()) throw new Error("EMPTY_GOAL");
    return this.runtime.createGoal(objective);
  }
}
export function capturedObjectIds(workspace: MobileWorkspaceDto): string[] {
  const observationId = workspace.observations[0]?.id;
  return toDataObjects(workspace.feed, workspace.observations).filter(object => object.observationId === observationId).map(object => object.id);
}
