//! Per-backend HTTP calls for the Quill router.
//!
//! Each function takes the shared [`QuillConfig`] plus the user prompt and
//! returns the assistant's reply text. All transport is blocking `ureq`.

use crate::{QuillConfig, QuillError};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read};

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

/// POST `body` and return the raw response reader for streaming, mapping
/// transport/HTTP failures onto [`QuillError`].
fn post_stream(
    cfg: &QuillConfig,
    provider: &'static str,
    url: &str,
    headers: &[(&str, &str)],
    body: Value,
) -> Result<Box<dyn Read + Send + Sync + 'static>, QuillError> {
    let mut req = agent(cfg).post(url);
    for (k, v) in headers {
        req = req.set(k, v);
    }
    match req.send_json(body) {
        Ok(resp) => Ok(resp.into_reader()),
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

/// Strip an SSE `data:` prefix, returning the JSON payload if present.
/// Returns `None` for comment/event lines and the `[DONE]` sentinel.
fn sse_data(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("data:")?.trim();
    if rest.is_empty() || rest == "[DONE]" {
        None
    } else {
        Some(rest)
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

// ── Streaming variants ────────────────────────────────────────────────────────

fn build_messages(cfg: &QuillConfig, prompt: &str) -> Vec<Value> {
    let mut messages = Vec::new();
    if let Some(sys) = &cfg.system {
        messages.push(json!({ "role": "system", "content": sys }));
    }
    messages.push(json!({ "role": "user", "content": prompt }));
    messages
}

/// Ollama streaming — `/api/chat` with `stream: true` returns newline-delimited
/// JSON, one object per token batch.
pub fn ollama_stream(
    cfg: &QuillConfig,
    prompt: &str,
    mut on_chunk: impl FnMut(&str),
) -> Result<String, QuillError> {
    let url = format!("{}/api/chat", cfg.ollama_host);
    let body = json!({
        "model": cfg.ollama_model,
        "messages": build_messages(cfg, prompt),
        "stream": true,
    });

    let reader = BufReader::new(post_stream(cfg, "ollama", &url, &[], body)?);
    let mut full = String::new();
    for line in reader.lines() {
        let line = line.map_err(|e| QuillError::BadResponse {
            provider: "ollama",
            reason: e.to_string(),
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(chunk) = v["message"]["content"].as_str() {
            if !chunk.is_empty() {
                full.push_str(chunk);
                on_chunk(chunk);
            }
        }
        if v["done"].as_bool() == Some(true) {
            break;
        }
    }
    Ok(full)
}

/// Claude streaming — `/v1/messages` with `stream: true` returns SSE. We collect
/// `content_block_delta` text deltas.
pub fn claude_stream(
    cfg: &QuillConfig,
    prompt: &str,
    mut on_chunk: impl FnMut(&str),
) -> Result<String, QuillError> {
    let key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| QuillError::MissingApiKey("ANTHROPIC_API_KEY"))?;

    let mut body = json!({
        "model": cfg.claude_model,
        "max_tokens": 1024,
        "stream": true,
        "messages": [{ "role": "user", "content": prompt }],
    });
    if let Some(sys) = &cfg.system {
        body["system"] = json!(sys);
    }

    let reader = BufReader::new(post_stream(
        cfg,
        "claude",
        "https://api.anthropic.com/v1/messages",
        &[
            ("x-api-key", key.as_str()),
            ("anthropic-version", "2023-06-01"),
        ],
        body,
    )?);

    let mut full = String::new();
    for line in reader.lines() {
        let line = line.map_err(|e| QuillError::BadResponse {
            provider: "claude",
            reason: e.to_string(),
        })?;
        let Some(data) = sse_data(&line) else { continue };
        let v: Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v["type"] == "content_block_delta" && v["delta"]["type"] == "text_delta" {
            if let Some(chunk) = v["delta"]["text"].as_str() {
                full.push_str(chunk);
                on_chunk(chunk);
            }
        } else if v["type"] == "message_stop" {
            break;
        }
    }
    Ok(full)
}

/// Gemini streaming — `:streamGenerateContent?alt=sse` returns SSE with one
/// candidate chunk per event.
pub fn gemini_stream(
    cfg: &QuillConfig,
    prompt: &str,
    mut on_chunk: impl FnMut(&str),
) -> Result<String, QuillError> {
    let key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| QuillError::MissingApiKey("GEMINI_API_KEY"))?;

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
        cfg.gemini_model, key
    );

    let mut body = json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
    });
    if let Some(sys) = &cfg.system {
        body["systemInstruction"] = json!({ "parts": [{ "text": sys }] });
    }

    let reader = BufReader::new(post_stream(cfg, "gemini", &url, &[], body)?);
    let mut full = String::new();
    for line in reader.lines() {
        let line = line.map_err(|e| QuillError::BadResponse {
            provider: "gemini",
            reason: e.to_string(),
        })?;
        let Some(data) = sse_data(&line) else { continue };
        let v: Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(chunk) = v["candidates"][0]["content"]["parts"][0]["text"].as_str() {
            full.push_str(chunk);
            on_chunk(chunk);
        }
    }
    Ok(full)
}
