import { CheckCircle2, KeyRound, ServerCog, TestTube2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import type { ModelProviderDto, ProviderCapabilitiesDto } from "@ordinconn/contracts";
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
  const test = async () => { setTesting(true); setTestResult(undefined); try { setTestResult(await onTest(input())); } finally { setTesting(false); } };

  return (
    <section className="page">
      <header className="page-header"><span className="eyebrow">{t("nav.models")}</span><h1>{t("models.title")}</h1><p>{t("models.description")}</p></header>
      <div className="settings-grid">
        <form className="panel provider-form" onSubmit={submit}>
          <div className="panel-heading"><div><span className="eyebrow">{t("models.providerType")}</span><h2>{t("common.create")}</h2></div><ServerCog size={18} /></div>
          <label><span>{t("models.providerName")}</span><input required value={name} onChange={(event) => setName(event.target.value)} /></label>
          <label><span>{t("models.baseUrl")}</span><input required type="url" value={baseUrl} onChange={(event) => setBaseUrl(event.target.value)} /></label>
          <label><span>{t("models.modelId")}</span><input required value={modelId} onChange={(event) => setModelId(event.target.value)} /></label>
          <div className="inline-fields">
            <label><span>{t("models.temperature")}</span><input required type="number" min="0" max="2" step="0.1" value={temperature} onChange={(event) => setTemperature(Number(event.target.value))} /></label>
            <label><span>{t("models.contextWindow")}</span><input required type="number" min="1" step="1" value={contextWindow} onChange={(event) => setContextWindow(Number(event.target.value))} /></label>
          </div>
          <label><span>{t("models.apiKey")}</span><div className="input-with-icon"><KeyRound size={15} /><input type="password" autoComplete="new-password" value={apiKey} onChange={(event) => setApiKey(event.target.value)} /></div><small>{t("models.secretHint")}</small></label>
          <fieldset><legend>{t("models.capabilities")}</legend><div className="capability-grid">{capabilityKeys.map((key) => <label className="check-row" key={key}><input type="checkbox" checked={capabilities[key]} onChange={(event) => setCapabilities((current) => ({ ...current, [key]: event.target.checked }))} /><span>{t(`models.capability.${key}`)}</span></label>)}</div></fieldset>
          <div className="form-actions"><button className="secondary-button" disabled={testing || !name || !modelId} onClick={test} type="button"><TestTube2 size={15} />{testing ? t("models.testPending") : t("common.test")}</button><button className="gold-button" type="submit" disabled={!name || !modelId}>{t("common.save")}</button></div>
          {testResult ? <div className={testResult.ok ? "test-result success" : "test-result error"}>{testResult.ok ? <CheckCircle2 size={15} /> : null}{testResult.message || t(testResult.ok ? "models.connectionSucceeded" : "models.connectionFailed")}</div> : null}
        </form>
        <article className="panel provider-list">
          <span className="eyebrow">{t("models.savedProviders")}</span>
          {providers.length === 0 ? <p className="empty-copy">{t("models.noProviders")}</p> : providers.map((provider) => <div className="provider-row" key={provider.id}><div><strong>{provider.name}</strong><span>{provider.defaultModel}</span><small>{provider.baseUrl}</small><small>{t("models.temperature")}: {provider.temperature} · {t("models.contextWindow")}: {provider.contextWindow} {t("models.contextUnit")}</small></div><span className={provider.enabled ? "state-chip ready" : "state-chip"}>{provider.enabled ? t("common.enabled") : t("common.unavailable")}</span></div>)}
        </article>
      </div>
    </section>
  );
}
