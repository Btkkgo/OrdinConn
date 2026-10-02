import type { MobileActionInputDto, MobileActionReceiptDto, MobileDeviceSessionDto, MobileFrameDto, MobileUiSnapshotDto } from "@ordinconn/contracts";
import { Crosshair, Smartphone } from "lucide-react";
import { useState, type ReactNode } from "react";
import type { Translator } from "../i18n";
import { MobileActionControls } from "./MobileActionControls";

interface MobileDeviceViewProps {
  compact?: boolean;
  observeBeforeAction?: boolean;
  children?: ReactNode;
  fixtureScreen?: ReactNode;
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

export function frameSurfaceStyle(frame: MobileFrameDto | undefined) {
  const outerAspect = 9 / 16;
  const imageAspect = frame ? frame.width / frame.height : outerAspect;
  return imageAspect <= outerAspect
    ? { width: `${imageAspect / outerAspect * 100}%`, height: "100%" }
    : { width: "100%", height: `${outerAspect / imageAspect * 100}%` };
}

export function MobileDeviceView({ compact = false, observeBeforeAction = false, children, fixtureScreen, frame, snapshot, session, allowedApps, latestActionReceipt, adbStatus, inspect, onInspectChange, onObserve, onStop, onAction, t }: MobileDeviceViewProps) {
  const aligned = frame && snapshot && frame.width === snapshot.screenWidth && frame.height === snapshot.screenHeight;
  const [selection, setSelection] = useState<{ snapshotId: string; elementRef: string }>();
  const selected = snapshot && selection?.snapshotId === snapshot.snapshotId ? snapshot.elements.find((element) => element.ref === selection.elementRef) : undefined;
  return (
    <div className={compact ? "device-stage workbench-device" : "device-stage"}>
      {!compact ? <>
      <div className="device-toolbar">
        <button className="secondary-button" type="button" onClick={onObserve}>{t("mobile.observeNow")}</button>
        <button className="secondary-button" type="button" onClick={onStop}>{t("mobile.stopSession")}</button>
        <button className={inspect ? "icon-button active" : "icon-button"} type="button" onClick={() => onInspectChange(!inspect)} aria-pressed={inspect} aria-label={t("mobile.inspectElements")}><Crosshair size={16} /></button>
      </div>
      </> : null}
      <div className="phone-frame" style={{ "--phone-aspect": compact ? "9 / 16" : frame ? `${frame.width} / ${frame.height}` : "9 / 20" } as React.CSSProperties}>
        <div className="device-frame-surface" style={compact ? frameSurfaceStyle(frame) : { width: "100%", height: "100%" }}>
        {fixtureScreen ?? (frame ? <img src={frame.dataUrl} alt={t("mobile.currentScreen")} /> : (
          <div className="device-empty"><Smartphone size={28} aria-hidden="true" /><strong>{adbStatus === "missing" ? t("mobile.adbUnavailable") : t("mobile.noLiveFrame")}</strong><span>{adbStatus === "missing" ? t("mobile.installAdb") : t("mobile.connectEmulator")}</span><button className="secondary-button" type="button" onClick={onObserve}>{t("mobile.checkDevice")}</button></div>
        ))}
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
      </div>
      {children}
      {compact ? <details className="workbench-inspector"><summary>{t("workbench.deviceTools")}</summary><button type="button" onClick={() => onInspectChange(!inspect)} aria-pressed={inspect}>{t("mobile.inspectElements")}</button></details> : null}
      <div className={compact ? "workbench-device-details" : "device-details"} hidden={compact && !inspect}>
      <div className="device-meta"><span>{snapshot?.packageName ?? t("mobile.noForegroundApp")}</span><span>{snapshot ? t("mobile.uiElements", { count: snapshot.elements.length }) : t("mobile.uiTreeUnavailable")}</span></div>
      {snapshot ? <div className="mobile-ui-tree" aria-label={t("collection.uiTree")}>{snapshot.elements.map(element=><button type="button" className="secondary-button" key={element.ref} aria-pressed={selected?.ref===element.ref} onClick={()=>setSelection({snapshotId:snapshot.snapshotId,elementRef:element.ref})}><span>{element.ref} · {element.role}</span><span>{element.text??element.contentDescription??"—"}</span></button>)}</div> : null}
      {inspect && selected ? <dl className="inspector-detail"><div><dt>{t("collection.elementId")}</dt><dd>{selected.ref}</dd></div><div><dt>{t("collection.text")}</dt><dd>{selected.text ?? "—"}</dd></div><div><dt>{t("collection.roleClass")}</dt><dd>{selected.role} · {selected.className}</dd></div><div><dt>{t("collection.description")}</dt><dd>{selected.contentDescription ?? "—"}</dd></div><div><dt>{t("collection.bounds")}</dt><dd>{selected.bounds.x},{selected.bounds.y} {selected.bounds.width}×{selected.bounds.height}</dd></div><div><dt>{t("collection.elementState")}</dt><dd>{t("collection.clickable")}: {t(selected.clickable ? "collection.yes":"collection.no")} · {t("collection.scrollable")}: {t(selected.scrollable ? "collection.yes":"collection.no")} · {t("collection.editable")}: {t(selected.className.endsWith("EditText") ? "collection.yes":"collection.no")} · {t("collection.enabled")}: {t(selected.enabled ? "collection.yes":"collection.no")}</dd></div><div><dt>{t("collection.resourceId")}</dt><dd>{selected.resourceId ?? "—"}</dd></div><div><dt>{t("collection.method")}</dt><dd>{selected.extractionSource} · {Math.round(selected.confidence * 100)}%</dd></div></dl> : null}
      <MobileActionControls observeBeforeAction={observeBeforeAction} session={session} snapshot={snapshot} selected={selected} allowedApps={allowedApps} receipt={latestActionReceipt} onAction={onAction} t={t} />
      </div>
    </div>
  );
}
