use std::time::Duration;

use anyhow::{Context, Result};
use aws_config::{BehaviorVersion, timeout::TimeoutConfig};
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client, config::Region};
use http::Uri;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct S3Config {
    pub endpoint: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub force_path_style: bool,
    pub connect_timeout_secs: u64,
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
        }
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
                .build(),
        )
        .load()
        .await;

    let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
        .endpoint_url(endpoint)
        .force_path_style(config.force_path_style)
        .region(region)
        .build();

    Ok(Client::from_conf(s3_config))
}
