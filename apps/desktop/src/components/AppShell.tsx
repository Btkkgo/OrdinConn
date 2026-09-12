import type { ReactNode } from "react";
import type { SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { Navigation, type PageId } from "./Navigation";
import { TopContextBar } from "./TopContextBar";

interface AppShellProps {
  page: PageId;
  signal?: SignalDto;
  onNavigate: (page: PageId) => void;
  workspace: ReactNode;
  dock: ReactNode;
  dockCollapsed: boolean;
  t: Translator;
}

export function AppShell({ page, signal, onNavigate, workspace, dock, dockCollapsed, t }: AppShellProps) {
  return (
    <div className={dockCollapsed ? "app-shell dock-collapsed" : "app-shell"}>
      <Navigation page={page} onNavigate={onNavigate} t={t} />
      <TopContextBar page={page} signal={signal} t={t} />
      <main className="workspace">{workspace}</main>
      <aside className="agent-region">{dock}</aside>
    </div>
  );
}
