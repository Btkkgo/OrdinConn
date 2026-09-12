import type { SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { MarketPage } from "./MarketPage";

export function TraditionalFinancePage(props: { signals: SignalDto[]; onOpenSignal: (signal: SignalDto) => void; t: Translator }) {
  return <MarketPage market="traditional" categories={["trading", "event", "demand"]} {...props} />;
}
