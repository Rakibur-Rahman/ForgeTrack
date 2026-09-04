use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("token error: {0}")]
    Token(#[from] jsonwebtoken::errors::Error),
    #[error("password error")]
    Password,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    Access,
    Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    pub kind: TokenKind,
}

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthError::Password)
}

pub fn verify_password(password: &str, hash: &str) -> Result<(), AuthError> {
    let hash = PasswordHash::new(hash).map_err(|_| AuthError::InvalidCredentials)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .map_err(|_| AuthError::InvalidCredentials)
}

pub fn issue_token(user_id: Uuid, secret: &str, ttl_hours: i64) -> Result<String, AuthError> {
    issue_typed_token(user_id, secret, ttl_hours, TokenKind::Access)
}

pub fn issue_typed_token(
    user_id: Uuid,
    secret: &str,
    ttl_hours: i64,
    kind: TokenKind,
) -> Result<String, AuthError> {
    let now = Utc::now();
    encode(
        &Header::default(),
        &Claims {
            sub: user_id,
            iat: now.timestamp(),
            exp: (now + Duration::hours(ttl_hours)).timestamp(),
            kind,
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(AuthError::from)
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AuthError> {
    Ok(decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?
    .claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn password_round_trip() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(verify_password("correct horse battery staple", &hash).is_ok());
        assert!(verify_password("wrong", &hash).is_err());
    }
    #[test]
    fn token_valid_and_tampered() {
        let token = issue_token(Uuid::new_v4(), "secret", 1).unwrap();
        assert!(verify_token(&token, "secret").is_ok());
        assert!(verify_token(&(token + "x"), "secret").is_err());
    }
    #[test]
    fn token_expired() {
        let token = issue_token(Uuid::new_v4(), "secret", -1).unwrap();
        assert!(verify_token(&token, "secret").is_err());
    }
    #[test]
    fn token_kind_is_preserved() {
        let token = issue_typed_token(Uuid::new_v4(), "secret", 24, TokenKind::Refresh).unwrap();
        assert_eq!(
            verify_token(&token, "secret").unwrap().kind,
            TokenKind::Refresh
        );
    }
}
