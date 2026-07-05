//! Quantum Quill — the vault's agent assistant.
//!
//! A small LLM router that speaks to a local Ollama instance by default and can
//! be pointed at Claude or Gemini instead. Everything goes over blocking HTTP
//! (`ureq`) so callers can drive it from a simple `ask()` call.
//!
//! Configuration is resolved from the environment via [`QuillConfig::from_env`]:
//!
//! | Variable            | Meaning                                   | Default                     |
//! |---------------------|-------------------------------------------|-----------------------------|
//! | `QUILL_PROVIDER`    | `ollama` \| `claude` \| `gemini`          | `ollama`                    |
//! | `OLLAMA_HOST`       | base URL of the Ollama server             | `http://localhost:11434`    |
//! | `QUILL_OLLAMA_MODEL`| Ollama model tag                          | `llama3.2`                  |
//! | `QUILL_CLAUDE_MODEL`| Claude model id                           | `claude-opus-4-8`           |
//! | `QUILL_GEMINI_MODEL`| Gemini model id                           | `gemini-2.5-flash`          |
//! | `ANTHROPIC_API_KEY` | required when provider is `claude`        | —                           |
//! | `GEMINI_API_KEY`    | required when provider is `gemini`        | —                           |

mod providers;

pub use providers::Provider;

use std::time::Duration;

/// Error surface for every routing/transport failure.
#[derive(Debug, thiserror::Error)]
pub enum QuillError {
    #[error("unknown provider '{0}' (expected ollama|claude|gemini)")]
    UnknownProvider(String),

    #[error("missing API key: set {0}")]
    MissingApiKey(&'static str),

    #[error("{provider} request failed ({status}): {body}")]
    Http {
        provider: &'static str,
        status: u16,
        body: String,
    },

    #[error("{provider} transport error: {source}")]
    Transport {
        provider: &'static str,
        #[source]
        source: Box<ureq::Error>,
    },

    #[error("could not parse {provider} response: {reason}")]
    BadResponse {
        provider: &'static str,
        reason: String,
    },
}

/// Router configuration: which provider to use and per-provider model choices.
#[derive(Debug, Clone)]
pub struct QuillConfig {
    pub provider: Provider,
    pub ollama_host: String,
    pub ollama_model: String,
    pub claude_model: String,
    pub gemini_model: String,
    /// Optional system prompt sent with every request.
    pub system: Option<String>,
    /// Per-request timeout.
    pub timeout: Duration,
}

impl Default for QuillConfig {
    fn default() -> Self {
        Self {
            provider: Provider::Ollama,
            ollama_host: "http://localhost:11434".into(),
            ollama_model: "llama3.2".into(),
            claude_model: "claude-opus-4-8".into(),
            gemini_model: "gemini-2.5-flash".into(),
            system: Some(
                "You are Quantum Quill, the agent assistant living inside a Quantum Vault. \
                 Answer concisely and stay in character as a helpful vault scribe."
                    .into(),
            ),
            timeout: Duration::from_secs(60),
        }
    }
}

impl QuillConfig {
    /// Build a config from environment variables, falling back to [`Default`].
    pub fn from_env() -> Result<Self, QuillError> {
        let mut cfg = Self::default();
        if let Ok(p) = std::env::var("QUILL_PROVIDER") {
            if !p.trim().is_empty() {
                cfg.provider = Provider::from_str(&p).ok_or(QuillError::UnknownProvider(p))?;
            }
        }
        if let Ok(v) = std::env::var("OLLAMA_HOST") {
            if !v.trim().is_empty() {
                cfg.ollama_host = v.trim_end_matches('/').to_string();
            }
        }
        if let Ok(v) = std::env::var("QUILL_OLLAMA_MODEL") {
            cfg.ollama_model = v;
        }
        if let Ok(v) = std::env::var("QUILL_CLAUDE_MODEL") {
            cfg.claude_model = v;
        }
        if let Ok(v) = std::env::var("QUILL_GEMINI_MODEL") {
            cfg.gemini_model = v;
        }
        Ok(cfg)
    }
}

/// The routing entry point. Cheap to clone; holds only configuration.
#[derive(Debug, Clone)]
pub struct QuillRouter {
    config: QuillConfig,
}

impl QuillRouter {
    pub fn new(config: QuillConfig) -> Self {
        Self { config }
    }

    /// Build a router from the environment.
    pub fn from_env() -> Result<Self, QuillError> {
        Ok(Self::new(QuillConfig::from_env()?))
    }

    pub fn config(&self) -> &QuillConfig {
        &self.config
    }

    /// The provider this router will dispatch to.
    pub fn provider(&self) -> Provider {
        self.config.provider
    }

    /// Send a single-turn prompt to the configured provider and return the reply text.
    pub fn ask(&self, prompt: &str) -> Result<String, QuillError> {
        match self.config.provider {
            Provider::Ollama => providers::ollama(&self.config, prompt),
            Provider::Claude => providers::claude(&self.config, prompt),
            Provider::Gemini => providers::gemini(&self.config, prompt),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_parsing_is_case_insensitive() {
        assert_eq!(Provider::from_str("Ollama"), Some(Provider::Ollama));
        assert_eq!(Provider::from_str("CLAUDE"), Some(Provider::Claude));
        assert_eq!(Provider::from_str("gemini"), Some(Provider::Gemini));
        assert_eq!(Provider::from_str("gpt"), None);
    }

    #[test]
    fn default_config_targets_ollama() {
        let cfg = QuillConfig::default();
        assert_eq!(cfg.provider, Provider::Ollama);
        assert_eq!(cfg.ollama_host, "http://localhost:11434");
    }
}
