mod models;
mod routemount;
mod utils;

use sqlx::postgres::PgPoolOptions;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env or environment");

    let pool_env = std::env::var("POOL_CONNECTIONS")
        .expect("POOL_CONNECTIONS environment variable must be set");

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

    let app = crate::routemount::create_router(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
