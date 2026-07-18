export type Provider = "ollama" | "anthropic" | "openai";

export interface LlmConfig {
  provider: Provider;
  model: string;
  api_key: string | null;
  base_url: string | null;
}

export interface AppSettings {
  active: LlmConfig;
}

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
}
