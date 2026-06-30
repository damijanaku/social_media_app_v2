use crate::AppState;
use crate::models::post_model::Post;
use crate::models::user_model::{LoginUser, RegisterUser, UpdateUser, User, UserResponse};
use crate::utils::cache;
use crate::utils::jwt::{Claims, verify_auth_token};
use crate::utils::pagination::PaginationParams;
use axum::extract::{Path, Query, State};
use axum::{Json, http::StatusCode};
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use bcrypt::{hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde_json::{Value, json};
use uuid::Uuid;

pub async fn register_user(
    State(state): State<AppState>,
    Json(payload): Json<RegisterUser>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let pool = &state.db;

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
            .fetch_one(pool)
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
            .fetch_one(pool)
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
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(UserResponse::from(user)))
}

pub async fn login_user(
    State(state): State<AppState>,
    Json(payload): Json<LoginUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.db;

    if payload.username.is_empty() || payload.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Username and password are required".to_string(),
        ));
    }

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_optional(pool)
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

pub async fn my_profile(
    Query(pagination): Query<PaginationParams>,
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pool = &state.db;
    let claims = verify_auth_token(TypedHeader(auth)).await?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| StatusCode::BAD_REQUEST)?;

    let limit = pagination.limit.unwrap_or(10);
    let page = pagination.page.unwrap_or(1);

    let cache_key = format!("user:profile:{}:p{}:l{}", user_id, page, limit);
    if let Ok(Some(cached_data)) = state.cache.get::<serde_json::Value>(&cache_key).await {
        return Ok(Json(cached_data));
    }

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

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
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total_posts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = json!({
        "user": UserResponse::from(user),
        "posts": posts,
        "meta": {
            "total_items": total_posts.0,
            "page": page,
            "limit": limit
        }
    });

    if let Err(e) = state
        .cache
        .set(
            &cache_key,
            &response,
            Some(Duration::seconds(60).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache profile: {}", e);
    }

    Ok(Json(response))
}

