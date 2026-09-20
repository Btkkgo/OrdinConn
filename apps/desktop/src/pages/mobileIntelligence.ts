import type { IntelligenceItemDto, SignalDto, WarehouseEntryDto } from "@ordinconn/contracts";

export type WarehouseFilter = "all" | "favorite" | "saved";

export function sortIntelligenceItems(items: IntelligenceItemDto[]): IntelligenceItemDto[] {
  return [...items].sort((left, right) => right.observedAt.localeCompare(left.observedAt));
}

export function sourceLabel(method: IntelligenceItemDto["sourceMethod"]): string {
  return ({ MOBILE: "Mobile", API: "API", WEBSOCKET: "WebSocket", RSS: "RSS", HTML: "HTML", WEB: "Web", DESKTOP: "Desktop" })[method];
}

export function matchRelatedSignals(item: IntelligenceItemDto, signals: SignalDto[]): SignalDto[] {
  return signals.filter((signal) =>
    item.relatedSignalIds.includes(signal.id) || item.assets.some((asset) => asset.toUpperCase() === signal.asset.toUpperCase()),
  );
}

export function signalReadiness(item: IntelligenceItemDto, signals: SignalDto[]): string {
  return matchRelatedSignals(item, signals).length > 0 ? "Signal linked" : "No signal yet";
}

export function filterWarehouseItems(
  items: IntelligenceItemDto[],
  entries: WarehouseEntryDto[],
  query: string,
  filter: WarehouseFilter,
): IntelligenceItemDto[] {
  const normalized = query.trim().toLocaleLowerCase();
  const byId = new Map(entries.map((entry) => [entry.itemId, entry]));
  return sortIntelligenceItems(items).filter((item) => {
    const entry = byId.get(item.id);
    if (!entry || (!entry.favorite && !entry.saved)) return false;
    if (filter === "favorite" && !entry.favorite) return false;
    if (filter === "saved" && !entry.saved) return false;
    if (!normalized) return true;
    return [item.title, item.summary, item.sourceApp, ...item.assets, ...entry.tags]
      .join(" ")
      .toLocaleLowerCase()
      .includes(normalized);
  });
}
