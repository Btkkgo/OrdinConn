import type { MobileActionInputDto, MobileActionReceiptDto, MobileDeviceSessionDto, MobileFrameDto, MobileUiSnapshotDto } from "@ordinconn/contracts";
import { Crosshair, Smartphone } from "lucide-react";
import { useState } from "react";
import type { Translator } from "../i18n";
import { MobileActionControls } from "./MobileActionControls";

interface MobileDeviceViewProps {
  frame?: MobileFrameDto;
  snapshot?: MobileUiSnapshotDto;
  session?: MobileDeviceSessionDto;
  allowedApps: string[];
  latestActionReceipt?: MobileActionReceiptDto | null;
  adbStatus: "ready" | "missing" | "offline" | "error";
  inspect: boolean;
  onInspectChange: (value: boolean) => void;
  onObserve: () => void;
  onStop: () => void;
  onAction: (input: MobileActionInputDto) => Promise<void>;
  t: Translator;
}

export function MobileDeviceView({ frame, snapshot, session, allowedApps, latestActionReceipt, adbStatus, inspect, onInspectChange, onObserve, onStop, onAction, t }: MobileDeviceViewProps) {
  const aligned = frame && snapshot && frame.width === snapshot.screenWidth && frame.height === snapshot.screenHeight;
  const [selection, setSelection] = useState<{ snapshotId: string; elementRef: string }>();
  const selected = snapshot && selection?.snapshotId === snapshot.snapshotId ? snapshot.elements.find((element) => element.ref === selection.elementRef) : undefined;
  return (
    <div className="device-stage">
      <div className="device-toolbar">
        <button className="secondary-button" type="button" onClick={onObserve}>{t("mobile.observeNow")}</button>
        <button className="secondary-button" type="button" onClick={onStop}>{t("mobile.stopSession")}</button>
        <button className={inspect ? "icon-button active" : "icon-button"} type="button" onClick={() => onInspectChange(!inspect)} aria-pressed={inspect} aria-label={t("mobile.inspectElements")}><Crosshair size={16} /></button>
      </div>
      <div className="phone-frame" style={{ "--phone-aspect": frame ? `${frame.width} / ${frame.height}` : "9 / 20" } as React.CSSProperties}>
        {frame ? <img src={frame.dataUrl} alt={t("mobile.currentScreen")} /> : (
          <div className="device-empty"><Smartphone size={36} /><strong>{adbStatus === "missing" ? t("mobile.adbUnavailable") : t("mobile.noLiveFrame")}</strong><span>{adbStatus === "missing" ? t("mobile.installAdb") : t("mobile.connectEmulator")}</span></div>
        )}
        {inspect && aligned ? <div className="element-overlay" aria-label="UI element inspector">{snapshot.elements.map((element) => (
          <button
            className="element-box"
            key={element.ref}
            title={`${element.ref} ${element.role} ${element.text ?? element.contentDescription ?? ""}`}
            onClick={() => snapshot && setSelection({ snapshotId: snapshot.snapshotId, elementRef: element.ref })}
            style={{ left: `${element.bounds.x / frame.width * 100}%`, top: `${element.bounds.y / frame.height * 100}%`, width: `${element.bounds.width / frame.width * 100}%`, height: `${element.bounds.height / frame.height * 100}%` }}
            type="button"
          ><span>{element.ref}</span></button>
        ))}</div> : null}
      </div>
      <div className="device-meta"><span>{snapshot?.packageName ?? t("mobile.noForegroundApp")}</span><span>{snapshot ? t("mobile.uiElements", { count: snapshot.elements.length }) : t("mobile.uiTreeUnavailable")}</span></div>
      {inspect && selected ? <dl className="inspector-detail"><div><dt>Ref</dt><dd>{selected.ref}</dd></div><div><dt>Text</dt><dd>{selected.text ?? "—"}</dd></div><div><dt>Role / Class</dt><dd>{selected.role} · {selected.className}</dd></div><div><dt>Content Description</dt><dd>{selected.contentDescription ?? "—"}</dd></div><div><dt>Bounds</dt><dd>{selected.bounds.x},{selected.bounds.y} {selected.bounds.width}×{selected.bounds.height}</dd></div><div><dt>State</dt><dd>{selected.clickable ? "clickable" : "not clickable"} · {selected.scrollable ? "scrollable" : "fixed"} · {selected.enabled ? "enabled" : "disabled"}</dd></div><div><dt>Resource ID</dt><dd>{selected.resourceId ?? "—"}</dd></div><div><dt>Extraction</dt><dd>{selected.extractionSource} · {Math.round(selected.confidence * 100)}%</dd></div></dl> : null}
      <MobileActionControls session={session} snapshot={snapshot} selected={selected} allowedApps={allowedApps} receipt={latestActionReceipt} onAction={onAction} t={t} />
    </div>
  );
}
