import { useMemo, useState } from "react";
import type { IntelligenceItemDto, MobileWorkspaceDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { filterWarehouseItems, sourceLabel, type WarehouseFilter } from "./mobileIntelligence";

interface WarehousePageProps { workspace: MobileWorkspaceDto; onOpenDetail: (item: IntelligenceItemDto) => void; t: Translator; }

export function WarehousePage({ workspace, onOpenDetail, t }: WarehousePageProps) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<WarehouseFilter>("all");
  const items = useMemo(() => filterWarehouseItems(workspace.feed, workspace.warehouse, query, filter), [workspace.feed, workspace.warehouse, query, filter]);
  return <section className="page warehouse-page"><header className="page-header"><span className="eyebrow">{t("warehouse.eyebrow")}</span><h1>{t("warehouse.title")}</h1><p>{t("warehouse.description")}</p></header><div className="warehouse-toolbar"><input type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t("warehouse.search")} aria-label={t("warehouse.search")}/><div className="segmented">{(["all", "favorite", "saved"] as const).map((value) => <button key={value} type="button" className={filter === value ? "active" : ""} onClick={() => setFilter(value)}>{value}</button>)}</div></div><div className="warehouse-grid">{items.length ? items.map((item) => <button className="panel warehouse-card" type="button" key={item.id} onClick={() => onOpenDetail(item)}><span className="source-chip">{sourceLabel(item.sourceMethod)}</span><strong>{item.title}</strong><p>{item.summary}</p><footer><span>{item.assets.join(" · ")}</span><span>{item.evidenceStatus}</span></footer></button>) : <p className="empty-copy">{t("warehouse.empty")}</p>}</div></section>;
}
