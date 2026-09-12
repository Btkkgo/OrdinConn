import { ArrowDownRight, ArrowUpRight, Eye, Minus } from "lucide-react";
import type { SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";

interface SignalCardProps { signal: SignalDto; onOpen: (signal: SignalDto) => void; t: Translator }

const directionIcon = { bullish: ArrowUpRight, bearish: ArrowDownRight, neutral: Minus, watch: Eye } as const;

export function SignalCard({ signal, onOpen, t }: SignalCardProps) {
  const Icon = directionIcon[signal.direction];
  const ageMinutes = Math.max(0, Math.floor((Date.now() - new Date(signal.createdAt).getTime()) / 60_000));
  const age = ageMinutes < 1 ? t("signal.justNow") : ageMinutes < 60 ? t("signal.minutesAgo", { count: ageMinutes }) : t("signal.hoursAgo", { count: Math.floor(ageMinutes / 60) });
  return (
    <button className="signal-card" onClick={() => onOpen(signal)} type="button">
      <div className="signal-card-top">
        <span className="asset-symbol">{signal.asset}</span>
        <span className={`direction ${signal.direction}`}><Icon size={13} />{t(`signal.direction.${signal.direction}`)}</span>
      </div>
      <div className="signal-type">{t(`signal.category.${signal.category}`)}</div>
      <h3>{signal.title}</h3>
      <p>{signal.summary}</p>
      <div className="signal-metrics">
        <span><small>{t("signal.confidence")}</small>{Math.round(signal.confidence * 100)}%</span>
        <span><small>{t("signal.urgency")}</small>{Math.round(signal.urgency * 100)}%</span>
        <span><small>{t("signal.evidence")}</small>{signal.evidence.length}</span>
        <span><small>{t("signal.age")}</small>{age}</span>
      </div>
      <div className="signal-agent"><span>{t("signal.agent")}</span><strong>{signal.agentId}</strong></div>
      <div className="quality-line"><i style={{ width: `${signal.evidenceQuality * 100}%` }} /></div>
    </button>
  );
}
