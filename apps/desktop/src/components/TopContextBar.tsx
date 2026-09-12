import { ChevronDown, Database, ShieldCheck } from "lucide-react";
import type { SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import type { PageId } from "./Navigation";

interface TopContextBarProps { page: PageId; signal?: SignalDto; t: Translator }

export function TopContextBar({ page, signal, t }: TopContextBarProps) {
  return (
    <header className="context-bar">
      <div className="breadcrumbs">
        <span>{t("top.context")}</span><strong>{signal?.asset ?? t(`nav.${page}`)}</strong>
        {signal ? <span>{signal.category}</span> : null}
      </div>
      <div className="context-actions">
        <div className="context-pill"><Database size={14} /><span>{t("top.marketData")}</span></div>
        <div className="context-pill gold"><ShieldCheck size={14} /><span>{t("top.model")}</span><ChevronDown size={13} /></div>
      </div>
    </header>
  );
}
