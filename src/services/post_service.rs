use crate::models::post_model::{CreatePostInput, Post, UpdatePost};
use crate::utils::jwt::verify_auth_token;
use crate::utils::pagination::{self, PaginationParams};
use axum::extract::{Path, Query};
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_post(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    State(pool): State<PgPool>,
    Json(body): Json<CreatePostInput>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if body.title.is_empty() || body.body.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Title and body are required".to_string(),
        ));
    }

    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized access".to_string()))?;

    let user_id = Uuid::parse_str(&claims.id).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid user ID format".to_string(),
        )
    })?;

    let post = sqlx::query_as::<_, Post>(
        "INSERT INTO posts (id, title, body, user_id, likes_count, created_at)
         VALUES ($1, $2, $3, $4, 0, $5)
         RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(&body.title)
    .bind(&body.body)
    .bind(user_id)
    .bind(Utc::now())
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "post": post })))
}

pub async fn update_post(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    State(pool): State<PgPool>,
    Path(target_id): Path<Uuid>,
    Json(payload): Json<UpdatePost>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let user_id = Uuid::parse_str(&claims.id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    if payload.title.is_none() && payload.body.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "No fields provided for update".to_string(),
        ));
    }

    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1")
        .bind(target_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Post not found".to_string()))?;

    if post.user_id != user_id {
        return Err((
            StatusCode::FORBIDDEN,
            "Unauthorized to update this post".to_string(),
        ));
    }

    let updated_post = sqlx::query_as::<_, Post>(
        "UPDATE posts SET
            title = COALESCE($1, title),
            body  = COALESCE($2, body)
         WHERE id = $3
         RETURNING *",
    )
    .bind(payload.title.as_deref())
    .bind(payload.body.as_deref())
    .bind(target_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "post": updated_post })))
}

pub async fn delete_post(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    State(pool): State<PgPool>,
    Path(target_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1")
        .bind(target_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Post not found".to_string()))?;

    if post.user_id.to_string() != claims.sub {
        return Err((
            StatusCode::FORBIDDEN,
            "Unauthorized to delete this post".to_string(),
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let comments_deleted = sqlx::query("DELETE FROM comments WHERE post_id = $1")
        .bind(target_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .rows_affected();

    let likes_deleted = sqlx::query("DELETE FROM likes WHERE post_id = $1")
        .bind(target_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .rows_affected();

    sqlx::query("DELETE FROM posts WHERE id = $1")
        .bind(target_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "message": "Post deleted successfully",
        "details": {
            "commentsDeleted": comments_deleted,
            "likesDeleted": likes_deleted
        }
    })))
}

pub async fn get_my_posts(
    Query(pagination): Query<PaginationParams>,
    State(pool): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid user ID in token".to_string(),
        )
    })?;

    let limit = pagination.limit.unwrap_or(10);
    let page = pagination.page.unwrap_or(1);
    let offset = (page - 1) * limit;

    let posts = sqlx::query_as::<_, Post>(
        "SELECT * FROM posts 
        WHERE user_id = $1 
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "posts": posts,
        "meta": {
            "total_items": total_count.0,
            "page": page,
            "limit": limit
        }
    })))
}

pub async fn get_posts_from_user(
    Path(target_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
    State(pool): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let limit = pagination.limit.unwrap_or(10);
    let page = pagination.page.unwrap_or(1);
    let offset = (page - 1) * limit;

    let posts = sqlx::query_as::<_, Post>(
        "SELECT * FROM posts 
         WHERE user_id = $1 
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(target_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE user_id = $1")
        .bind(target_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "posts": posts,
        "meta": {
            "total_items": total_count.0,
            "page": page,
            "limit": limit
        }
    })))
}

pub async fn get_post_by_id(
    Path(target_id): Path<Uuid>,
    State(pool): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Post>, (StatusCode, String)> {
    verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1")
        .bind(target_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Post not found".to_string()))?;

    Ok(Json(post))
}

pub async fn get_feed(
    Query(pagination): Query<PaginationParams>,
    State(pool): State<PgPool>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    let limit = pagination.limit.unwrap_or(10);
    let page = pagination.page.unwrap_or(1);
    let offset = (page - 1) * limit;

    let posts = sqlx::query_as::<_, Post>(
        "SELECT p.* FROM posts p
         INNER JOIN follows f ON f.followed_id = p.user_id
         WHERE f.follower_id = $1
         ORDER BY p.created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM posts p
         INNER JOIN follows f ON f.followed_id = p.user_id
         WHERE f.follower_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "posts": posts,
        "meta": {
            "total_items": total_count.0,
            "page": page,
            "limit": limit
        }
    })))
}
