import type { SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { MarketPage } from "./MarketPage";

export function CryptoPage(props: { signals: SignalDto[]; onOpenSignal: (signal: SignalDto) => void; t: Translator }) {
  return <MarketPage market="crypto" categories={["onchain", "event", "exchange"]} {...props} />;
}
