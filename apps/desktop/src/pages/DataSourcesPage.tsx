import { Database, ShieldCheck } from "lucide-react";
import type { ConnectorDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";

export function DataSourcesPage({ connectors, t }: { connectors: ConnectorDto[]; t: Translator }) {
  return (
    <section className="page">
      <header className="page-header"><span className="eyebrow">{t("nav.dataSources")}</span><h1>{t("sources.title")}</h1><p>{t("sources.description")}</p></header>
      <div className="source-grid">{connectors.map((connector) => <article className="panel source-card" key={connector.id}><div className="source-icon"><Database size={19} /></div><div className="source-title"><div><strong>{connector.name}</strong><span>{t("sources.mock")}</span></div><span className={`state-chip ${connector.status}`}>{t(`common.${connector.status}`)}</span></div><dl><div><dt>{t("common.market")}</dt><dd>{connector.market}</dd></div><div><dt>{t("sources.reliability")}</dt><dd>{Math.round(connector.reliability * 100)}%</dd></div><div><dt>{t("sources.lastUpdate")}</dt><dd>{new Date(connector.lastUpdate).toLocaleString()}</dd></div></dl><div className="capability-tags">{connector.capabilities.map((capability) => <span key={capability}>{capability}</span>)}</div><p className="source-provenance"><ShieldCheck size={13} />{t("sources.provenance")}</p></article>)}</div>
    </section>
  );
}
