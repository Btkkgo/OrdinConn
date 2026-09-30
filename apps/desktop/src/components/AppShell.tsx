import type { ReactNode } from "react";
import type { Translator } from "../i18n";
import { Navigation, type PageId } from "./Navigation";

interface AppShellProps {
  page: PageId;
  onNavigate: (page: PageId) => void;
  workspace: ReactNode;
  t: Translator;
}

export function AppShell({ page, onNavigate, workspace, t }: AppShellProps) {
  return (
    <div className="app-shell mobile-shell">
      <Navigation page={page} onNavigate={onNavigate} t={t} />
      <main className="workspace">{workspace}</main>
    </div>
  );
}
