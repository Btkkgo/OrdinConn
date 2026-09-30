import type { Translator } from "../i18n";
import type { WorkbenchViewModel } from "./model";
import { PageHeader, StatusBadge } from "../components/WorkspaceUI";
export function WorkbenchHeader({ model, t }: { model: WorkbenchViewModel; t: Translator }) {
  return <PageHeader title={t("workbench.title")} description={t("workbench.subtitle")} status={<StatusBadge tone={model.device.connected ? "success" : "neutral"}>{t(model.device.connected ? "workbench.connected" : "workbench.disconnected")}</StatusBadge>} />;
}
export function WorkbenchLegend({ t }: { t: Translator }) {
  return <footer className="workbench-legend">{["observe", "capture", "plan", "approval"].map(key => <span key={key}><i/>{t(`workbench.legend.${key}`)}</span>)}</footer>;
}
