use ctc_engine::{db, market, trading, Error};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();
    if std::env::var("TRADING_MODE").unwrap_or_else(|_| "simulated".into()) != "simulated" {
        return Err("Only simulated execution is permitted".into());
    }
    let role = std::env::var("WORKER_ROLE")?;
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;
    let limits = db::limits_from_env()?;
    match role.as_str() {
        "market" => market::run(pool).await,
        "risk" => {
            db::init_wallet(&pool, &limits).await?;
            trading::risk_loop(pool, limits).await
        }
        "execution" => trading::execution_loop(pool, limits).await,
        _ => Err("Unknown worker role".into()),
    }
}
