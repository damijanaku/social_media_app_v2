use crate::services::user_service::{login_user, register_user};
use crate::utils::jwt::refresh_token;
use axum::{Router, routing::delete, routing::get, routing::post, routing::put};
use sqlx::PgPool;

pub fn create_router(pool: PgPool) -> Router<()> {
    Router::new()
        .route("/api/v1/users/login", post(login_user))
        .route("/api/v1/users/register", post(register_user))
        .with_state(pool)
}
