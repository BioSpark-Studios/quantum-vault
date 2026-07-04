//! Axum REST API for the Storefront.
//!
//! Routes:
//!   GET    /blueprints               — list all blueprints
//!   GET    /blueprints/:id           — get blueprint by id
//!   GET    /collections              — list all collections
//!   GET    /collections/:id          — get collection by id
//!   GET    /tomes                    — list all tomes
//!   GET    /tomes/:id                — get tome by id
//!   GET    /status                   — RES wire packet summary

use crate::store::Storefront;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::sync::{Arc, Mutex};

pub type StoreState = Arc<Mutex<Storefront>>;

pub fn router(state: StoreState) -> Router {
    Router::new()
        .route("/blueprints", get(list_blueprints))
        .route("/blueprints/:id", get(get_blueprint))
        .route("/collections", get(list_collections))
        .route("/collections/:id", get(get_collection))
        .route("/tomes", get(list_tomes))
        .route("/tomes/:id", get(get_tome))
        .route("/status", get(status))
        .with_state(state)
}

async fn list_blueprints(State(sf): State<StoreState>) -> impl IntoResponse {
    let sf = sf.lock().unwrap();
    let bps: Vec<_> = sf.all_blueprints().into_iter().cloned().collect();
    Json(bps)
}

async fn get_blueprint(
    State(sf): State<StoreState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let sf = sf.lock().unwrap();
    match sf.get_blueprint(&id) {
        Some(bp) => Json(serde_json::to_value(bp).unwrap()).into_response(),
        None => (StatusCode::NOT_FOUND, "blueprint not found").into_response(),
    }
}

async fn list_collections(State(sf): State<StoreState>) -> impl IntoResponse {
    let sf = sf.lock().unwrap();
    let cols: Vec<_> = sf.collections.values().cloned().collect();
    Json(cols)
}

async fn get_collection(
    State(sf): State<StoreState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let sf = sf.lock().unwrap();
    match sf.collections.get(&id) {
        Some(c) => Json(serde_json::to_value(c).unwrap()).into_response(),
        None => (StatusCode::NOT_FOUND, "collection not found").into_response(),
    }
}

async fn list_tomes(State(sf): State<StoreState>) -> impl IntoResponse {
    let sf = sf.lock().unwrap();
    let tomes: Vec<_> = sf.tomes.values().cloned().collect();
    Json(tomes)
}

async fn get_tome(
    State(sf): State<StoreState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let sf = sf.lock().unwrap();
    match sf.tomes.get(&id) {
        Some(t) => Json(serde_json::to_value(t).unwrap()).into_response(),
        None => (StatusCode::NOT_FOUND, "tome not found").into_response(),
    }
}

async fn status(State(sf): State<StoreState>) -> impl IntoResponse {
    let sf = sf.lock().unwrap();
    let pkt = sf.emit_res_packet();
    Json(pkt.payload)
}