pub async fn profile(
    Path(target_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pool = &state.db;
    verify_auth_token(TypedHeader(auth)).await?;

    let limit = pagination.limit.unwrap_or(10);
    let page = pagination.page.unwrap_or(1);

    let cache_key = format!("user:profile:{}:p{}:l{}", target_id, page, limit);
    if let Ok(Some(cached_data)) = state.cache.get::<serde_json::Value>(&cache_key).await {
        return Ok(Json(cached_data));
    }

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(target_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

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
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total_posts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE user_id = $1")
        .bind(target_id)
        .fetch_one(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = json!({
        "user": UserResponse::from(user),
        "posts": posts,
        "meta": {
            "total_items": total_posts.0,
            "page": page,
            "limit": limit
        }
    });

    if let Err(e) = state
        .cache
        .set(
            &cache_key,
            &response,
            Some(Duration::seconds(60).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache profile: {}", e);
    }

    Ok(Json(response))
}

pub async fn get_user(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(target_id): Path<Uuid>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let pool = &state.db;
    verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized access".to_string()))?;

    let cache_key = format!("user:{}", target_id);
    if let Some(cached_user) = state
        .cache
        .get::<UserResponse>(&cache_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        return Ok(Json(cached_user));
    }

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(target_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let response = UserResponse::from(user);

    if let Err(e) = state
        .cache
        .set(
            &cache_key,
            &response,
            Some(Duration::seconds(300).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache user: {}", e);
    }

    Ok(Json(response))
}

pub async fn get_user_by_username(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(username): Path<String>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let pool = &state.db;
    verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized access".to_string()))?;

    let cache_key = format!("user:username:{}", username);
    if let Some(cached_user) = state
        .cache
        .get::<UserResponse>(&cache_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        return Ok(Json(cached_user));
    }

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&username)
        .fetch_optional(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let response = UserResponse::from(user);

    if let Err(e) = state
        .cache
        .set(
            &cache_key,
            &response,
            Some(Duration::seconds(300).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache user by username: {}", e);
    }

    Ok(Json(response))
}
pub async fn update_user(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.db;
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Unauthorized access".to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    if let Some(ref username) = payload.username {
        if username.len() < 5 {
            return Err((
                StatusCode::BAD_REQUEST,
                "Username must be at least 5 characters long".to_string(),
            ));
        }
    }
    if let Some(ref password) = payload.password {
        if password.len() < 8 {
            return Err((
                StatusCode::BAD_REQUEST,
                "Password must be at least 8 characters long".to_string(),
            ));
        }
    }

    if payload.username.is_none()
        && payload.email.is_none()
        && payload.name.is_none()
        && payload.password.is_none()
        && payload.birthday.is_none()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "No fields provided for update".to_string(),
        ));
    }

    let hashed_password = if let Some(ref pw) = payload.password {
        Some(hash(pw, 12).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?)
    } else {
        None
    };

    let updated_user = sqlx::query_as::<_, User>(
        "UPDATE users SET
            username     = COALESCE($1, username),
            email        = COALESCE($2, email),
            name         = COALESCE($3, name),
            password_hash = COALESCE($4, password_hash),
            birthday     = COALESCE($5, birthday)
         WHERE id = $6
         RETURNING *",
    )
    .bind(payload.username.as_deref())
    .bind(payload.email.as_deref())
    .bind(payload.name.as_deref())
    .bind(hashed_password.as_deref())
    .bind(payload.birthday)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") {
            (
                StatusCode::CONFLICT,
                "Username or email already exists".to_string(),
            )
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, msg)
        }
    })?
    .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let cache_key = format!("user:{}", user_id);
    if let Err(e) = state.cache.delete(&cache_key).await {
        eprintln!("Failed to invalidate user cache: {}", e);
    }

    if let Some(username) = &payload.username {
        let username_key = format!("user:username:{}", username);
        if let Err(e) = state.cache.delete(&username_key).await {
            eprintln!("Failed to invalidate username cache: {}", e);
        }
    }

    if let Err(e) = state
        .cache
        .invalidate_pattern(&format!("user:profile:{}:*", user_id))
        .await
    {
        eprintln!("Failed to invalidate profile cache: {}", e);
    }

    Ok(Json(json!({
        "message": "User updated successfully",
        "user": UserResponse::from(updated_user)
    })))
}

pub async fn delete_user(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.db;
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Unauthorized access".to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    let _ = state
        .cache
        .invalidate_pattern(&format!("user:{}", user_id))
        .await;
    let _ = state
        .cache
        .invalidate_pattern(&format!("user:profile:{}:*", user_id))
        .await;
    let _ = state
        .cache
        .invalidate_pattern(&format!("user:posts:{}:*", user_id))
        .await;
    let _ = state
        .cache
        .invalidate_pattern(&format!("followers:{}:*", user_id))
        .await;
    let _ = state
        .cache
        .invalidate_pattern(&format!("following:{}:*", user_id))
        .await;
    let _ = state
        .cache
        .invalidate_pattern(&format!("feed:{}:*", user_id))
        .await;
    let _ = state
        .cache
        .delete(&crate::utils::cache::keys::follower_count(user_id))
        .await;
    let _ = state
        .cache
        .delete(&crate::utils::cache::keys::following_count(user_id))
        .await;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("DELETE FROM likes WHERE post_id IN (SELECT id FROM posts WHERE user_id = $1)")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("DELETE FROM comments WHERE post_id IN (SELECT id FROM posts WHERE user_id = $1)")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("DELETE FROM posts WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("DELETE FROM comments WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("DELETE FROM likes WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("DELETE FROM follows WHERE follower_id = $1 OR followed_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        json!({ "message": "User and all associated data deleted successfully" }),
    ))
}

