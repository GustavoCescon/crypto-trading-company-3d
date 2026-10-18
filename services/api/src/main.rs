use ctc_api::{AppState, Config, Error, MIGRATOR};
use ctc_contracts::EventEnvelope;
use sqlx::postgres::PgPoolOptions;
use std::{sync::atomic::Ordering, time::Duration};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let config = Config::from_env()?;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&config.database_url)
        .await?;
    MIGRATOR.run(&pool).await?;
    let redis = redis::Client::open(config.redis_url)?
        .get_connection_manager()
        .await?;
    let state = AppState::new(pool, redis);
    ctc_api::record_event(
        &state.pool,
        &EventEnvelope::service_started("api", env!("CARGO_PKG_VERSION")),
    )
    .await?;
    let publisher_state = state.clone();
    let publisher = tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tick.tick().await;
            if ctc_api::publish_batch(&publisher_state).await.is_err() {
                publisher_state
                    .publish_errors
                    .fetch_add(1, Ordering::Relaxed);
                // Do not log credentials or database URLs from driver errors.
                tracing::warn!("outbox publication failed; pending events retained for retry");
            }
        }
    });
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(address = %config.bind_addr, mode = "simulated", "API started; execution disabled");
    let result = axum::serve(listener, ctc_api::router(state.clone()))
        .with_graceful_shutdown(shutdown())
        .await;
    publisher.abort();
    let _ = publisher.await;
    state.pool.close().await;
    result?;
    Ok(())
}

async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}
