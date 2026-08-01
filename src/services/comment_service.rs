use crate::AppState;
use crate::models::comment_model::{Comment, PostComment};
use crate::utils::jwt::verify_auth_token;
use crate::utils::pagination::{self, PaginationParams};
use axum::extract::{Path, Query};
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn post_comment(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<PostComment>,
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

    let post_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM posts WHERE id = $1)")
            .bind(post_id)
            .fetch_one(pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !post_exists {
        return Err((StatusCode::NOT_FOUND, "Post not found".to_string()));
    }

    sqlx::query(
        "INSERT INTO comments (id, post_id, user_id, body, created_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(post_id)
    .bind(user_id)
    .bind(&payload.body)
    .bind(Utc::now())
    .execute(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let comments_pattern = format!("comments:{}:*", post_id);
    let post_cache_key = crate::utils::cache::keys::post(post_id);

    let (comments_result, post_result) = tokio::join!(
        state.cache.invalidate_pattern(&comments_pattern),
        state.cache.delete(&post_cache_key),
    );

    if let Err(e) = comments_result {
        eprintln!(
            "Failed to invalidate comments cache for post {}: {}",
            post_id, e
        );
    }
    if let Err(e) = post_result {
        eprintln!("Failed to delete post cache for {}: {}", post_id, e);
    }

    Ok(Json(json!({ "message": "Comment posted successfully" })))
}

pub async fn get_comments(
    Path(post_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.db;
    verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let limit = pagination.limit.unwrap_or(10);
    let page = pagination.page.unwrap_or(1);

    let cache_key = crate::utils::cache::keys::post_comments(post_id, page as u32, limit as u32);
    if let Some(cached_data) = state
        .cache
        .get::<serde_json::Value>(&cache_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        return Ok(Json(cached_data));
    }

    let offset = (page - 1) * limit;

    let comments = sqlx::query_as::<_, Comment>(
        "SELECT * FROM comments 
         WHERE post_id = $1 
         ORDER BY created_at DESC 
         LIMIT $2 OFFSET $3",
    )
    .bind(post_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE post_id = $1")
        .bind(post_id)
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response = json!({
        "comments": comments,
        "meta": {
            "total_items": total_count,
            "page": page,
            "limit": limit
        }
    });

    if let Err(e) = state
        .cache
        .set(
            &cache_key,
            &response,
            Some(Duration::seconds(120).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache comments: {}", e);
    }

    Ok(Json(response))
}

pub async fn delete_comment(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    State(state): State<AppState>,
    Path((post_id, comment_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.db;
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    let result =
        sqlx::query("DELETE FROM comments WHERE id = $1 AND user_id = $2 AND post_id = $3")
            .bind(comment_id)
            .bind(user_id)
            .bind(post_id)
            .execute(pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            "Comment not found or you don't have permission to delete it".to_string(),
        ));
    }

    let comments_pattern = format!("comments:{}:*", post_id);
    let post_cache_key = crate::utils::cache::keys::post(post_id);

    let (invalidate_result, delete_result) = tokio::join!(
        state.cache.invalidate_pattern(&comments_pattern),
        state.cache.delete(&post_cache_key),
    );

    if let Err(e) = invalidate_result {
        eprintln!(
            "Failed to invalidate comments cache for post {}: {}",
            post_id, e
        );
    }
    if let Err(e) = delete_result {
        eprintln!("Failed to delete post cache for {}: {}", post_id, e);
    }

    Ok(Json(json!({ "message": "Comment deleted successfully" })))
}
