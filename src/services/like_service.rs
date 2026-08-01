use crate::AppState;
use crate::models::like_model::Like;
use crate::utils::jwt::verify_auth_token;
use axum::extract::Path;
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn like_post(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<Like>, (StatusCode, String)> {
    let pool = &state.db;
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid user ID format".to_string(),
        )
    })?;

    let like_status_key = crate::utils::cache::keys::user_like_status(user_id, post_id);
    if let Ok(Some(true)) = state.cache.get::<bool>(&like_status_key).await {
        return Err((StatusCode::CONFLICT, "Already liked".to_string()));
    }

    let already_liked = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM likes WHERE user_id = $1 AND post_id = $2)",
    )
    .bind(user_id)
    .bind(post_id)
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if already_liked {
        let _ = state
            .cache
            .set(
                &like_status_key,
                &true,
                Some(Duration::seconds(60).to_std().unwrap()),
            )
            .await;
        return Err((StatusCode::CONFLICT, "Already liked".to_string()));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let like = sqlx::query_as::<_, Like>(
        "INSERT INTO likes (id, user_id, post_id) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(post_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("UPDATE posts SET likes_count = likes_count + 1 WHERE id = $1")
        .bind(post_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let likes_cache_key = crate::utils::cache::keys::post_likes(post_id);
    let user_liked_key = crate::utils::cache::keys::user_liked_post(user_id, post_id);
    let post_cache_key = crate::utils::cache::keys::post(post_id);

    let (likes_result, user_liked_result, post_result) = tokio::join!(
        state.cache.delete(&likes_cache_key),
        state.cache.delete(&user_liked_key),
        state.cache.delete(&post_cache_key),
    );

    if let Err(e) = likes_result {
        eprintln!("Failed to delete likes cache: {}", e);
    }
    if let Err(e) = user_liked_result {
        eprintln!("Failed to delete user liked cache: {}", e);
    }
    if let Err(e) = post_result {
        eprintln!("Failed to delete post cache: {}", e);
    }

    Ok(Json(like))
}

pub async fn unlike_post(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.db;
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid user ID format".to_string(),
        )
    })?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let result = sqlx::query("DELETE FROM likes WHERE user_id = $1 AND post_id = $2")
        .bind(user_id)
        .bind(post_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Like not found".to_string()));
    }

    sqlx::query("UPDATE posts SET likes_count = likes_count - 1 WHERE id = $1")
        .bind(post_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let likes_cache_key = crate::utils::cache::keys::post_likes(post_id);
    let user_liked_key = crate::utils::cache::keys::user_liked_post(user_id, post_id);
    let post_cache_key = crate::utils::cache::keys::post(post_id);

    let (likes_result, user_liked_result, post_result) = tokio::join!(
        state.cache.delete(&likes_cache_key),
        state.cache.delete(&user_liked_key),
        state.cache.delete(&post_cache_key),
    );

    if let Err(e) = likes_result {
        eprintln!("Failed to delete likes cache: {}", e);
    }
    if let Err(e) = user_liked_result {
        eprintln!("Failed to delete user liked cache: {}", e);
    }
    if let Err(e) = post_result {
        eprintln!("Failed to delete post cache: {}", e);
    }

    Ok(Json(json!({ "message": "Post unliked successfully" })))
}

pub async fn get_likes(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.db;
    verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let cache_key = crate::utils::cache::keys::post_likes(post_id);
    if let Some(cached_data) = state
        .cache
        .get::<serde_json::Value>(&cache_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        return Ok(Json(cached_data));
    }

    let count = sqlx::query_scalar::<_, i64>("SELECT likes_count FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Post not found".to_string()))?;

    let response = json!({ "post_id": post_id, "likes": count });

    if let Err(e) = state
        .cache
        .set(
            &cache_key,
            &response,
            Some(Duration::seconds(60).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache likes count: {}", e);
    }

    Ok(Json(response))
}

pub async fn check_user_like(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.db;
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid user ID format".to_string(),
        )
    })?;

    let cache_key = crate::utils::cache::keys::user_like_status(user_id, post_id);
    if let Some(cached_liked) = state
        .cache
        .get::<bool>(&cache_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        return Ok(Json(json!({ "post_id": post_id, "liked": cached_liked })));
    }

    let post_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM posts WHERE id = $1)")
            .bind(post_id)
            .fetch_one(pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !post_exists {
        return Err((StatusCode::NOT_FOUND, "Post not found".to_string()));
    }

    let liked = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM likes WHERE user_id = $1 AND post_id = $2)",
    )
    .bind(user_id)
    .bind(post_id)
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Err(e) = state
        .cache
        .set(
            &cache_key,
            &liked,
            Some(Duration::seconds(60).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache like status: {}", e);
    }

    Ok(Json(json!({ "post_id": post_id, "liked": liked })))
}
