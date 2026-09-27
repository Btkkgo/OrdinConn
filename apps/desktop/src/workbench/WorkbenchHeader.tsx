import type { Translator } from "../i18n";
import type { WorkbenchViewModel } from "./model";
export function WorkbenchHeader({ model, t }: { model: WorkbenchViewModel; t: Translator }) {
  return <header className="workbench-header"><div><h1>{t("workbench.title")}</h1><p>{t("workbench.subtitle")}</p></div>
    <div className="workbench-connection"><span className={model.device.connected ? "connection-dot connected" : "connection-dot"}/>{t(model.device.connected ? "workbench.connected" : "workbench.disconnected")}</div>
  </header>;
}
export function WorkbenchLegend({ t }: { t: Translator }) {
  return <footer className="workbench-legend">{["observe", "capture", "plan", "approval"].map(key => <span key={key}><i/>{t(`workbench.legend.${key}`)}</span>)}</footer>;
}