pub async fn follow_user(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(target_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let pool = &state.db;
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized access".to_string()))?;

    let follower_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    if follower_id == target_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "You cannot follow yourself".to_string(),
        ));
    }

    let target_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
            .bind(target_id)
            .fetch_one(pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !target_exists {
        return Err((StatusCode::NOT_FOUND, "User not found".to_string()));
    }

    let already_following = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM follows WHERE follower_id = $1 AND followed_id = $2)",
    )
    .bind(follower_id)
    .bind(target_id)
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if already_following {
        return Err((
            StatusCode::CONFLICT,
            "Already following this user".to_string(),
        ));
    }

    sqlx::query(
        "INSERT INTO follows (id, follower_id, followed_id, created_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(follower_id)
    .bind(target_id)
    .bind(Utc::now())
    .execute(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _ = state
        .cache
        .invalidate_pattern(&format!("followers:{}:*", target_id))
        .await;
    let _ = state
        .cache
        .invalidate_pattern(&format!("following:{}:*", follower_id))
        .await;
    let _ = state
        .cache
        .delete(&crate::utils::cache::keys::follower_count(target_id))
        .await;
    let _ = state
        .cache
        .delete(&crate::utils::cache::keys::following_count(follower_id))
        .await;

    let _ = state
        .cache
        .invalidate_pattern(&format!("feed:{}:*", follower_id))
        .await;

    Ok(Json(json!({ "message": "User followed successfully" })))
}

pub async fn unfollow_user(
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(target_id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let pool = &state.db;
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized access".to_string()))?;

    let follower_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    if follower_id == target_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "You cannot unfollow yourself".to_string(),
        ));
    }

    let result = sqlx::query("DELETE FROM follows WHERE follower_id = $1 AND followed_id = $2")
        .bind(follower_id)
        .bind(target_id)
        .execute(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            "You are not following this user".to_string(),
        ));
    }

    let _ = state
        .cache
        .invalidate_pattern(&format!("followers:{}:*", target_id))
        .await;
    let _ = state
        .cache
        .invalidate_pattern(&format!("following:{}:*", follower_id))
        .await;
    let _ = state
        .cache
        .delete(&crate::utils::cache::keys::follower_count(target_id))
        .await;
    let _ = state
        .cache
        .delete(&crate::utils::cache::keys::following_count(follower_id))
        .await;
    let _ = state
        .cache
        .invalidate_pattern(&format!("feed:{}:*", follower_id))
        .await;

    Ok(Json(json!({ "message": "User unfollowed successfully" })))
}

