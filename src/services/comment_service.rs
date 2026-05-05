use crate::models::comment_model::{Comment, PostComment};
use crate::utils::jwt::verify_auth_token;
use axum::extract::Path;
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn post_comment(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    State(pool): State<PgPool>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<PostComment>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
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
            .fetch_one(&pool)
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
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({ "message": "Comment posted successfully" })))
}

pub async fn get_comments(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    State(pool): State<PgPool>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let comments = sqlx::query_as::<_, Comment>(
        "SELECT * FROM comments WHERE post_id = $1 ORDER BY created_at DESC",
    )
    .bind(post_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let count = comments.len();

    Ok(Json(json!({
        "comments": comments,
        "count": count
    })))
}

pub async fn delete_comment(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    State(pool): State<PgPool>,
    Path((_post_id, comment_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized".to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    let result = sqlx::query("DELETE FROM comments WHERE id = $1 AND user_id = $2")
        .bind(comment_id)
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            "Comment not found or you don't have permission to delete it".to_string(),
        ));
    }

    Ok(Json(json!({ "message": "Comment deleted successfully" })))
}
