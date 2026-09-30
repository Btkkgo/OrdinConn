import { MetricValue, metricColorMap } from "../components/WorkspaceUI";
import type { Translator } from "../i18n";
import { categoryIds, type RealtimeMetric, type WorkbenchViewModel } from "./model";
export function metricLabel(id: string, t: Translator): string { return categoryIds.includes(id as typeof categoryIds[number]) ? t(`workbench.category.${id}`) : id; }
function deltaText(metric: RealtimeMetric, t: Translator): string {
  return metric.deltaPercent === undefined ? t("workbench.noBaseline") : `${metric.direction === "up" ? "↑" : metric.direction === "down" ? "↓" : "—"} ${Math.abs(metric.deltaPercent).toFixed(1)}% · ${t("workbench.todayAdded")}`;
}
export function RealtimeDataPanel({ model, onSelect, onOpenData, t }: { model: WorkbenchViewModel; onSelect: (id: string) => void; onOpenData: () => void; t: Translator }) {
  const selected = model.metrics.find(metric => metric.id === model.selectedMetric)!;
  return <section className="workbench-panel realtime-panel" aria-label={t("workbench.data")} title={t("workbench.metricScope")}>
    <header className="workbench-panel-header"><div><h2>{t("workbench.data")}</h2><p>{t("workbench.collectedToday")}</p></div><MetricValue value={model.total.count} tone={metricColorMap.total} /></header>
    <article className="metric-comparison"><span>{t("workbench.vsYesterday")}</span><div><strong className={model.total.deltaPercent === undefined || model.total.deltaPercent === 0 ? "tone-neutral" : model.total.deltaPercent > 0 ? "tone-success" : "tone-warning"}>{model.total.deltaPercent === undefined ? "—" : `${model.total.deltaPercent >= 0 ? "+" : ""}${model.total.deltaPercent.toFixed(1)}%`}</strong><span>{model.total.deltaPercent === undefined ? t("workbench.noBaseline") : t("workbench.deltaCount", { count: `${model.total.deltaCount >= 0 ? "+" : ""}${model.total.deltaCount}` })}</span></div></article>
    <div className="metric-list">{model.metrics.map(metric => <button type="button" key={metric.id} className={model.selectedMetric === metric.id ? "metric-card selected" : "metric-card"} aria-pressed={model.selectedMetric === metric.id} onClick={() => onSelect(metric.id)}><span><strong>{metricLabel(metric.id, t)}</strong><MetricValue value={metric.count} tone={metricColorMap[metric.id as keyof typeof metricColorMap] ?? "neutral"} /></span><small className={metric.deltaPercent === undefined ? "" : metric.direction === "up" ? "tone-success" : metric.direction === "down" ? "tone-warning" : ""}>{deltaText(metric, t)}</small></button>)}</div>
    <button type="button" className="workbench-current-view" onClick={onOpenData}>{t("workbench.currentView", { label: metricLabel(selected.id, t), count: selected.count })}</button>
  </section>;
}
