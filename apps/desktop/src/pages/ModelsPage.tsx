import { KeyRound, ServerCog, TestTube2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import type { ModelProviderDto, ProviderCapabilitiesDto } from "@ordinconn/contracts";
import { Alert, EmptyState, SectionHeader, StatusBadge } from "../components/WorkspaceUI";
import type { Translator } from "../i18n";
import type { ConnectionTestResult, ProviderInput } from "../runtime/client";

const capabilityKeys = ["chatCompletions", "responses", "streaming", "toolCalling", "reasoning", "vision", "structuredOutput", "jsonMode"] as const;
const defaults: ProviderCapabilitiesDto = { chatCompletions: true, responses: false, streaming: true, toolCalling: true, reasoning: false, vision: false, structuredOutput: false, jsonMode: true };

interface ModelsPageProps {
  providers: ModelProviderDto[];
  onSave: (input: ProviderInput) => Promise<void>;
  onTest: (input: ProviderInput) => Promise<ConnectionTestResult>;
  t: Translator;
}

export function ModelsPage({ providers, onSave, onTest, t }: ModelsPageProps) {
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("http://localhost:11434/v1");
  const [apiKey, setApiKey] = useState("");
  const [modelId, setModelId] = useState("");
  const [temperature, setTemperature] = useState(0.2);
  const [contextWindow, setContextWindow] = useState(32768);
  const [capabilities, setCapabilities] = useState(defaults);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<ConnectionTestResult>();
  const input = (): ProviderInput => ({ name, baseUrl, apiKey: apiKey || undefined, modelId, temperature, contextWindow, enabled: true, capabilities });
  const submit = async (event: FormEvent) => { event.preventDefault(); await onSave(input()); setApiKey(""); };
  const test = async () => { setTesting(true); setTestResult(undefined); try { setTestResult(await onTest(input())); } catch (cause) { setTestResult({ ok: false, message: cause instanceof Error ? cause.message : typeof cause === "string" ? cause : t("common.error") }); } finally { setTesting(false); } };

  return (
    <section className="models-section">
      <SectionHeader title={t("models.title")} description={t("models.description")} />
      <div className="settings-grid">
        <form className="panel provider-form" onSubmit={submit}>
          <div className="panel-heading"><div><span className="eyebrow">{t("models.providerType")}</span><h2>{t("common.create")}</h2></div><ServerCog size={18} /></div>
          <div className="inline-fields">
          <label><span>{t("models.providerName")}</span><input required value={name} onChange={(event) => setName(event.target.value)} /></label>
          <label><span>{t("models.modelId")}</span><input required value={modelId} onChange={(event) => setModelId(event.target.value)} /></label>
          </div>
          <label><span>{t("models.baseUrl")}</span><input required type="url" value={baseUrl} onChange={(event) => setBaseUrl(event.target.value)} /></label>
          <div className="inline-fields">
            <label><span>{t("models.temperature")}</span><input required type="number" min="0" max="2" step="0.1" value={temperature} onChange={(event) => setTemperature(Number(event.target.value))} /></label>
            <label><span>{t("models.contextWindow")}</span><input required type="number" min="1" step="1" value={contextWindow} onChange={(event) => setContextWindow(Number(event.target.value))} /></label>
          </div>
          <label><span>{t("models.apiKey")}</span><div className="input-with-icon"><KeyRound size={15} /><input type="password" autoComplete="new-password" value={apiKey} onChange={(event) => setApiKey(event.target.value)} /></div><small>{t("models.secretHint")}</small></label>
          <fieldset><legend>{t("models.capabilities")}</legend><div className="capability-grid">{capabilityKeys.map((key) => <label className="check-row" key={key}><input type="checkbox" checked={capabilities[key]} onChange={(event) => setCapabilities((current) => ({ ...current, [key]: event.target.checked }))} /><span>{t(`models.capability.${key}`)}</span></label>)}</div></fieldset>
          <div className="form-actions"><button className="secondary-button" disabled={testing || !name || !modelId} onClick={test} type="button"><TestTube2 size={15} />{testing ? t("models.testPending") : t("common.test")}</button><button className="primary-button" type="submit" disabled={!name || !modelId}>{t("common.save")}</button></div>
          {testResult ? <Alert success={testResult.ok} title={t(testResult.ok ? "models.connectionSucceeded" : "models.connectionFailed")} description={testResult.ok ? undefined : t(testResult.message?.includes("rejected") ? "models.requestRejected" : "models.checkConnection")} detail={testResult.message} t={t} /> : null}
        </form>
        <article className="panel provider-list">
          <SectionHeader title={t("models.savedProviders")} />
          {providers.length === 0 ? <EmptyState title={t("models.noProvidersTitle")} description={t("models.noProviders")} /> : providers.map((provider) => <div className="card provider-row" key={provider.id}><div><strong>{provider.name}</strong><span>{provider.defaultModel}</span><small className="provider-url" title={provider.baseUrl}>{provider.baseUrl}</small><small>{t("models.temperature")}: {provider.temperature} · {t("models.contextWindow")}: {provider.contextWindow} {t("models.contextUnit")}</small></div><StatusBadge tone={provider.enabled ? "success" : "neutral"}>{provider.enabled ? t("common.enabled") : t("common.unavailable")}</StatusBadge></div>)}
        </article>
      </div>
    </section>
  );
}