pub async fn get_followers(
    Path(target_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.db;
    verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized access".to_string()))?;

    let limit = pagination.limit.unwrap_or(10);
    let page = pagination.page.unwrap_or(1);

    let cache_key = crate::utils::cache::keys::followers(target_id, page as u32, limit as u32);
    if let Some(cached_data) = state
        .cache
        .get::<serde_json::Value>(&cache_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        return Ok(Json(cached_data));
    }

    let offset = (page - 1) * limit;

    let followers = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u
         INNER JOIN follows f ON f.follower_id = u.id
         WHERE f.followed_id = $1
         LIMIT $2 OFFSET $3",
    )
    .bind(target_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .into_iter()
    .map(UserResponse::from)
    .collect::<Vec<UserResponse>>();

    let total_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM follows WHERE followed_id = $1")
        .bind(target_id)
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response = json!({
        "followers": followers,
        "meta": {
            "total_items": total_count.0,
            "page": page,
            "limit": limit
        }
    });

    if let Err(e) = state
        .cache
        .set(
            &cache_key,
            &response,
            Some(Duration::seconds(60).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache followers: {}", e);
    }

    Ok(Json(response))
}

pub async fn get_following(
    Path(target_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.db;
    verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized access".to_string()))?;

    let limit = pagination.limit.unwrap_or(10);
    let page = pagination.page.unwrap_or(1);

    let cache_key = crate::utils::cache::keys::following(target_id, page as u32, limit as u32);
    if let Some(cached_data) = state
        .cache
        .get::<serde_json::Value>(&cache_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        return Ok(Json(cached_data));
    }

    let offset = (page - 1) * limit;

    let following = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u
         INNER JOIN follows f ON f.followed_id = u.id
         WHERE f.follower_id = $1
         LIMIT $2 OFFSET $3",
    )
    .bind(target_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .into_iter()
    .map(UserResponse::from)
    .collect::<Vec<UserResponse>>();

    let total_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM follows WHERE follower_id = $1")
        .bind(target_id)
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response = json!({
        "following": following,
        "meta": {
            "total_items": total_count.0,
            "page": page,
            "limit": limit
        }
    });

    if let Err(e) = state
        .cache
        .set(
            &cache_key,
            &response,
            Some(Duration::seconds(60).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache following: {}", e);
    }

    Ok(Json(response))
}

pub async fn get_my_followers(
    Query(pagination): Query<PaginationParams>,
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.db;
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized access".to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    let limit = pagination.limit.unwrap_or(10);
    let page = pagination.page.unwrap_or(1);

    let cache_key = crate::utils::cache::keys::followers(user_id, page as u32, limit as u32);
    if let Some(cached_data) = state
        .cache
        .get::<serde_json::Value>(&cache_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        return Ok(Json(cached_data));
    }

    let target_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let offset = (page - 1) * limit;

    let followers: Vec<UserResponse> = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u
         INNER JOIN follows f ON f.follower_id = u.id
         WHERE f.followed_id = $1
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .into_iter()
    .map(UserResponse::from)
    .collect();

    let total_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM follows WHERE followed_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response = json!({
        "user": { "id": target_user.id, "username": target_user.username },
        "followers": followers,
        "meta": {
            "total_items": total_count.0,
            "page": page,
            "limit": limit
        }
    });

    if let Err(e) = state
        .cache
        .set(
            &cache_key,
            &response,
            Some(Duration::seconds(60).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache followers: {}", e);
    }

    let count_cache_key = crate::utils::cache::keys::follower_count(user_id);
    let count_response = json!({ "count": total_count.0 });
    if let Err(e) = state
        .cache
        .set(
            &count_cache_key,
            &count_response,
            Some(Duration::seconds(60).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache follower count: {}", e);
    }

    Ok(Json(response))
}

pub async fn get_my_following(
    Query(pagination): Query<PaginationParams>,
    State(state): State<AppState>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let pool = &state.db;
    let claims = verify_auth_token(TypedHeader(auth))
        .await
        .map_err(|status| (status, "Unauthorized access".to_string()))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    let limit = pagination.limit.unwrap_or(10);
    let page = pagination.page.unwrap_or(1);

    let cache_key = crate::utils::cache::keys::following(user_id, page as u32, limit as u32);
    if let Some(cached_data) = state
        .cache
        .get::<serde_json::Value>(&cache_key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    {
        return Ok(Json(cached_data));
    }

    let target_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "User not found".to_string()))?;

    let offset = (page - 1) * limit;

    let following: Vec<UserResponse> = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u
         INNER JOIN follows f ON f.followed_id = u.id
         WHERE f.follower_id = $1
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .into_iter()
    .map(UserResponse::from)
    .collect();

    let total_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM follows WHERE follower_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response = json!({
        "user": { "id": target_user.id, "username": target_user.username },
        "following": following,
        "meta": {
            "total_items": total_count.0,
            "page": page,
            "limit": limit
        }
    });

    if let Err(e) = state
        .cache
        .set(
            &cache_key,
            &response,
            Some(Duration::seconds(60).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache following: {}", e);
    }

    let count_cache_key = crate::utils::cache::keys::following_count(user_id);
    let count_response = json!({ "count": total_count.0 });
    if let Err(e) = state
        .cache
        .set(
            &count_cache_key,
            &count_response,
            Some(Duration::seconds(60).to_std().unwrap()),
        )
        .await
    {
        eprintln!("Failed to cache following count: {}", e);
    }

    Ok(Json(response))
}
