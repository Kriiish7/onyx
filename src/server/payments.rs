use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct CheckoutSessionRequest {
    pub customer_email: Option<String>,
    pub customer_id: Option<String>,
    pub price_id: Option<String>,
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
    pub reference_id: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
pub struct CheckoutSessionResponse {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct BillingPortalRequest {
    pub customer_id: String,
    pub return_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BillingPortalResponse {
    pub url: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(ErrorResponse { error: self.message });
        (self.status, body).into_response()
    }
}

pub async fn create_checkout_session(
    State(state): State<AppState>,
    Json(request): Json<CheckoutSessionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let price_id = request
        .price_id
        .clone()
        .unwrap_or_else(|| state.payments.default_price_id.clone());
    let success_url = request
        .success_url
        .clone()
        .unwrap_or_else(|| state.payments.success_url.clone());
    let cancel_url = request
        .cancel_url
        .clone()
        .unwrap_or_else(|| state.payments.cancel_url.clone());

    let mut params = stripe::CreateCheckoutSession::new();
    params.success_url = Some(success_url.as_str());
    params.cancel_url = Some(cancel_url.as_str());
    params.mode = Some(stripe::CheckoutSessionMode::Subscription);
    params.line_items = Some(vec![stripe::CreateCheckoutSessionLineItems {
        price: Some(price_id),
        quantity: Some(1),
        ..Default::default()
    }]);
    params.customer_email = request.customer_email.as_deref();
    params.customer = request
        .customer_id
        .as_ref()
        .and_then(|id| id.parse().ok());
    params.client_reference_id = request.reference_id.as_deref();
    params.metadata = request.metadata.clone().map(|items| items.into_iter().collect());

    let session = stripe::CheckoutSession::create(&state.stripe, params)
        .await
        .map_err(|err| ApiError::internal(format!("stripe checkout error: {err}")))?;

    let url = session
        .url
        .ok_or_else(|| ApiError::internal("stripe session missing url"))?;

    Ok(Json(CheckoutSessionResponse {
        id: session.id.to_string(),
        url,
    }))
}

pub async fn create_billing_portal_session(
    State(state): State<AppState>,
    Json(request): Json<BillingPortalRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let return_url = request
        .return_url
        .clone()
        .unwrap_or_else(|| state.payments.portal_return_url.clone());

    let customer = request
        .customer_id
        .parse()
        .map_err(|_| ApiError::bad_request("invalid customer_id"))?;

    let mut params = stripe::CreateBillingPortalSession::new(customer);
    params.return_url = Some(return_url.as_str());

    let session = stripe::BillingPortalSession::create(&state.stripe, params)
        .await
        .map_err(|err| ApiError::internal(format!("stripe portal error: {err}")))?;

    Ok(Json(BillingPortalResponse { url: session.url }))
}

pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let signature = headers
        .get("stripe-signature")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::bad_request("missing Stripe-Signature header"))?;

    let payload = std::str::from_utf8(&body)
        .map_err(|_| ApiError::bad_request("invalid webhook payload encoding"))?;

    let event = stripe::Webhook::construct_event(
        payload,
        signature,
        &state.payments.stripe_webhook_secret,
    )
    .map_err(|err| ApiError::bad_request(format!("invalid webhook signature: {err}")))?;

    match event.type_ {
        stripe::EventType::CheckoutSessionCompleted
        | stripe::EventType::CustomerSubscriptionCreated
        | stripe::EventType::CustomerSubscriptionUpdated
        | stripe::EventType::CustomerSubscriptionDeleted
        | stripe::EventType::InvoicePaymentFailed => {
            println!("stripe webhook event: {:?}", event.type_);
        }
        _ => {
            println!("stripe webhook ignored event: {:?}", event.type_);
        }
    }

    Ok(StatusCode::OK)
}
