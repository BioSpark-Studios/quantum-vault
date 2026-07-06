//! Axum REST API for the Storefront.
//!
//! Public routes:
//!   POST   /auth/register              — create account, returns JWT
//!   POST   /auth/login                 — returns JWT
//!   GET    /blueprints                 — list all blueprints
//!   GET    /blueprints/:id             — get blueprint by id
//!   GET    /collections                — list all collections
//!   GET    /collections/:id            — get collection by id
//!   GET    /tomes                      — list all tomes
//!   GET    /tomes/:id                  — get tome by id
//!   GET    /status                     — RES wire packet summary
//!   GET    /quill                      — Quantum Quill web console (HTML)
//!   POST   /quill/ask                  — ask Quantum Quill (LLM router) a question
//!   POST   /quill/ask/stream           — ask Quantum Quill, streamed as SSE
//!   GET    /quill/settings             — read the persisted Quill config
//!   PUT    /quill/settings             — update the persisted Quill config
//!
//! Authenticated routes (Bearer JWT):
//!   GET    /me                         — current user profile + credits
//!   POST   /checkout/:blueprint_id     — purchase a blueprint
//!   POST   /capsules/upload            — upload a new capsule (multipart)

use crate::{
    auth::{verify_token, AuthResponse, LoginBody, MeResponse, RegisterBody, UserStore},
    checkout::PurchaseRecord,
    store::Storefront,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        Html, IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::Multipart;
use std::sync::{Arc, Mutex};
use tokio_stream::wrappers::ReceiverStream;

pub type StoreState = Arc<Mutex<Storefront>>;
pub type UserState = Arc<Mutex<UserStore>>;

#[derive(Clone)]
pub struct AppState {
    pub store: StoreState,
    pub users: UserState,
    pub vault_dir: std::path::PathBuf,
}

// ── Auth helper ───────────────────────────────────────────────────────────────

fn bearer_user_id(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    let token = auth.strip_prefix("Bearer ")?;
    verify_token(token).ok().map(|c| c.sub)
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(state: AppState) -> Router {
    Router::new()
        // Public
        .route("/auth/register",       post(register))
        .route("/auth/login",          post(login))
        .route("/blueprints",          get(list_blueprints))
        .route("/blueprints/:id",      get(get_blueprint))
        .route("/collections",         get(list_collections))
        .route("/collections/:id",     get(get_collection))
        .route("/tomes",               get(list_tomes))
        .route("/tomes/:id",           get(get_tome))
        .route("/status",              get(status))
        .route("/quill",               get(quill_page))
        .route("/quill/ask",           post(quill_ask))
        .route("/quill/ask/stream",    post(quill_ask_stream))
        .route("/quill/settings",      get(quill_get_settings).put(quill_put_settings))
        // Authenticated
        .route("/me",                  get(me))
        .route("/checkout/:id",        post(checkout))
        .route("/capsules/upload",     post(upload_capsule))
        .with_state(state)
}

/// Backwards-compatible router for callers that still pass a bare StoreState.
pub fn simple_router(store: StoreState) -> Router {
    let state = AppState {
        store,
        users: Arc::new(Mutex::new(UserStore::default())),
        vault_dir: std::path::PathBuf::from("."),
    };
    router(state)
}

// ── Quantum Quill ─────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct AskBody {
    prompt: String,
}

/// GET /quill — a self-contained web console for chatting with Quill and
/// editing its settings, backed by the JSON/SSE endpoints on this server.
async fn quill_page() -> Html<&'static str> {
    Html(QUILL_HTML)
}

