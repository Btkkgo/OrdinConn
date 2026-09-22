import type {
  MobileActionInputDto,
  MobileActionReceiptDto,
  MobileActionTargetDto,
  MobileDeviceSessionDto,
  MobileElementDto,
  MobileUiSnapshotDto,
  MobileSwipeDirectionDto,
} from "@ordinconn/contracts";
import { useState } from "react";
import type { Translator } from "../i18n";

interface MobileActionControlsProps {
  session?: MobileDeviceSessionDto;
  snapshot?: MobileUiSnapshotDto;
  selected?: MobileElementDto;
  allowedApps: string[];
  receipt?: MobileActionReceiptDto | null;
  onAction: (input: MobileActionInputDto) => Promise<void>;
  t: Translator;
}

export function mobileActionAvailability(
  currentSession: MobileDeviceSessionDto | undefined,
  snapshot: MobileUiSnapshotDto | undefined,
  allowedApps: string[],
  now = Date.now(),
) {
  const age = snapshot ? now - Date.parse(snapshot.capturedAt) : Number.POSITIVE_INFINITY;
  const active = !!currentSession && !!snapshot && currentSession.status === "connected" &&
    currentSession.deviceType === "emulator" && currentSession.deviceId.startsWith("emulator-") &&
    currentSession.sessionId === snapshot.sessionId && currentSession.currentApp === snapshot.packageName &&
    allowedApps.includes(snapshot.packageName) && Number.isFinite(age) && age >= 0 && age <= 10_000;
  return { escape: active, navigate: active && !snapshot?.sensitiveState };
}

function safeElement(snapshot: MobileUiSnapshotDto | undefined, selected: MobileElementDto | undefined) {
  return !!snapshot && !!selected && selected.enabled && selected.text !== "[REDACTED]" &&
    !snapshot.redactions?.some((redaction) => redaction.startsWith(`element:${selected.ref}:`));
}

export function MobileActionControls({ session, snapshot, selected, allowedApps, receipt, onAction, t }: MobileActionControlsProps) {
  const [typeDraft, setTypeDraft] = useState("");
  const [openPackage, setOpenPackage] = useState(allowedApps[0] ?? "");
  const [busy, setBusy] = useState(false);
  const availability = mobileActionAvailability(session, snapshot, allowedApps);
  const validSelection = safeElement(snapshot, selected);
  const canTap = availability.navigate && validSelection && !!selected?.clickable && !busy;
  const canType = availability.navigate && validSelection && !!selected?.focused && selected.className.endsWith("EditText") &&
    /^[A-Za-z0-9 ]{1,256}$/.test(typeDraft) && !busy;
  const selectedPackage = allowedApps.includes(openPackage) ? openPackage : allowedApps[0] ?? "";

  const execute = async (target: MobileActionTargetDto, text?: string) => {
    if (!session || !snapshot || busy) return;
    const input: MobileActionInputDto = {
      sessionId: session.sessionId,
      snapshotId: snapshot.snapshotId,
      expectedPackage: snapshot.packageName,
      target,
      ...(text === undefined ? {} : { text }),
    };
    setBusy(true);
    if (target.kind === "type") setTypeDraft("");
    try { await onAction(input); } finally { setBusy(false); }
  };

  return <section className="mobile-action-panel" aria-label={t("mobile.manualActions")}>
    <h3>{t("mobile.manualActions")}</h3>
    <div className="mobile-action-row">
      <button className="secondary-button" type="button" disabled={!canTap} onClick={() => selected && void execute({ kind: "tap", elementRef: selected.ref })}>{t("mobile.tapSelected")}</button>
      <button className="secondary-button" type="button" disabled={!availability.escape || busy} onClick={() => void execute({ kind: "back" })}>{t("mobile.back")}</button>
      <button className="secondary-button" type="button" disabled={!availability.escape || busy} onClick={() => void execute({ kind: "home" })}>{t("mobile.home")}</button>
    </div>
    <div className="mobile-action-row">
      {(["up", "down", "left", "right"] as MobileSwipeDirectionDto[]).map((direction) => <button className="secondary-button" type="button" key={direction} disabled={!availability.navigate || busy} onClick={() => void execute({ kind: "swipe", direction })}>{t(`mobile.swipe.${direction}`)}</button>)}
    </div>
    <div className="mobile-action-row">
      <label>{t("mobile.safeType")}<input type="text" autoComplete="off" autoCorrect="off" spellCheck={false} maxLength={256} value={typeDraft} onChange={(event) => setTypeDraft(event.target.value)} /></label>
      <button className="secondary-button" type="button" disabled={!canType} onClick={() => selected && void execute({ kind: "type", elementRef: selected.ref }, typeDraft)}>{t("mobile.typeSelected")}</button>
    </div>
    <div className="mobile-action-row">
      <label>{t("mobile.allowedApp")}<select value={selectedPackage} onChange={(event) => setOpenPackage(event.target.value)}>{allowedApps.map((packageName) => <option key={packageName} value={packageName}>{packageName}</option>)}</select></label>
      <button className="secondary-button" type="button" disabled={!availability.navigate || !selectedPackage || busy} onClick={() => void execute({ kind: "open_app", packageName: selectedPackage })}>{t("mobile.openAllowedApp")}</button>
    </div>
    <p className="mobile-action-boundary">{t("mobile.manualBoundary")}</p>
    {receipt ? <div className="mobile-action-receipt" aria-live="polite"><strong>{t("mobile.lastAction")}</strong><span>{receipt.status} · {receipt.verification ?? (receipt.decision.outcome === "denied" ? receipt.decision.reason : "—")}</span>{receipt.textLength != null ? <span>{t("mobile.typeLength", { count: receipt.textLength })}</span> : null}</div> : null}
  </section>;
}
