use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::config_loader;

#[derive(Debug, Serialize, Deserialize)]
pub struct SupabaseClaims {
    pub sub: String,
    pub role: String,
    pub email: Option<String>,
    pub exp: usize,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub email: Option<String>,
    pub role: String,
}

#[derive(Debug)]
pub struct AuthError(anyhow::Error);

impl From<anyhow::Error> for AuthError {
    fn from(err: anyhow::Error) -> Self {
        AuthError(err)
    }
}

// Tell axum how to convert `AuthError` into a response.
impl axum::response::IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::UNAUTHORIZED,
            format!("Unauthorized: {}", self.0),
        )
            .into_response()
    }
}

pub fn validate_supabase_jwt(token: &str) -> Result<SupabaseClaims, AuthError> {
    let config =
        config_loader::load().map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
    let secret = config.supabase.jwt_secret;

    let decoding_key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.set_audience(&["authenticated", "service_role"]);

    let token_data = decode::<SupabaseClaims>(token, &decoding_key, &validation).map_err(|e| {
        anyhow::anyhow!("JWT validation failed: {}", e)
    })?;

    Ok(token_data.claims)
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Get Authorization header
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "Missing Authorization header".to_string(),
            ))?;

        let auth_str = auth_header.to_str().map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid Authorization header".to_string(),
            )
        })?;

        // 2. Expect "Bearer <token>"
        if !auth_str.starts_with("Bearer ") {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Invalid Authorization header format".to_string(),
            ));
        }

        let token = &auth_str[7..];

        // 3. Validate JWT
        let claims = validate_supabase_jwt(token)
            .map_err(|e| (StatusCode::UNAUTHORIZED, e.0.to_string()))?;

        // 4. Parse sub to Uuid
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid user ID in token".to_string(),
            )
        })?;

        // 5. Return AuthUser
        Ok(AuthUser {
            user_id,
            email: claims.email,
            role: claims.role,
        })
    }
}

#[cfg(test)]
mod tests;