const QUILL_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Quantum Quill</title>
<style>
  :root { --bg:#0d1f0d; --surface:#12281a; --border:#1f4023; --text:#dfeede;
          --muted:#7fa588; --accent:#4CAF50; --accent-dim:#2f6f33; }
  * { box-sizing:border-box; }
  body { margin:0; font:15px/1.5 system-ui,-apple-system,Segoe UI,Roboto,sans-serif;
         background:var(--bg); color:var(--text); }
  .wrap { max-width:760px; margin:0 auto; padding:28px 20px 60px; }
  h1 { font-size:22px; margin:0 0 2px; }
  h1 .pen { color:var(--accent); }
  .sub { color:var(--muted); font-size:13px; margin:0 0 22px; }
  .card { background:var(--surface); border:1px solid var(--border); border-radius:12px;
          padding:18px; margin-bottom:18px; }
  .card h2 { font-size:14px; letter-spacing:.04em; text-transform:uppercase;
             color:var(--accent); margin:0 0 14px; }
  label { display:block; font-size:12px; color:var(--muted); margin:12px 0 4px; }
  input, select, textarea { width:100%; background:var(--bg); color:var(--text);
          border:1px solid var(--border); border-radius:8px; padding:9px 10px; font:inherit; }
  textarea { resize:vertical; }
  button { background:var(--accent); color:#08150a; border:none; border-radius:8px;
           padding:10px 16px; font:inherit; font-weight:600; cursor:pointer; }
  button.secondary { background:transparent; color:var(--muted); border:1px solid var(--border); }
  button:disabled { opacity:.5; cursor:default; }
  .row { display:flex; gap:10px; align-items:center; flex-wrap:wrap; }
  .row .grow { flex:1; }
  .msg { font-size:13px; margin-top:10px; min-height:18px; }
  .ok { color:var(--accent); } .err { color:#e06666; }
  .hint { color:var(--muted); font-size:12px; }
  #out { white-space:pre-wrap; background:var(--bg); border:1px solid var(--border);
         border-radius:8px; padding:12px; min-height:80px; margin-top:12px; }
  .pill { font-size:12px; color:var(--muted); }
  .prov-fields { display:none; } .prov-fields.on { display:block; }
</style>
</head>
<body>
<div class="wrap">
  <h1><span class="pen">&#9998;</span> Quantum Quill</h1>
  <p class="sub">The vault's agent assistant &middot; configure and chat, no command line needed.</p>

  <div class="card">
    <h2>Ask</h2>
    <textarea id="prompt" rows="2" placeholder="Ask Quantum Quill&hellip;"></textarea>
    <div class="row" style="margin-top:10px">
      <button id="ask">Ask</button>
      <button id="clear" class="secondary">Clear</button>
      <span id="prov" class="pill"></span>
    </div>
    <div id="out"></div>
  </div>

  <div class="card">
    <h2>Configuration</h2>
    <p class="hint">Saved to <code>.vaultforge/quill.json</code>. API keys stay in the server's environment.</p>
    <label>Provider</label>
    <select id="provider">
      <option value="ollama">ollama</option>
      <option value="claude">claude</option>
      <option value="gemini">gemini</option>
    </select>

    <div class="prov-fields" data-prov="ollama">
      <label>Ollama host</label><input id="ollama_host">
      <label>Ollama model</label><input id="ollama_model">
    </div>
    <div class="prov-fields" data-prov="claude">
      <label>Claude model</label><input id="claude_model">
      <p class="hint">Requires <code>ANTHROPIC_API_KEY</code> in the server environment.</p>
    </div>
    <div class="prov-fields" data-prov="gemini">
      <label>Gemini model</label><input id="gemini_model">
      <p class="hint">Requires <code>GEMINI_API_KEY</code> in the server environment.</p>
    </div>

    <label>System prompt</label>
    <textarea id="system" rows="3"></textarea>

    <div class="row" style="margin-top:14px">
      <button id="save">Save settings</button>
      <span id="savemsg" class="msg"></span>
    </div>
  </div>
</div>

<script>
const $ = id => document.getElementById(id);
const FIELDS = ["ollama_host","ollama_model","claude_model","gemini_model","system"];

function syncProvFields() {
  const p = $("provider").value;
  document.querySelectorAll(".prov-fields").forEach(el =>
    el.classList.toggle("on", el.dataset.prov === p));
}

async function loadSettings() {
  try {
    const s = await (await fetch("/quill/settings")).json();
    $("provider").value = s.provider || "ollama";
    FIELDS.forEach(f => $(f).value = s[f] ?? "");
    syncProvFields();
  } catch (e) { $("savemsg").textContent = "Could not load settings: " + e; }
}

$("provider").addEventListener("change", syncProvFields);

$("save").addEventListener("click", async () => {
  const body = { provider: $("provider").value };
  FIELDS.forEach(f => body[f] = $(f).value);
  const m = $("savemsg");
  m.textContent = "Saving…"; m.className = "msg";
  try {
    const r = await fetch("/quill/settings", {
      method: "PUT", headers: {"Content-Type":"application/json"}, body: JSON.stringify(body),
    });
    if (r.ok) { m.textContent = "✓ Saved"; m.className = "msg ok"; }
    else { m.textContent = "✗ " + (await r.text()); m.className = "msg err"; }
  } catch (e) { m.textContent = "✗ " + e; m.className = "msg err"; }
});

$("clear").addEventListener("click", () => { $("out").textContent = ""; $("prov").textContent = ""; });

$("ask").addEventListener("click", async () => {
  const prompt = $("prompt").value.trim();
  if (!prompt) return;
  const out = $("out"); out.textContent = ""; $("prov").textContent = "";
  $("ask").disabled = true;
  try {
    const resp = await fetch("/quill/ask/stream", {
      method: "POST", headers: {"Content-Type":"application/json"}, body: JSON.stringify({prompt}),
    });
    const reader = resp.body.getReader();
    const dec = new TextDecoder();
    let buf = "";
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      buf += dec.decode(value, { stream: true });
      let i;
      while ((i = buf.indexOf("\n\n")) >= 0) {
        const frame = buf.slice(0, i); buf = buf.slice(i + 2);
        let ev = "message", data = [];
        for (const line of frame.split("\n")) {
          if (line.startsWith("event:")) ev = line.slice(6).trim();
          else if (line.startsWith("data:")) data.push(line.slice(5).replace(/^ /, ""));
        }
        const payload = data.join("\n");
        if (ev === "meta") $("prov").textContent = "via " + payload;
        else if (ev === "error") { out.textContent += "\n⚠ " + payload; }
        else if (ev === "done") { /* finished */ }
        else out.textContent += payload;
      }
    }
  } catch (e) { out.textContent += "\n⚠ " + e; }
  finally { $("ask").disabled = false; }
});

