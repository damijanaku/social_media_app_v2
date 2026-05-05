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
        .with_state(pool)
}
