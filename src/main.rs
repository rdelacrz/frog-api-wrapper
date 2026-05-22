use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tracing::{error, info};

// ---------------------------------------------------------------------------
// Application state shared across handlers
// ---------------------------------------------------------------------------
struct AppState {
    frog_base_url: String,
    frog_api_key: String,
    client: Client,
}

// ---------------------------------------------------------------------------
// Handler: POST /v1/chat/completions
//
// Accepts an OpenAI-compatible chat completions request, forwards it to the
// Frog API, and streams / returns the upstream response unchanged.
// ---------------------------------------------------------------------------
async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let upstream_url = format!("{}/chat/completions", state.frog_base_url);

    info!(
        model = body
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
        "Forwarding chat completions request to Frog API"
    );

    // Check whether the caller requested streaming so we can forward the
    // Accept header correctly (optional – forwarded verbatim below).
    let stream = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Build the upstream request.
    let mut req_builder = state
        .client
        .post(&upstream_url)
        .header("Authorization", format!("Bearer {}", state.frog_api_key))
        .header("Content-Type", "application/json");

    // Forward Accept header if present (important for streaming SSE).
    if let Some(accept) = headers.get("accept") {
        req_builder = req_builder.header("Accept", accept.clone());
    }

    let upstream_resp = match req_builder.json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to reach Frog API: {e}");
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": {
                        "message": format!("Failed to reach Frog API: {e}"),
                        "type": "proxy_error"
                    }
                })),
            )
                .into_response();
        }
    };

    let status = StatusCode::from_u16(upstream_resp.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    // Forward all upstream response headers that are relevant.
    let mut resp_headers = HeaderMap::new();
    for (name, value) in upstream_resp.headers() {
        // Skip hop-by-hop headers that must not be forwarded.
        let lower = name.as_str().to_lowercase();
        if matches!(
            lower.as_str(),
            "transfer-encoding" | "connection" | "keep-alive" | "upgrade" | "proxy-authenticate"
        ) {
            continue;
        }
        if let Ok(v) = HeaderValue::from_bytes(value.as_bytes()) {
            resp_headers.insert(name.clone(), v);
        }
    }

    if stream {
        // For streaming responses, pipe the bytes directly so SSE works.
        let byte_stream = upstream_resp.bytes_stream();
        let body = Body::from_stream(byte_stream);
        let mut response = Response::new(body);
        *response.status_mut() = status;
        *response.headers_mut() = resp_headers;
        return response;
    }

    // For non-streaming responses, collect the body and return it.
    match upstream_resp.bytes().await {
        Ok(bytes) => {
            let mut response = Response::new(Body::from(bytes));
            *response.status_mut() = status;
            *response.headers_mut() = resp_headers;
            response
        }
        Err(e) => {
            error!("Failed to read Frog API response body: {e}");
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": {
                        "message": format!("Failed to read Frog API response: {e}"),
                        "type": "proxy_error"
                    }
                })),
            )
                .into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
#[tokio::main]
async fn main() {
    // Load .env file if present (silently ignore if missing).
    let _ = dotenvy::dotenv();

    // Initialise structured logging (RUST_LOG controls the filter).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "frog_api_wrapper=info,tower_http=info".parse().unwrap()),
        )
        .init();

    // Read configuration from environment variables.
    let frog_api_key = std::env::var("FROG_API_KEY").unwrap_or_else(|_| {
        eprintln!("ERROR: FROG_API_KEY environment variable is not set.");
        std::process::exit(1);
    });

    let frog_base_url =
        std::env::var("FROG_BASE_URL").unwrap_or_else(|_| "https://frogapi.app/v1".to_string());

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let state = Arc::new(AppState {
        frog_base_url: frog_base_url.clone(),
        frog_api_key,
        client: Client::new(),
    });

    // Build the Axum router.
    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state);

    let addr = format!("{host}:{port}");
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("ERROR: Cannot bind to {addr}: {e}");
            std::process::exit(1);
        }
    };

    info!("frog-api-wrapper listening on http://{addr}  →  upstream: {frog_base_url}");

    axum::serve(listener, app).await.expect("server error");
}