loadSettings();
</script>
</body>
</html>
"##;

/// POST /quill/ask — route a prompt through the Quill LLM router.
///
/// The router (provider + models) is resolved from `<vault>/.vaultforge/quill.json`
/// with environment overrides. Runs on a blocking thread since `quill` uses a
/// blocking HTTP client.
async fn quill_ask(State(app): State<AppState>, Json(body): Json<AskBody>) -> impl IntoResponse {
    let vault = app.vault_dir.clone();
    let joined = tokio::task::spawn_blocking(move || {
        let router = quill::QuillRouter::resolve(&vault)?;
        let provider = router.provider().label();
        router.ask(&body.prompt).map(|reply| (provider, reply))
    })
    .await;

    match joined {
        Ok(Ok((provider, reply))) => (
            StatusCode::OK,
            Json(serde_json::json!({ "provider": provider, "reply": reply })),
        )
            .into_response(),
        Ok(Err(e)) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("join error: {e}") })),
        )
            .into_response(),
    }
}

/// POST /quill/ask/stream — same as `/quill/ask` but streamed token-by-token
/// as Server-Sent Events. Emits a `meta` event (provider), unnamed `message`
/// events (text chunks), then a terminal `done` or `error` event.
async fn quill_ask_stream(
    State(app): State<AppState>,
    Json(body): Json<AskBody>,
) -> Sse<ReceiverStream<Result<Event, std::convert::Infallible>>> {
    let vault = app.vault_dir.clone();
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(64);

    tokio::task::spawn_blocking(move || match quill::QuillRouter::resolve(&vault) {
        Ok(router) => {
            let _ = tx.blocking_send(Ok(Event::default()
                .event("meta")
                .data(router.provider().label())));
            let res = router.ask_stream(&body.prompt, |chunk| {
                let _ = tx.blocking_send(Ok(Event::default().data(chunk)));
            });
            let terminal = match res {
                Ok(_) => Event::default().event("done").data("ok"),
                Err(e) => Event::default().event("error").data(e.to_string()),
            };
            let _ = tx.blocking_send(Ok(terminal));
        }
        Err(e) => {
            let _ = tx.blocking_send(Ok(Event::default().event("error").data(e.to_string())));
        }
    });

    Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default())
}

