use std::time::Duration;

use axum::http::HeaderName;
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use hyper::Request;
use serde_json::json;
use tower_http::compression::CompressionLayer;
use tower_http::propagate_header::PropagateHeaderLayer;
use tower_http::request_id::{MakeRequestUuid, SetRequestIdLayer};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::{Level, Span};

use super::handlers::health_check_handler;
use crate::registry::registar::ServiceRegistry;
use crate::servers::health_check_api::HEALTH_CHECK_API_LOG_TARGET;

pub fn router(register: ServiceRegistry) -> Router {
    Router::new()
        .route("/", get(|| async { Json(json!({})) }))
        .route("/health_check", get(health_check_handler))
        .with_state(register)
        .layer(CompressionLayer::new())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateHeaderLayer::new(HeaderName::from_static("x-request-id")))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(|request: &Request<axum::body::Body>, _span: &Span| {
                    let method = request.method().to_string();
                    let uri = request.uri().to_string();
                    let request_id = request
                        .headers()
                        .get("x-request-id")
                        .map(|v| v.to_str().unwrap_or_default())
                        .unwrap_or_default();

                    tracing::span!(
                target: HEALTH_CHECK_API_LOG_TARGET,
                tracing::Level::INFO, "request", method = %method, uri = %uri, request_id = %request_id);
                })
                .on_response(|response: &Response, latency: Duration, _span: &Span| {
                    let status_code = response.status();
                    let request_id = response
                        .headers()
                        .get("x-request-id")
                        .map(|v| v.to_str().unwrap_or_default())
                        .unwrap_or_default();
                    let latency_ms = latency.as_millis();

                    tracing::span!(
                target: HEALTH_CHECK_API_LOG_TARGET,
                tracing::Level::INFO, "response", latency = %latency_ms, status = %status_code, request_id = %request_id);
                }),
        )
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
}
