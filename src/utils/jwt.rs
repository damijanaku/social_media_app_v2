use axum::{Json, http::StatusCode};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub id: String,
    pub sub: String,
    pub exp: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: String,
    pub exp: usize,
}

// verify token
pub async fn verify_auth_token(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Claims, StatusCode> {
    // bearer token
    let token = auth.token();

    // load secret-from-env
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "mysecret".into());

    let token_data = decode(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(token_data.claims)
}

pub async fn refresh_token(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let token = auth.token();
    let refresh_secret =
        std::env::var("JWT_REFRESH_SECRET").unwrap_or_else(|_| "myrefreshsecret".into());

    let token_data = decode::<RefreshClaims>(
        token,
        &DecodingKey::from_secret(refresh_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid or expired refresh token".to_string(),
        )
    })?;

    //new access token
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "mysecret".into());
    let exp = Utc::now() + Duration::minutes(60);
    let new_claims = Claims {
        sub: token_data.claims.sub.clone(),
        id: token_data.claims.sub,
        exp: exp.timestamp() as usize,
    };

    let new_access_token = encode(
        &Header::default(),
        &new_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "access_token": new_access_token })))
}
