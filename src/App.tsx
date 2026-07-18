import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { GestureOverlay } from "./GestureOverlay";
import { Settings } from "./Settings";
import type { AppSettings, ChatMessage } from "./types";
import "./App.css";

const DEFAULT_SETTINGS: AppSettings = {
  active: {
    provider: "ollama",
    model: "qwen3:4b",
    api_key: null,
    base_url: "http://localhost:11434",
  },
};

type Tab = "home" | "settings";

export default function App() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<Tab>("home");

  useEffect(() => {
    invoke<AppSettings>("get_settings")
      .then(setSettings)
      .catch(() => {}); // fall back to default if backend not ready
  }, []);

  async function sendQuery() {
    if (!input.trim() || loading) return;
    const userMsg: ChatMessage = { role: "user", content: input.trim() };
    const next = [...messages, userMsg];
    setMessages(next);
    setInput("");
    setLoading(true);
    try {
      const reply = await invoke<string>("chat", {
        messages: next.map((m) => ({ role: m.role, content: m.content })),
      });
      setMessages([...next, { role: "assistant", content: reply }]);
    } catch (e) {
      setMessages([...next, { role: "assistant", content: `Error: ${e}` }]);
    } finally {
      setLoading(false);
    }
  }

  const { provider, model } = settings.active;
  const isLocal = provider === "ollama";

  return (
    <div className="app">
      <nav className="navbar">
        <span className="nav-brand">Jarvis</span>

        <div className="model-badge" onClick={() => setActiveTab("settings")} title="Configure model">
          <span className={`dot ${isLocal ? "local" : "cloud"}`} />
          <span className="model-name">{model}</span>
          <span className="model-provider">{provider}</span>
        </div>

        <div className="nav-links">
          <button
            className={`nav-btn${activeTab === "home" ? " active" : ""}`}
            onClick={() => setActiveTab("home")}
          >
            Home
          </button>
          <button
            className={`nav-btn${activeTab === "settings" ? " active" : ""}`}
            onClick={() => setActiveTab("settings")}
          >
            Settings
          </button>
        </div>
      </nav>

      <GestureOverlay />

      <main className="main-content">
        {activeTab === "home" && (
          <div className="messages">
            {messages.length === 0 ? (
              <div className="empty-state">
                <h1>Hi, I'm Jarvis</h1>
                <p className="subtitle">Your AI assistant. How can I help you today?</p>
              </div>
            ) : (
              messages.map((msg, i) => (
                <div key={i} className={`message ${msg.role}`}>
                  {msg.content}
                </div>
              ))
            )}
            {loading && (
              <div className="message assistant thinking">
                <span className="dot-pulse" />
              </div>
            )}
          </div>
        )}

        {activeTab === "settings" && (
          <Settings
            settings={settings}
            onSaved={(s) => {
              setSettings(s);
              setActiveTab("home");
            }}
          />
        )}
      </main>

      <div className="input-bar">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            sendQuery();
          }}
        >
          <input
            value={input}
            onChange={(e) => setInput(e.currentTarget.value)}
            placeholder="What can I assist you with..."
            disabled={loading}
          />
          <button type="submit" disabled={loading || !input.trim()}>
            {loading ? "…" : "Send"}
          </button>
        </form>
      </div>
    </div>
  );
}
