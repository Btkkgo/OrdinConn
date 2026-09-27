import type { MobileGoalDto } from "@ordinconn/contracts";
import { useRef, useState } from "react";
import { capturedObjectIds, WorkbenchCommandRunner, type WorkbenchRuntimePort } from "./commands";
import type { AgentPlan } from "./model";
import type { WorkbenchCommand } from "./AgentCommandPanel";

export function workbenchRuntimeError(error: unknown): string {
  if (error && typeof error === "object" && "code" in error) {
    if (error.code === "MODEL_NOT_CONFIGURED") return "workbench.modelNotConfigured";
    if (error.code === "MODEL_NOT_SELECTED") return "workbench.modelNotSelected";
    if (error.code === "USER_STOPPED") return "workbench.stopped";
  }
  if (error instanceof Error) return error.message;
  // Typed IPC errors are already redacted by the Rust boundary, as in App's existing error banner.
  if (error && typeof error === "object" && "message" in error && typeof error.message === "string") return error.message;
  return "workbench.taskFailed";
}

export function useWorkbenchCommands(port: WorkbenchRuntimePort | undefined) {
  const runtimeRef = useRef(port); runtimeRef.current = port;
  const [runner] = useState(() => new WorkbenchCommandRunner({
    observe: () => { if (!runtimeRef.current) return Promise.reject(new Error("NOT_IMPLEMENTED")); return runtimeRef.current.observe(); },
    action: input => { if (!runtimeRef.current) return Promise.reject(new Error("NOT_IMPLEMENTED")); return runtimeRef.current.action(input); },
    stop: () => runtimeRef.current?.stop() ?? Promise.resolve(),
    createGoal: objective => { if (!runtimeRef.current) return Promise.reject(new Error("NOT_IMPLEMENTED")); return runtimeRef.current.createGoal(objective); },
  }));
  const lock = useRef(false);
  const [busy, setBusy] = useState(false);
  const [phase, setPhase] = useState<"observing" | "executing" | "paused">();
  const [goals, setGoals] = useState<MobileGoalDto[]>([]);
  const [plans, setPlans] = useState<AgentPlan[]>([]);
  const [notice, setNotice] = useState("");
  const add = (plan: AgentPlan) => setPlans(current => [plan, ...current.filter(item => item.id !== plan.id)].slice(0, 50));
  const command = async (kind: WorkbenchCommand) => {
    if (kind === "stop") {
      try { await runner.stop(); setPhase("paused"); setNotice("workbench.stopped"); } catch (error) { setPhase(undefined); setNotice(workbenchRuntimeError(error)); }
      return;
    }
    if (lock.current) return;
    lock.current = true; setBusy(true); setPhase(kind === "collect" ? "executing" : "observing"); setNotice("");
    try {
      const result = await runner.execute(kind);
      if (kind !== "observe") add({ id: crypto.randomUUID(), title: kind === "collect" ? "workbench.command.collect" : "workbench.command.extract", summary: "", summaryKey: result.summaryKey, priority: "low", sourceObjectIds: capturedObjectIds(result.workspace), status: "completed", createdAt: new Date().toISOString() });
      setNotice(result.summaryKey);
    } catch (error) { setNotice(workbenchRuntimeError(error)); }
    finally { lock.current = false; setBusy(false); setPhase(current => current === "paused" ? "paused" : undefined); }
  };
  const submit = async (goal: string) => {
    if (lock.current) return;
    lock.current = true; setBusy(true); setNotice("");
    try { const saved = await runner.submitGoal(goal); setGoals(current => [saved, ...current.filter(item => item.id !== saved.id)]); } catch (error) { setNotice(workbenchRuntimeError(error)); }
    finally { lock.current = false; setBusy(false); }
  };
  return { busy, phase, plans, goals, notice, command, submit };
}
