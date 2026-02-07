use axum::{routing::{get, post}, Router};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

use crate::config::{AppConfig, PaymentsConfig};
use crate::error::{OnyxError, OnyxResult};

pub mod payments;

#[derive(Clone)]
pub struct AppState {
    pub stripe: stripe::Client,
    pub payments: PaymentsConfig,
}

pub async fn run_http_server(config: AppConfig) -> OnyxResult<()> {
    let stripe_client = stripe::Client::new(config.payments.stripe_api_key.clone());
    let state = AppState {
        stripe: stripe_client,
        payments: config.payments,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/billing/checkout", post(payments::create_checkout_session))
        .route("/billing/portal", post(payments::create_billing_portal_session))
        .route("/billing/webhook", post(payments::stripe_webhook))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .map_err(|err| OnyxError::Internal(format!("invalid server address: {err}")))?;

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|err| OnyxError::Internal(format!("failed to bind server: {err}")))?;

    axum::serve(listener, app)
        .await
        .map_err(|err| OnyxError::Internal(format!("server error: {err}")))?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}
