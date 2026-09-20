import { useMemo, useState } from "react";
import type { IntelligenceItemDto, MobileWorkspaceDto, SignalDto } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
import { MobileDeviceView } from "../components/MobileDeviceView";
import { matchRelatedSignals, signalReadiness, sortIntelligenceItems, sourceLabel } from "./mobileIntelligence";

interface MobileHomePageProps {
  workspace: MobileWorkspaceDto;
  signals: SignalDto[];
  selectedItemId?: string;
  onSelectItem: (id: string) => void;
  onObserve: () => void;
  onOpenDetail: (item: IntelligenceItemDto) => void;
  t: Translator;
}

export function MobileHomePage({ workspace, signals, selectedItemId, onSelectItem, onObserve, onOpenDetail, t }: MobileHomePageProps) {
  const [inspect, setInspect] = useState(false);
  const feed = useMemo(() => sortIntelligenceItems(workspace.feed), [workspace.feed]);
  const selected = feed.find((item) => item.id === selectedItemId) ?? feed[0];
  const related = selected ? matchRelatedSignals(selected, signals) : [];
  return (
    <section className="mobile-home">
      <header className="runtime-strip"><div><span className={`status-dot ${workspace.runtimeStatus}`} /><strong>{workspace.runtimeStatus}</strong></div><span>ADB {workspace.adbStatus}</span><span>{workspace.session?.deviceId ?? "No device"}</span><span>{workspace.uiSnapshot?.packageName ?? "No active app"}</span><span>Task: none</span><span>Agent: observe-only</span><span>Verify: {workspace.observations[0]?.verificationStatus ?? "waiting"}</span></header>
      <div className="intelligence-columns">
        <section className="intelligence-column feed-column" aria-label={t("mobile.feed")}><header><div><span className="eyebrow">{t("mobile.observe")}</span><h1>{t("mobile.feed")}</h1></div><span>{feed.length}</span></header><div className="feed-list">{feed.length ? feed.map((item) => <button className={selected?.id === item.id ? "feed-item selected" : "feed-item"} type="button" key={item.id} onClick={() => { onSelectItem(item.id); onOpenDetail(item); }}><span className="source-chip">{sourceLabel(item.sourceMethod)}</span><strong>{item.title}</strong><p>{item.summary}</p><footer><span>{item.assets.join(" · ") || "General"}</span><span>{item.evidenceStatus}</span></footer></button>) : <p className="empty-copy">{t("mobile.noObservations")}</p>}</div></section>
        <section className="intelligence-column device-column" aria-label={t("mobile.liveView")}><header><div><span className="eyebrow">{t("mobile.runtime")}</span><h2>{t("mobile.liveView")}</h2></div></header><MobileDeviceView frame={workspace.frame} snapshot={workspace.uiSnapshot} adbStatus={workspace.adbStatus} inspect={inspect} onInspectChange={setInspect} onObserve={onObserve} t={t} /></section>
        <section className="intelligence-column signal-column" aria-label={t("mobile.relatedSignals")}><header><div><span className="eyebrow">{t("mobile.interpret")}</span><h2>{t("mobile.relatedSignals")}</h2></div></header>{selected ? <><article className="selected-observation"><span className="state-chip">{signalReadiness(selected, signals)}</span><h3>{selected.title}</h3><p>{selected.summary}</p><button className="secondary-button" type="button" onClick={() => onOpenDetail(selected)}>{t("mobile.openDetail")}</button></article><div className="related-signals">{related.length ? related.map((signal) => <article className="signal-preview" key={signal.id}><span>{signal.asset}</span><strong>{signal.title}</strong><p>{signal.summary}</p></article>) : <p className="empty-copy">{t("mobile.noSignal")}</p>}</div></> : <p className="empty-copy">{t("mobile.selectObservation")}</p>}</section>
      </div>
    </section>
  );
}
