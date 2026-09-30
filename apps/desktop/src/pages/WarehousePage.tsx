import { useMemo, useState } from "react";
import type { IntelligenceItemDto, MobileWorkspaceDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { EmptyState, PageHeader } from "../components/WorkspaceUI";
import { filterWarehouseItems, sourceLabel, type WarehouseFilter } from "./mobileIntelligence";

interface WarehousePageProps { workspace: MobileWorkspaceDto; onOpenDetail: (item: IntelligenceItemDto) => void; onGoHome: () => void; t: Translator; }

export function WarehousePage({ workspace, onOpenDetail, onGoHome, t }: WarehousePageProps) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<WarehouseFilter>("all");
  const items = useMemo(() => filterWarehouseItems(workspace.feed, workspace.warehouse, query, filter), [workspace.feed, workspace.warehouse, query, filter]);
  const filtered = Boolean(query.trim()) || filter !== "all";
  return <section className="page warehouse-page">
    <PageHeader title={t("warehouse.title")} description={t("warehouse.description")} />
    <div className="warehouse-toolbar panel">
      <input type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("warehouse.search")} aria-label={t("warehouse.search")} />
      <div className="segmented">{(["all", "favorite", "saved"] as const).map((value) => <button key={value} type="button" aria-pressed={filter === value} className={filter === value ? "active" : ""} onClick={() => setFilter(value)}>{t(`warehouse.filter.${value}`)}</button>)}</div>
    </div>
    {items.length ? <div className="warehouse-grid">{items.map((item) => {
      const entry = workspace.warehouse.find(value => value.itemId === item.id);
      return <button className="card warehouse-card" type="button" key={item.id} onClick={() => onOpenDetail(item)}>
        <header><span className="source-chip">{sourceLabel(item.sourceMethod)}</span><span>{item.dataType}</span></header>
        <strong>{item.title}</strong><p>{item.summary}</p>
        <div className="warehouse-metadata"><span>{t("warehouse.source")}: {item.sourceApp}</span><span>{t("warehouse.savedAt")}: {entry?.createdAt ?? item.observedAt}</span><span>Evidence: {item.evidenceIds.length} · {item.evidenceStatus}</span></div>
        <footer><span>{item.assets.join(" · ")}</span><span>{entry?.tags.join(" · ")}</span></footer>
      </button>;
    })}</div> : <div className="panel warehouse-empty"><EmptyState title={t(filtered ? "warehouse.noMatches" : "warehouse.empty")} description={t(filtered ? "warehouse.noMatchesDescription" : "warehouse.emptyDescription")} actions={<><button className="primary-button" type="button" onClick={onGoHome}>{t("warehouse.goHome")}</button>{filtered ? <button className="secondary-button" type="button" onClick={() => { setQuery(""); setFilter("all"); }}>{t("warehouse.clearFilters")}</button> : null}</>} /></div>}
  </section>;
}
