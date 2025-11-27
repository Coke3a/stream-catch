use super::*;
use jsonwebtoken::{encode, EncodingKey, Header};
use std::env;

fn set_env_vars() {
    unsafe {
        env::set_var("SERVER_PORT", "8080");
        env::set_var("SERVER_BODY_LIMIT", "10");
        env::set_var("SERVER_TIMEOUT", "30");
        env::set_var("DATABASE_URL", "postgres://localhost:5432/db");
        env::set_var("SUPABASE_PROJECT_URL", "https://example.supabase.co");
        env::set_var("SUPABASE_JWT_SECRET", "supersecretjwtsecretforunittesting123");
    }
}

#[test]
fn test_validate_supabase_jwt_success() {
    set_env_vars();
    let secret = "supersecretjwtsecretforunittesting123";
    let my_claims = SupabaseClaims {
        sub: "123e4567-e89b-12d3-a456-426614174000".to_string(),
        role: "authenticated".to_string(),
        email: Some("test@example.com".to_string()),
        exp: 9999999999, // far future
    };

    let token = encode(
        &Header::default(),
        &my_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    let claims = validate_supabase_jwt(&token).expect("Valid token should pass");
    assert_eq!(claims.sub, my_claims.sub);
    assert_eq!(claims.email, my_claims.email);
}

#[test]
fn test_validate_supabase_jwt_expired() {
    set_env_vars();
    let secret = "supersecretjwtsecretforunittesting123";
    let my_claims = SupabaseClaims {
        sub: "123e4567-e89b-12d3-a456-426614174000".to_string(),
        role: "authenticated".to_string(),
        email: Some("test@example.com".to_string()),
        exp: 1, // past
    };

    let token = encode(
        &Header::default(),
        &my_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    let result = validate_supabase_jwt(&token);
    assert!(result.is_err());
}

#[test]
fn test_validate_supabase_jwt_invalid_signature() {
    set_env_vars();
    let secret = "wrongsecret";
    let my_claims = SupabaseClaims {
        sub: "123e4567-e89b-12d3-a456-426614174000".to_string(),
        role: "authenticated".to_string(),
        email: Some("test@example.com".to_string()),
        exp: 9999999999,
    };

    let token = encode(
        &Header::default(),
        &my_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    let result = validate_supabase_jwt(&token);
    assert!(result.is_err());
}
