import { useState } from "react";
import type { Translator } from "../i18n";
export type WorkbenchCommand = "observe" | "extract" | "collect" | "stop";
export function submitCommandDraft(draft: string, onSubmit: (goal: string) => void): boolean {
  if (!draft.trim()) return false; onSubmit(draft); return true;
}
export function AgentCommandPanel({ disabled, busy, onCommand, onSubmit, t }: { disabled: boolean; busy: boolean; onCommand: (command: WorkbenchCommand) => void; onSubmit: (goal: string) => void; t: Translator }) {
  const [draft, setDraft] = useState("");
  return <section className="workbench-commands" aria-label={t("workbench.commands")}><h3>{t("workbench.commands")}</h3><div className="workbench-command-grid">
    {(["observe", "extract", "collect", "stop"] as const).map(command => <button type="button" key={command} className={command === "observe" ? "primary-button" : command === "stop" ? "danger-button" : "secondary-button"} disabled={command !== "stop" && (disabled || busy)} onClick={() => onCommand(command)}>{t(`workbench.command.${command}`)}</button>)}
    </div><form className="workbench-command-input" onSubmit={event => { event.preventDefault(); if (submitCommandDraft(draft, onSubmit)) setDraft(""); }}><input type="text" value={draft} maxLength={1000} onChange={event => setDraft(event.target.value)} placeholder={t("workbench.commandPlaceholder")} aria-label={t("workbench.commandPlaceholder")}/><button className="primary-button" type="submit" disabled={busy || disabled || !draft.trim()}>{t("workbench.execute")}</button></form></section>;
}
