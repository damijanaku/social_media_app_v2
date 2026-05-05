use axum::{Router, routing::get};
use sqlx::PgPool;

pub fn create_router(_pool: PgPool) -> Router {
    Router::new().route("/health", get(|| async { "ok" }))
}
