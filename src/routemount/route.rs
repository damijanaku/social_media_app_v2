use crate::services::like_service::{get_likes, like_post, unlike_post};
use crate::services::post_service::{
    create_post, delete_post, get_my_posts, get_post_by_id, get_posts_from_user, update_post,
};
use crate::services::user_service::{
    delete_user, get_user, get_user_by_username, login_user, register_user, update_user,
};
use crate::utils::jwt::refresh_token;
use axum::{Router, routing::delete, routing::get, routing::post, routing::put};
use sqlx::PgPool;

pub fn create_router(pool: PgPool) -> Router<()> {
    Router::new()
        .route("/api/v1/users/login", post(login_user))
        .route("/api/v1/users/register", post(register_user))
        .route(
            "/api/v1/users/username/:username",
            get(get_user_by_username),
        )
        .route("/api/v1/users/:target_id", get(get_user))
        .route("/api/v1/users/me", put(update_user).delete(delete_user))
        .route("/api/v1/posts", post(create_post))
        .route("/api/v1/posts/me", get(get_my_posts))
        .route("/api/v1/posts/user/:target_id", get(get_posts_from_user))
        .route(
            "/api/v1/posts/:target_id",
            get(get_post_by_id).put(update_post).delete(delete_post),
        )
        .route(
            "/api/v1/posts/likes/:target_id",
            get(get_likes).post(like_post).delete(unlike_post),
        )
        .with_state(pool)
}
