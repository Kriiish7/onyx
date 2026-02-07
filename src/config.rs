use crate::error::{OnyxError, OnyxResult};
use config::{Config, Environment, File};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub payments: PaymentsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaymentsConfig {
    pub provider: Option<String>,
    pub stripe_api_key: String,
    pub stripe_webhook_secret: String,
    pub default_price_id: String,
    pub success_url: String,
    pub cancel_url: String,
    pub portal_return_url: String,
}

pub fn load_config(path: Option<&Path>) -> OnyxResult<AppConfig> {
    let mut builder = Config::builder()
        .add_source(File::with_name("config").required(false))
        .add_source(Environment::with_prefix("ONYX").separator("__"));

    if let Some(path) = path {
        builder = builder.add_source(File::from(path).required(false));
    }

    let config = builder
        .build()
        .map_err(|err| OnyxError::ConfigError(err.to_string()))?;

    let parsed: AppConfig = config
        .try_deserialize()
        .map_err(|err| OnyxError::ConfigError(err.to_string()))?;

    if let Some(provider) = &parsed.payments.provider {
        if provider.to_lowercase() != "stripe" {
            return Err(OnyxError::ConfigError(format!(
                "unsupported payments.provider '{}'; expected 'stripe'",
                provider
            )));
        }
    }

    Ok(parsed)
}
