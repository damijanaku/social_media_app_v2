mod models;
mod routemount;
mod services;
mod utils;

use sqlx::postgres::PgPoolOptions;
use std::env;
use std::error::Error;

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

    let app = crate::routemount::route::create_router(pool);

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
