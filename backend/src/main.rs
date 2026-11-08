use std::env;

use backend::{Config, MIGRATOR, router};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL")?;
    let public_base_url =
        env::var("PUBLIC_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".into());
    let backend_base_url = env::var("BACKEND_BASE_URL").ok().filter(|v| !v.is_empty());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".into());

    let pool = PgPoolOptions::new().connect(&database_url).await?;
    MIGRATOR.run(&pool).await?;

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        router(
            pool,
            Config {
                public_base_url,
                backend_base_url,
            },
        ),
    )
    .await?;
    Ok(())
}
