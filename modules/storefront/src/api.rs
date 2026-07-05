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
//!   POST   /quill/ask                  — ask Quantum Quill (LLM router) a question
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
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::Multipart;
use std::sync::{Arc, Mutex};

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
        .route("/quill/ask",           post(quill_ask))
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

/// POST /quill/ask — route a prompt through the Quill LLM router.
///
/// The router (provider + models) is resolved from the process environment.
/// Runs on a blocking thread since `quill` uses a blocking HTTP client.
async fn quill_ask(Json(body): Json<AskBody>) -> impl IntoResponse {
    let joined = tokio::task::spawn_blocking(move || {
        let router = quill::QuillRouter::from_env()?;
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
