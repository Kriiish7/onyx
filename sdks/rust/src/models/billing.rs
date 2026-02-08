//! Billing models — Stripe checkout and portal integration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request body for creating a Stripe checkout session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutSessionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl CheckoutSessionRequest {
    /// Create a new checkout session request.
    pub fn new() -> Self {
        Self {
            customer_email: None,
            customer_id: None,
            price_id: None,
            success_url: None,
            cancel_url: None,
            reference_id: None,
            metadata: None,
        }
    }

    pub fn customer_email(mut self, email: impl Into<String>) -> Self {
        self.customer_email = Some(email.into());
        self
    }

    pub fn customer_id(mut self, id: impl Into<String>) -> Self {
        self.customer_id = Some(id.into());
        self
    }

    pub fn price_id(mut self, id: impl Into<String>) -> Self {
        self.price_id = Some(id.into());
        self
    }

    pub fn success_url(mut self, url: impl Into<String>) -> Self {
        self.success_url = Some(url.into());
        self
    }

    pub fn cancel_url(mut self, url: impl Into<String>) -> Self {
        self.cancel_url = Some(url.into());
        self
    }

    pub fn reference_id(mut self, id: impl Into<String>) -> Self {
        self.reference_id = Some(id.into());
        self
    }

    pub fn metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl Default for CheckoutSessionRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response from creating a checkout session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutSessionResponse {
    pub id: String,
    pub url: String,
}

/// Request body for creating a billing portal session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingPortalRequest {
    pub customer_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,
}

impl BillingPortalRequest {
    pub fn new(customer_id: impl Into<String>) -> Self {
        Self {
            customer_id: customer_id.into(),
            return_url: None,
        }
    }

    pub fn return_url(mut self, url: impl Into<String>) -> Self {
        self.return_url = Some(url.into());
        self
    }
}

/// Response from creating a billing portal session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingPortalResponse {
    pub url: String,
}
