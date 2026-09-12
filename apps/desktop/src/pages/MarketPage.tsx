import type { Market, SignalCategory, SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { SignalCard } from "../components/SignalCard";

interface MarketPageProps {
  market: Market;
  categories: SignalCategory[];
  signals: SignalDto[];
  onOpenSignal: (signal: SignalDto) => void;
  t: Translator;
}

export function MarketPage({ market, categories, signals, onOpenSignal, t }: MarketPageProps) {
  const titleKey = market === "traditional" ? "market.traditionalTitle" : "market.cryptoTitle";
  const descriptionKey = market === "traditional" ? "market.traditionalDescription" : "market.cryptoDescription";
  return (
    <section className="page">
      <header className="page-header"><span className="eyebrow">{t(`nav.${market}`)}</span><h1>{t(titleKey)}</h1><p>{t(descriptionKey)}</p></header>
      <div className="market-notice"><span>{t("market.evidenceGate")}</span><p>{t("market.mockNotice")}</p></div>
      {categories.map((category) => {
        const lane = signals.filter((signal) => signal.market === market && signal.category === category);
        return (
          <section className="signal-lane" key={category}>
            <div className="lane-header"><h2>{t(`market.${category}`)}</h2><span>{lane.length} {t("market.published")}</span></div>
            <div className="signal-grid">{lane.map((signal) => <SignalCard key={signal.id} signal={signal} onOpen={onOpenSignal} t={t} />)}</div>
          </section>
        );
      })}
    </section>
  );
}
