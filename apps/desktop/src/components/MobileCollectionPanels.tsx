import { useRef, useState } from "react";
import type { MobileCollectionWorkspace, MobileDataObject, MobileDataProvenance } from "@ordinconn/contracts";
import type { Translator } from "../i18n";
export interface CollectionPanelsProps {collection:MobileCollectionWorkspace;t:Translator;onExtract:()=>Promise<void>;onProvenance:(id:string)=>Promise<MobileDataProvenance>;connected:boolean}
export function filterMobileData(objects:MobileDataObject[],filter:string):MobileDataObject[]{return objects.filter(o=>filter==="all"||filter==="realtime"||o.objectType===filter);}
export function MobileProvenanceDetail({source,t}:{source:MobileDataProvenance;t:Translator}) {
  const o=source.object;
  return <section className="mobile-provenance" aria-label={t("collection.provenance")}>
    <h3>{t("collection.provenance")}</h3><p>{o.content}</p>
    <dl>{[["device",o.deviceId],["app",o.packageName],["package",o.packageName],["activity",source.observation.activityName],["capturedAt",o.capturedAt],["observationId",o.observationId],["elementId",o.provenance.elementIds.join(", ")],["method",t(`collection.method.${o.provenance.extractionMethod}`)],["deduplicationKey",o.deduplicationKey],["sightings",String(source.sightingCount)]].map(([key,value])=><div key={key}><dt>{t(`collection.${key}`)}</dt><dd>{value}</dd></div>)}</dl>
    <details><summary>{t("collection.originalEvidence")}</summary>{source.observation.elements.filter(e=>o.provenance.elementIds.includes(e.id)).map(e=><dl key={e.id}><div><dt>{t("collection.elementId")}</dt><dd>{e.id}</dd></div><div><dt>{t("collection.text")}</dt><dd>{e.text??e.contentDescription??"—"}</dd></div><div><dt>{t("collection.resourceId")}</dt><dd>{e.resourceId??"—"}</dd></div><div><dt>{t("collection.bounds")}</dt><dd>{e.bounds.x},{e.bounds.y} {e.bounds.width}×{e.bounds.height}</dd></div></dl>)}
    {source.extractedData.map(d=><p key={d.id}>{t(`collection.extract.${d.type}`)} · {d.value} · {t(`collection.method.${d.extractionMethod}`)} · {Math.round(d.confidence*100)}%</p>)}</details>
  </section>;
}
export function MobileDataStream({collection,t,onProvenance}:CollectionPanelsProps){
  const [filter,setFilter]=useState("realtime");const [source,setSource]=useState<MobileDataProvenance>();const [error,setError]=useState(false);const [loading,setLoading]=useState(false);const generation=useRef(0);
  const select=async(id:string)=>{const request=++generation.current;setLoading(true);setSource(undefined);setError(false);try{const value=await onProvenance(id);if(request===generation.current)setSource(value);}catch{if(request===generation.current)setError(true);}finally{if(request===generation.current)setLoading(false);}};
  const objects=filterMobileData(collection.dataObjects,filter);
  return <section className="workbench-panel mobile-data-stream" aria-label={t("collection.dataStream")}>
    <header className="workbench-panel-header"><div><h2>{t("collection.dataStream")}</h2><p>{t("collection.localOnly")}</p></div><span>{collection.dataObjects.length}</span></header>
    <div className="mobile-data-filters">{["realtime","all","text","list","metric","status","content"].map(key=><button className="secondary-button" type="button" key={key} aria-pressed={filter===key} onClick={()=>setFilter(key)}>{t(`collection.filter.${key}`)}</button>)}</div>
    <div className="mobile-data-items">{objects.length?objects.map(o=><button className="mobile-data-item secondary-button" type="button" key={o.id} onClick={()=>void select(o.id)}><strong>{o.content}</strong><span>{o.packageName}</span><time dateTime={o.capturedAt}>{new Date(o.capturedAt).toLocaleString()}</time><span>{t(`collection.method.${o.provenance.extractionMethod}`)}</span></button>):<p className="workspace-empty">{t("collection.empty")}</p>}</div>
    {loading?<p role="status">{t("common.loading")}</p>:null}{error?<p role="alert">{t("collection.provenanceError")}</p>:null}
    {source?<><button type="button" className="secondary-button" onClick={()=>{generation.current++;setSource(undefined);}}>{t("common.close")}</button><MobileProvenanceDetail source={source} t={t}/></>:null}
  </section>;
}
export function MobileObservationContext({collection,t,onExtract,connected}:CollectionPanelsProps){
  const o=collection.observations[0];const action=collection.actions[0];
  const diff=collection.diffs.find(d=>d.afterObservationId===o?.id);const [busy,setBusy]=useState(false);const [error,setError]=useState(false);
  const extract=async()=>{setBusy(true);setError(false);try{await onExtract();}catch{setError(true);}finally{setBusy(false);}};
  return <section className="workbench-panel mobile-observation-context" aria-label={t("collection.context")}><header className="workbench-panel-header"><div><h2>{t("collection.context")}</h2><p>{t("collection.deterministic")}</p></div></header>
    {o?<dl>{[["app",o.packageName],["activity",o.activityName],["elementCount",String(o.elementCount)],["capturedAt",o.capturedAt],["changed",diff?t(diff.changed?"collection.yes":"collection.no"):"—"],["added",String(diff?.addedElements.length??0)],["removed",String(diff?.removedElements.length??0)],["changedElements",String(diff?.changedElements.length??0)],["extractedCount",String(collection.lastExtractionCount)],["newObjectCount",String(collection.lastNewObjectCount)]].map(([key,value])=><div key={key}><dt>{t(`collection.${key}`)}</dt><dd>{value}</dd></div>)}</dl>:<p className="workspace-empty">{t("collection.observeFirst")}</p>}
    {diff?<div>{diff.changeTypes.map(k=><p key={k}>{t(`collection.diff.${k}`)}</p>)}<details><summary>{t("collection.diffElements")}</summary><p>{t("collection.added")}: {diff.addedElements.join(", ")||"—"}</p><p>{t("collection.removed")}: {diff.removedElements.join(", ")||"—"}</p><p>{t("collection.changedElements")}: {diff.changedElements.join(", ")||"—"}</p></details></div>:null}
    {action?<div className="mobile-action-receipt"><strong>{t("mobile.lastAction")}</strong><p>{t(`collection.action.${action.actionType}`)} · {t(`collection.actionStatus.${action.status}`)}</p><p>{action.errorCode??""}</p><dl><div><dt>{t("collection.beforeObservation")}</dt><dd>{action.beforeObservationId??"—"}</dd></div><div><dt>{t("collection.afterObservation")}</dt><dd>{action.afterObservationId??"—"}</dd></div></dl></div>:null}
    <button type="button" className="secondary-button" disabled={!o||!connected||busy} onClick={()=>void extract()}>{t("collection.extractPage")}</button>{error?<p role="alert">{t("collection.extractError")}</p>:null}
    <p>{t("collection.agentBoundary")}</p>
  </section>;
}
