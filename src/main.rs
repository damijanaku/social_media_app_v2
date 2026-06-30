mod models;
mod routemount;
mod services;
mod utils;

use deadpool_redis::{Config as RedisConfig, Runtime};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::error::Error;
use utils::cache::CacheService;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
    pub cache: CacheService,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    let is_production = app_env == "production";

    println!("Starting server in {} mode", app_env);

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env or environment");

    let pool_env = env::var("POOL_CONNECTIONS").unwrap_or_else(|_| "12".to_string());

    let max_connections = pool_env
        .parse::<u32>()
        .expect("POOL_CONNECTIONS must be a valid unsigned integer");

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(&database_url)
        .await
        .expect("Failed to connect to the database pool");

    sqlx::migrate!("./migrations").run(&pool).await?;

    println!("Successfully connected to PostgreSQL!");

    let redis_url = env::var("REDIS_URL").expect("Redis url must be set in .env file");
    let redis_cfg = RedisConfig::from_url(redis_url);
    let redis_pool = redis_cfg
        .create_pool(Some(Runtime::Tokio1))
        .expect("failed to create redis pool");

    {
        let mut conn = redis_pool
            .get()
            .await
            .expect("Failed to get redis connection");
        let _: () = deadpool_redis::redis::cmd("PING")
            .query_async(&mut conn)
            .await?;
    }
    println!("Successfully connected to Redis");

    let cache = CacheService::new(redis_pool.clone());

    let state = AppState {
        db: pool,
        redis: redis_pool,
        cache,
    };

    let app = crate::routemount::route::create_router(state);

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!(" Listening on http://{}", addr);
    if !is_production {
        println!("Health check: http://localhost:{}/health", port);
    }

    axum::serve(listener, app).await?;

    Ok(())
}
