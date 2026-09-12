import type { SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { SignalCard } from "../components/SignalCard";
import { filterSignals, type SignalFilter } from "./overviewDashboard";

interface SignalsPageProps {
  signals: SignalDto[];
  filter: SignalFilter;
  onClearFilter: () => void;
  onOpenSignal: (signal: SignalDto) => void;
  t: Translator;
}

function filterLabel(filter: SignalFilter, t: Translator): string {
  if (filter.kind === "category") return t(`signal.category.${filter.category}`);
  if (filter.kind === "highConfidence") return t("signal.filter.highConfidence");
  if (filter.kind === "watch") return t("signal.filter.watch");
  if (filter.kind === "contradicting") return t("signal.filter.contradicting");
  return t("common.all");
}

export function SignalsPage({ signals, filter, onClearFilter, onOpenSignal, t }: SignalsPageProps) {
  const visibleSignals = filterSignals(signals, filter);
  return (
    <section className="page">
      <header className="page-header"><span className="eyebrow">{t("nav.signals")}</span><h1>{t("signal.allTitle")}</h1><p>{t("signal.allDescription")}</p></header>
      {filter.kind !== "all" ? <div className="signal-filter"><span>{t("signal.filter")}: <strong>{filterLabel(filter, t)}</strong></span><button onClick={onClearFilter} type="button">{t("signal.clearFilter")}</button></div> : null}
      {visibleSignals.length > 0 ? <div className="signal-grid all-signals">{visibleSignals.map((signal) => <SignalCard key={signal.id} signal={signal} onOpen={onOpenSignal} t={t} />)}</div> : <div className="empty-panel">{t("signal.noResults")}</div>}
    </section>
  );
}
