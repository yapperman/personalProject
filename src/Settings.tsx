import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, LlmConfig, Provider } from "./types";

const PROVIDER_DEFAULTS: Record<Provider, Partial<LlmConfig>> = {
  ollama: { model: "qwen3:4b", base_url: "http://localhost:11434" },
  anthropic: { model: "claude-haiku-4-5-20251001" },
  openai: { model: "gpt-4o-mini" },
};

const PROVIDER_LABELS: Record<Provider, string> = {
  ollama: "Ollama (Local)",
  anthropic: "Anthropic",
  openai: "OpenAI",
};

interface Props {
  settings: AppSettings;
  onSaved: (settings: AppSettings) => void;
}

export function Settings({ settings, onSaved }: Props) {
  const [config, setConfig] = useState<LlmConfig>({ ...settings.active });
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<{ ok: boolean; msg: string } | null>(null);

  function setProvider(provider: Provider) {
    const defaults = PROVIDER_DEFAULTS[provider];
    setConfig((c) => ({
      ...c,
      provider,
      model: defaults.model ?? c.model,
      base_url: defaults.base_url ?? null,
      api_key: provider === "ollama" ? null : c.api_key,
    }));
  }

  async function handleSave() {
    setSaving(true);
    setStatus(null);
    try {
      const next: AppSettings = { active: config };
      await invoke("save_settings", { newSettings: next });
      onSaved(next);
      setStatus({ ok: true, msg: "Settings saved." });
    } catch (e) {
      setStatus({ ok: false, msg: String(e) });
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="settings-panel">
      <h2>Model Configuration</h2>

      <section className="settings-section">
        <label className="settings-label">Provider</label>
        <div className="provider-toggles">
          {(["ollama", "anthropic", "openai"] as Provider[]).map((p) => (
            <button
              key={p}
              className={`provider-btn${config.provider === p ? " active" : ""}`}
              onClick={() => setProvider(p)}
            >
              {p === "ollama" && <span className="dot local" />}
              {p !== "ollama" && <span className="dot cloud" />}
              {PROVIDER_LABELS[p]}
            </button>
          ))}
        </div>
      </section>

      <section className="settings-section">
        <label className="settings-label" htmlFor="model-input">Model</label>
        <input
          id="model-input"
          className="settings-input"
          value={config.model}
          onChange={(e) => setConfig((c) => ({ ...c, model: e.target.value }))}
          placeholder="e.g. qwen3:4b"
        />
      </section>

      {config.provider === "ollama" && (
        <section className="settings-section">
          <label className="settings-label" htmlFor="base-url-input">Ollama URL</label>
          <input
            id="base-url-input"
            className="settings-input"
            value={config.base_url ?? ""}
            onChange={(e) => setConfig((c) => ({ ...c, base_url: e.target.value || null }))}
            placeholder="http://localhost:11434"
          />
        </section>
      )}

      {config.provider !== "ollama" && (
        <section className="settings-section">
          <label className="settings-label" htmlFor="api-key-input">API Key</label>
          <input
            id="api-key-input"
            className="settings-input"
            type="password"
            value={config.api_key ?? ""}
            onChange={(e) => setConfig((c) => ({ ...c, api_key: e.target.value || null }))}
            placeholder="Paste your API key here"
          />
          {config.provider === "openai" && (
            <p className="settings-hint">
              Supports any OpenAI-compatible endpoint — set a custom base URL via the Ollama provider if needed.
            </p>
          )}
        </section>
      )}

      <button className="save-btn" onClick={handleSave} disabled={saving}>
        {saving ? "Saving…" : "Apply & Save"}
      </button>

      {status && (
        <p className={`settings-status ${status.ok ? "ok" : "err"}`}>{status.msg}</p>
      )}
    </div>
  );
}
