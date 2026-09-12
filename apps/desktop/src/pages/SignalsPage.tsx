import type { SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { SignalCard } from "../components/SignalCard";

export function SignalsPage({ signals, onOpenSignal, t }: { signals: SignalDto[]; onOpenSignal: (signal: SignalDto) => void; t: Translator }) {
  return (
    <section className="page">
      <header className="page-header"><span className="eyebrow">{t("nav.signals")}</span><h1>{t("signal.allTitle")}</h1><p>{t("signal.allDescription")}</p></header>
      {signals.length > 0 ? <div className="signal-grid all-signals">{signals.map((signal) => <SignalCard key={signal.id} signal={signal} onOpen={onOpenSignal} t={t} />)}</div> : <div className="empty-panel">{t("signal.noResults")}</div>}
    </section>
  );
}