/// GET /quill/settings — read the persisted Quill configuration (no secrets).
async fn quill_get_settings(State(app): State<AppState>) -> impl IntoResponse {
    let settings = quill::QuillSettings::load(&app.vault_dir);
    (StatusCode::OK, Json(settings))
}

/// PUT /quill/settings — persist a new Quill configuration to disk.
async fn quill_put_settings(
    State(app): State<AppState>,
    Json(settings): Json<quill::QuillSettings>,
) -> impl IntoResponse {
    match settings.save(&app.vault_dir) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "saved" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ── Auth handlers ─────────────────────────────────────────────────────────────

async fn register(
    State(app): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> impl IntoResponse {
    let mut users = app.users.lock().unwrap();
    match users.register(&body.username, &body.display_name, &body.password) {
        Ok(uid) => {
            let user = users.get(&uid).unwrap();
            let token = crate::auth::issue_token(user).unwrap_or_default();
            Json(AuthResponse {
                token,
                user_id: user.id.clone(),
                username: user.username.clone(),
                display_name: user.display_name.clone(),
                credits: user.credits,
            }).into_response()
        }
        Err(e) => (StatusCode::CONFLICT, Json(serde_json::json!({"error": e}))).into_response(),
    }
}

async fn login(
    State(app): State<AppState>,
    Json(body): Json<LoginBody>,
) -> impl IntoResponse {
    let users = app.users.lock().unwrap();
    match users.login(&body.username, &body.password) {
        Some(user) => {
            let token = crate::auth::issue_token(user).unwrap_or_default();
            Json(AuthResponse {
                token,
                user_id: user.id.clone(),
                username: user.username.clone(),
                display_name: user.display_name.clone(),
                credits: user.credits,
            }).into_response()
        }
        None => (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid credentials"}))).into_response(),
    }
}

async fn me(
    State(app): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match bearer_user_id(&headers) {
        None => (StatusCode::UNAUTHORIZED, "missing or invalid token").into_response(),
        Some(uid) => {
            let users = app.users.lock().unwrap();
            match users.get(&uid) {
                Some(user) => Json(MeResponse::from(user)).into_response(),
                None => (StatusCode::NOT_FOUND, "user not found").into_response(),
            }
        }
    }
}

// ── Checkout ──────────────────────────────────────────────────────────────────

async fn checkout(
    State(app): State<AppState>,
    headers: HeaderMap,
    Path(blueprint_id): Path<String>,
) -> impl IntoResponse {
    let uid = match bearer_user_id(&headers) {
        None => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "not authenticated"}))).into_response(),
        Some(u) => u,
    };

    // Get blueprint price
    let price = {
        let store = app.store.lock().unwrap();
        match store.get_blueprint(&blueprint_id) {
            None => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "blueprint not found"}))).into_response(),
            Some(bp) => bp.price_credits,
        }
    };

    // Check already purchased
    {
        let users = app.users.lock().unwrap();
        if let Some(u) = users.get(&uid) {
            if u.purchased_blueprint_ids.contains(&blueprint_id) {
                return (StatusCode::CONFLICT, Json(serde_json::json!({"error": "already purchased"}))).into_response();
            }
        }
    }

    // Spend credits
    let remaining = {
        let mut users = app.users.lock().unwrap();
        match users.spend_credits(&uid, price) {
            Ok(r) => r,
            Err(e) => return (StatusCode::PAYMENT_REQUIRED, Json(serde_json::json!({"error": e}))).into_response(),
        }
    };

    // Record purchase
    {
        let mut users = app.users.lock().unwrap();
        if let Some(u) = users.get_mut(&uid) {
            u.purchased_blueprint_ids.push(blueprint_id.clone());
        }
    }

    let record = PurchaseRecord::new(&uid, &blueprint_id, price);
    Json(serde_json::json!({
        "record": record,
        "remaining_credits": remaining,
    })).into_response()
}

// ── Capsule upload ─────────────────────────────────────────────────────────────

