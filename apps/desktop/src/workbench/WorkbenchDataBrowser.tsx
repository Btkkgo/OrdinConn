import { useEffect } from "react";
import type { IntelligenceItemDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
export function WorkbenchDataBrowser({ items, onSelect, onClose, t }: { items: IntelligenceItemDto[]; onSelect: (item: IntelligenceItemDto) => void; onClose: () => void; t: Translator }) {
  useEffect(() => { const close = (event: KeyboardEvent) => { if (event.key === "Escape") onClose(); }; window.addEventListener("keydown", close); return () => window.removeEventListener("keydown", close); }, [onClose]);
  return <div className="modal-backdrop" role="presentation" onMouseDown={onClose}><section className="workbench-data-browser" role="dialog" aria-modal="true" aria-label={t("workbench.dataBrowser")} onMouseDown={event => event.stopPropagation()}><header><h2>{t("workbench.dataBrowser")}</h2><button type="button" autoFocus onClick={onClose}>{t("common.close")}</button></header><p>{t("workbench.metricScope")}</p><div>{items.length ? items.map(item => <button type="button" key={item.id} onClick={() => onSelect(item)}><strong>{item.title}</strong><span>{item.summary}</span><small>{item.sourceApp} · {item.observedAt} · {item.evidenceStatus}</small></button>) : <p>{t("mobile.noObservations")}</p>}</div></section></div>;
}
