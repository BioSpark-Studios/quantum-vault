//! Per-backend HTTP calls for the Quill router.
//!
//! Each function takes the shared [`QuillConfig`] plus the user prompt and
//! returns the assistant's reply text. All transport is blocking `ureq`.

use crate::{QuillConfig, QuillError};
use serde_json::{json, Value};

/// Which language-model backend to route a request to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// Local Ollama server (the default).
    Ollama,
    /// Anthropic Claude via the Messages API.
    Claude,
    /// Google Gemini via the Generative Language API.
    Gemini,
}

impl Provider {
    /// Parse a provider name, case-insensitively.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ollama" => Some(Provider::Ollama),
            "claude" | "anthropic" => Some(Provider::Claude),
            "gemini" | "google" => Some(Provider::Gemini),
            _ => None,
        }
    }

    /// Lowercase label, handy for logging.
    pub fn label(self) -> &'static str {
        match self {
            Provider::Ollama => "ollama",
            Provider::Claude => "claude",
            Provider::Gemini => "gemini",
        }
    }
}

fn agent(cfg: &QuillConfig) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(cfg.timeout)
        .build()
}

/// POST `body` to `url` with extra headers and decode the JSON response,
/// mapping every failure onto [`QuillError`].
fn post_json(
    cfg: &QuillConfig,
    provider: &'static str,
    url: &str,
    headers: &[(&str, &str)],
    body: Value,
) -> Result<Value, QuillError> {
    let mut req = agent(cfg).post(url);
    for (k, v) in headers {
        req = req.set(k, v);
    }
    match req.send_json(body) {
        Ok(resp) => resp
            .into_json::<Value>()
            .map_err(|e| QuillError::BadResponse {
                provider,
                reason: e.to_string(),
            }),
        Err(ureq::Error::Status(status, resp)) => {
            let body = resp
                .into_string()
                .unwrap_or_else(|_| "<no body>".to_string());
            Err(QuillError::Http {
                provider,
                status,
                body,
            })
        }
        Err(e) => Err(QuillError::Transport {
            provider,
            source: Box::new(e),
        }),
    }
}

/// Ollama — `POST {host}/api/chat` (non-streaming). No API key required.
pub fn ollama(cfg: &QuillConfig, prompt: &str) -> Result<String, QuillError> {
    let url = format!("{}/api/chat", cfg.ollama_host);

    let mut messages = Vec::new();
    if let Some(sys) = &cfg.system {
        messages.push(json!({ "role": "system", "content": sys }));
    }
    messages.push(json!({ "role": "user", "content": prompt }));

    let body = json!({
        "model": cfg.ollama_model,
        "messages": messages,
        "stream": false,
    });

    let resp = post_json(cfg, "ollama", &url, &[], body)?;
    resp["message"]["content"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| QuillError::BadResponse {
            provider: "ollama",
            reason: "missing message.content".into(),
        })
}

/// Claude — `POST https://api.anthropic.com/v1/messages`.
///
/// Requires `ANTHROPIC_API_KEY`. Uses the `anthropic-version: 2023-06-01` wire
/// header and a single-turn user message.
pub fn claude(cfg: &QuillConfig, prompt: &str) -> Result<String, QuillError> {
    let key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| QuillError::MissingApiKey("ANTHROPIC_API_KEY"))?;

    let mut body = json!({
        "model": cfg.claude_model,
        "max_tokens": 1024,
        "messages": [{ "role": "user", "content": prompt }],
    });
    if let Some(sys) = &cfg.system {
        body["system"] = json!(sys);
    }

    let resp = post_json(
        cfg,
        "claude",
        "https://api.anthropic.com/v1/messages",
        &[
            ("x-api-key", key.as_str()),
            ("anthropic-version", "2023-06-01"),
        ],
        body,
    )?;

    // content is an array of blocks; take the first text block.
    resp["content"]
        .as_array()
        .and_then(|blocks| {
            blocks
                .iter()
                .find(|b| b["type"] == "text")
                .and_then(|b| b["text"].as_str())
        })
        .map(str::to_string)
        .ok_or_else(|| QuillError::BadResponse {
            provider: "claude",
            reason: "no text block in content".into(),
        })
}

/// Gemini — `POST .../v1beta/models/{model}:generateContent?key=…`.
///
/// Requires `GEMINI_API_KEY`.
pub fn gemini(cfg: &QuillConfig, prompt: &str) -> Result<String, QuillError> {
    let key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| QuillError::MissingApiKey("GEMINI_API_KEY"))?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        cfg.gemini_model, key
    );

    let mut body = json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
    });
    if let Some(sys) = &cfg.system {
        body["systemInstruction"] = json!({ "parts": [{ "text": sys }] });
    }

    let resp = post_json(cfg, "gemini", &url, &[], body)?;
    resp["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| QuillError::BadResponse {
            provider: "gemini",
            reason: "no text in candidates[0]".into(),
        })
}