async fn upload_capsule(
    State(app): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let uid = match bearer_user_id(&headers) {
        None => return (StatusCode::UNAUTHORIZED, "not authenticated").into_response(),
        Some(u) => u,
    };

    let mut yaml_bytes: Option<Vec<u8>> = None;
    let mut png_bytes: Option<Vec<u8>> = None;
    let mut filename = String::from("uploaded");

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        let fname = field.file_name().unwrap_or("").to_string();
        let data = field.bytes().await.unwrap_or_default().to_vec();

        match field_name.as_str() {
            "capsule" => { yaml_bytes = Some(data); filename = fname; }
            "thumbnail" => { png_bytes = Some(data); }
            _ => {}
        }
    }

    let yaml = match yaml_bytes {
        None => return (StatusCode::BAD_REQUEST, "missing 'capsule' field").into_response(),
        Some(y) => y,
    };

    // Validate it's non-empty and parses as UTF-8
    let raw: serde_json::Value = match String::from_utf8(yaml.clone())
        .map_err(|e| e.to_string())
        .and_then(|s| serde_json::from_str(&s).or_else(|_| {
            // try YAML-style: key: value lines → loose json object
            let mut map = serde_json::Map::new();
            for line in s.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    map.insert(k.trim().to_string(), serde_json::Value::String(v.trim().to_string()));
                }
            }
            Ok(serde_json::Value::Object(map))
        }))
    {
        Ok(v) => v,
        Err(e) => return (StatusCode::UNPROCESSABLE_ENTITY, format!("invalid capsule file: {e}")).into_response(),
    };

    // Required fields check
    for field in ["id", "persona", "tier"] {
        if raw[field].is_null() {
            return (StatusCode::UNPROCESSABLE_ENTITY, format!("missing field: {field}")).into_response();
        }
    }

    // Assign a lineage hash based on uploader + original id
    let original_id = raw["id"].as_str().unwrap_or("unknown");
    let lineage_hash = myth_wire::lineage_hash(&uid, original_id);

    // Save the YAML to vault_dir/capsules/
    let capsule_dir = app.vault_dir.join("capsules");
    if let Err(e) = std::fs::create_dir_all(&capsule_dir) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("cannot create capsule dir: {e}")).into_response();
    }
    let safe_name = filename.replace(['/', '\\'], "_").replace("..", "_");
    let dest = capsule_dir.join(&safe_name);
    if let Err(e) = std::fs::write(&dest, &yaml) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("write failed: {e}")).into_response();
    }

    // Optionally save thumbnail PNG
    if let Some(png) = png_bytes {
        let thumb_path = dest.with_extension("thumb.png");
        let _ = std::fs::write(thumb_path, png);
    }

    Json(serde_json::json!({
        "saved": dest.to_string_lossy(),
        "lineage_hash": lineage_hash,
        "uploaded_by": uid,
    })).into_response()
}

// ── Public store handlers (unchanged) ─────────────────────────────────────────

async fn list_blueprints(State(app): State<AppState>) -> impl IntoResponse {
    let sf = app.store.lock().unwrap();
    let bps: Vec<_> = sf.all_blueprints().into_iter().cloned().collect();
    Json(bps)
}

async fn get_blueprint(State(app): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let sf = app.store.lock().unwrap();
    match sf.get_blueprint(&id) {
        Some(bp) => Json(serde_json::to_value(bp).unwrap()).into_response(),
        None => (StatusCode::NOT_FOUND, "blueprint not found").into_response(),
    }
}

async fn list_collections(State(app): State<AppState>) -> impl IntoResponse {
    let sf = app.store.lock().unwrap();
    let cols: Vec<_> = sf.collections.values().cloned().collect();
    Json(cols)
}

async fn get_collection(State(app): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let sf = app.store.lock().unwrap();
    match sf.collections.get(&id) {
        Some(c) => Json(serde_json::to_value(c).unwrap()).into_response(),
        None => (StatusCode::NOT_FOUND, "collection not found").into_response(),
    }
}

async fn list_tomes(State(app): State<AppState>) -> impl IntoResponse {
    let sf = app.store.lock().unwrap();
    let tomes: Vec<_> = sf.tomes.values().cloned().collect();
    Json(tomes)
}

async fn get_tome(State(app): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let sf = app.store.lock().unwrap();
    match sf.tomes.get(&id) {
        Some(t) => Json(serde_json::to_value(t).unwrap()).into_response(),
        None => (StatusCode::NOT_FOUND, "tome not found").into_response(),
    }
}

async fn status(State(app): State<AppState>) -> impl IntoResponse {
    let sf = app.store.lock().unwrap();
    Json(sf.emit_res_packet().payload)
}
