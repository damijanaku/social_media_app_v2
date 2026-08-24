#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod models;
mod routemount;
mod services;
mod utils;

use axum::Router;
use deadpool_redis::{Config as RedisConfig, Runtime};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::error::Error;
use std::fs;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{limit::RequestBodyLimitLayer, timeout::TimeoutLayer};
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

    let db_host = env::var("DB_HOST").unwrap_or_else(|_| "db".to_string());
    let db_port = env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let db_user = env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".to_string());

    let db_pass = fs::read_to_string("/run/secrets/db_password")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "rootroot123".to_string());

    let db_name = fs::read_to_string("/run/secrets/db_name")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "social_media_app_v2".to_string());

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        format!(
            "postgresql://{}:{}@{}:{}/{}?sslmode=disable",
            db_user, db_pass, db_host, db_port, db_name
        )
    });

    let pool_env = env::var("POOL_CONNECTIONS").unwrap_or_else(|_| "12".to_string());
    let max_connections = pool_env
        .parse::<u32>()
        .expect("POOL_CONNECTIONS must be a valid unsigned integer");

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(max_connections / 2)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .test_before_acquire(false)
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

    let app = app
        .layer(ServiceBuilder::new().layer(TimeoutLayer::new(Duration::from_secs(30))))
        .layer(
            ServiceBuilder::new().layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)), // 10MB
        );

    let app = if !is_production {
        app.layer(
            tower_http::trace::TraceLayer::new_for_http()
                .make_span_with(tower_http::trace::DefaultMakeSpan::new())
                .on_response(tower_http::trace::DefaultOnResponse::new()),
        )
    } else {
        app
    };

    if !is_production {
        println!("Logging middleware enabled for development");
    } else {
        println!("Logging middleware disabled for production performance");
    }

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("Listening on http://{}", addr);
    if !is_production {
        println!("Health check: http://localhost:{}/health", port);
    }
    println!("Compression disabled for better performance");

    axum::serve(listener, app).await?;

    Ok(())
}
