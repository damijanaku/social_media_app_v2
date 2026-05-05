use crate::models::user_model::{LoginUser, RegisterUser, UpdateUser, User, UserResponse};
use crate::utils::jwt::{Claims, verify_auth_token};
use axum::extract::Path;
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use bcrypt::{hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn register_user(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterUser>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    if payload.username.is_empty() || payload.email.is_empty() || payload.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username, email, and password are required".to_string(),
        ));
    }

    if payload.username.len() < 5 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username must be at least 5 characters long".to_string(),
        ));
    }

    if payload.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    let username_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
            .bind(&payload.username)
            .fetch_one(&pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if username_exists {
        return Err((
            StatusCode::CONFLICT,
            "Username or email already exists".to_string(),
        ));
    }

    let email_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
            .bind(&payload.email)
            .fetch_one(&pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if email_exists {
        return Err((
            StatusCode::CONFLICT,
            "Username or email already exists".to_string(),
        ));
    }

    let hashed = hash(&payload.password, 12)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (id, username, email, name, password_hash, birthday, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(&payload.username)
    .bind(&payload.email)
    .bind(&payload.name)
    .bind(&hashed)
    .bind(payload.birthday)
    .bind(Utc::now())
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(UserResponse::from(user)))
}

pub async fn login_user(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username and password are required".to_string(),
        ));
    }

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_optional(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Wrong username or password".to_string(),
        ))?;

    let valid = verify(&payload.password, &user.password_hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Wrong username or password".to_string(),
        ));
    }

    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "mysecret".into());
    let access_exp = Utc::now() + Duration::hours(1);
    let claims = Claims {
        sub: user.id.to_string(),
        id: user.id.to_string(),
        exp: access_exp.timestamp() as usize,
    };

    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "message": "Login successful",
        "user": UserResponse::from(user),
        "access_token": access_token
    })))
}
