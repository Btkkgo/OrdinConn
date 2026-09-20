import { describe, expect, it } from "vitest";
import type { IntelligenceItemDto, SignalDto, WarehouseEntryDto } from "@ordinconn/contracts";
import {
  filterWarehouseItems,
  matchRelatedSignals,
  signalReadiness,
  sortIntelligenceItems,
  sourceLabel,
} from "./mobileIntelligence";

const items: IntelligenceItemDto[] = [
  {
    id: "older", sourceMethod: "RSS", sourceApp: "Reuters", title: "Older", summary: "Macro",
    observedAt: "2026-09-20T01:00:00Z", dataType: "evidence", evidenceStatus: "validated",
    confidence: 0.9, assets: ["NVDA"], favorite: false, saved: false, officialSource: true,
    hasContradiction: false, evidenceIds: ["e1"], relatedSignalIds: [],
  },
  {
    id: "newer", sourceMethod: "MOBILE", sourceApp: "X", title: "BTC update", summary: "ETF inflow",
    observedAt: "2026-09-20T02:00:00Z", dataType: "mobile_observation", evidenceStatus: "observation_only",
    confidence: 1, assets: ["BTC"], favorite: true, saved: true, officialSource: false,
    hasContradiction: false, mobileObservationId: "newer", evidenceIds: [], relatedSignalIds: ["signal-1"],
  },
];

const signals = [
  { id: "signal-1", asset: "BTC", title: "BTC signal" },
  { id: "signal-2", asset: "SOL", title: "SOL signal" },
] as SignalDto[];

const warehouse: WarehouseEntryDto[] = [{
  id: "warehouse-1", itemId: "newer", favorite: true, saved: true, tags: ["macro"],
  createdAt: "2026-09-20T02:00:00Z", updatedAt: "2026-09-20T02:00:00Z",
}];

describe("mobile intelligence projections", () => {
  it("sorts newest first and names source methods", () => {
    expect(sortIntelligenceItems(items).map((item) => item.id)).toEqual(["newer", "older"]);
    expect(sourceLabel("MOBILE")).toBe("Mobile");
    expect(sourceLabel("WEBSOCKET")).toBe("WebSocket");
  });

  it("matches explicit and asset-related signals with an honest empty state", () => {
    expect(matchRelatedSignals(items[1], signals).map((signal) => signal.id)).toEqual(["signal-1"]);
    expect(signalReadiness(items[0], signals)).toBe("No signal yet");
  });

  it("projects saved and favorite warehouse items through search", () => {
    expect(filterWarehouseItems(items, warehouse, "btc", "all").map((item) => item.id)).toEqual(["newer"]);
    expect(filterWarehouseItems(items, warehouse, "", "favorite").map((item) => item.id)).toEqual(["newer"]);
    expect(filterWarehouseItems(items, warehouse, "", "saved").map((item) => item.id)).toEqual(["newer"]);
  });
});
