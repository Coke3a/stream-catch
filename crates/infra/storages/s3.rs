use std::time::Duration;

use anyhow::{Context, Result};
use aws_config::{BehaviorVersion, timeout::TimeoutConfig};
use aws_credential_types::Credentials;
use aws_sdk_s3::{
    Client,
    config::{Region, StalledStreamProtectionConfig},
    error::SdkError,
};
use http::Uri;
use std::error::Error as StdError;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct S3Config {
    pub endpoint: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub force_path_style: bool,
    pub connect_timeout_secs: u64,
    pub read_timeout_secs: u64,
}

impl S3Config {
    pub fn new(endpoint: String, region: String, access_key: String, secret_key: String) -> Self {
        Self {
            endpoint,
            region,
            access_key,
            secret_key,
            force_path_style: true,
            connect_timeout_secs: 10,
            read_timeout_secs: 60,
        }
    }
}

#[derive(Debug)]
pub struct StorageUploadError {
    retryable: bool,
    message: String,
    source: Option<anyhow::Error>,
}

impl StorageUploadError {
    pub fn retryable(message: impl Into<String>) -> anyhow::Error {
        anyhow::Error::new(Self {
            retryable: true,
            message: message.into(),
            source: None,
        })
    }

    pub fn retryable_with_source(message: impl Into<String>, source: anyhow::Error) -> anyhow::Error {
        anyhow::Error::new(Self {
            retryable: true,
            message: message.into(),
            source: Some(source),
        })
    }

    pub fn non_retryable(message: impl Into<String>) -> anyhow::Error {
        anyhow::Error::new(Self {
            retryable: false,
            message: message.into(),
            source: None,
        })
    }

    pub fn non_retryable_with_source(
        message: impl Into<String>,
        source: anyhow::Error,
    ) -> anyhow::Error {
        anyhow::Error::new(Self {
            retryable: false,
            message: message.into(),
            source: Some(source),
        })
    }

    pub fn is_retryable(&self) -> bool {
        self.retryable
    }
}

impl std::fmt::Display for StorageUploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl StdError for StorageUploadError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_ref().map(|err| err.as_ref())
    }
}

pub fn is_retryable_s3_error<E>(err: &SdkError<E>) -> bool {
    match err {
        SdkError::TimeoutError(_) => true,
        SdkError::DispatchFailure(_) => true,
        SdkError::ResponseError(_) => true,
        SdkError::ServiceError(service_err) => {
            let status = service_err.raw().status().as_u16();
            matches!(status, 408 | 429) || (500..=599).contains(&status)
        }
        SdkError::ConstructionFailure(_) => false,
        _ => false,
    }
}

pub async fn build_s3_client(config: &S3Config) -> Result<Client> {
    let endpoint = format!("{}/", config.endpoint.trim_end_matches('/'));
    Uri::from_str(&endpoint).context("invalid s3 endpoint URL")?;

    let credentials = Credentials::new(
        config.access_key.clone(),
        config.secret_key.clone(),
        None,
        None,
        "s3-compatible",
    );

    let region = Region::new(config.region.clone());
    let shared_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region.clone())
        .credentials_provider(credentials)
        .timeout_config(
            TimeoutConfig::builder()
                .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
                .read_timeout(Duration::from_secs(config.read_timeout_secs))
                .build(),
        )
        .load()
        .await;

    let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
        .endpoint_url(endpoint)
        .force_path_style(config.force_path_style)
        .region(region)
        .stalled_stream_protection(StalledStreamProtectionConfig::disabled())
        .build();

    Ok(Client::from_conf(s3_config))
}
