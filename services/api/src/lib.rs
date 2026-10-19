use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use ctc_contracts::{EventEnvelope, EVENT_STREAM};
use redis::aio::ConnectionManager;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tower_http::trace::TraceLayer;
mod projection;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub bind_addr: String,
}

pub fn validate_trading_mode(mode: &str) -> Result<(), &'static str> {
    match mode {
        "simulated" => Ok(()),
        _ => Err("only TRADING_MODE=simulated is supported in this release"),
    }
}

impl Config {
    pub fn from_env() -> Result<Self, Error> {
        validate_trading_mode(
            &std::env::var("TRADING_MODE").unwrap_or_else(|_| "simulated".into()),
        )?;
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")?,
            redis_url: std::env::var("REDIS_URL")?,
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: ConnectionManager,
    pub published_events: Arc<AtomicU64>,
    pub publish_errors: Arc<AtomicU64>,
}

impl AppState {
    pub fn new(pool: PgPool, redis: ConnectionManager) -> Self {
        Self {
            pool,
            redis,
            published_events: Arc::new(AtomicU64::new(0)),
            publish_errors: Arc::new(AtomicU64::new(0)),
        }
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/health/live",
            get(|| async { Json(json!({"status": "ok"})) }),
        )
        .route("/health/ready", get(ready))
        .route("/api/v1/system", get(system))
        .route("/api/v1/events", get(events))
        .route("/api/v1/snapshot", get(projection::snapshot))
        .route("/api/v1/company/start", axum::routing::post(start_company))
        .route("/api/v1/company/stop", axum::routing::post(stop_company))
        .route("/api/v1/ws", get(projection::websocket))
        .route(
            "/api/v1/backtests",
            axum::routing::post(projection::backtest),
        )
        .route("/api/v1/decisions/{id}", get(projection::decision))
        .route("/metrics", get(metrics))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn system(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let (operating,): (String,) = sqlx::query_as("SELECT state FROM company_runtime WHERE id=1")
        .fetch_one(&state.pool)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(json!({
        "name":"Crypto Trading Company 3D", "version":env!("CARGO_PKG_VERSION"),
        "schema_version":1, "trading_mode":"simulated", "execution_enabled":false,
        "company_state":operating, "paper_execution_enabled":operating=="RUNNING",
        "mvp_complete":false
    })))
}

async fn ready(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let db = async {
        sqlx::query("SELECT 1")
            .execute(&state.pool)
            .await
            .map(|_| ())
    };
    let cache = async {
        redis::cmd("PING")
            .query_async::<String>(&mut state.redis.clone())
            .await
            .map(|_| ())
    };
    let (db, cache) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(2), db),
        tokio::time::timeout(Duration::from_secs(2), cache)
    );
    let database = matches!(db, Ok(Ok(())));
    let redis = matches!(cache, Ok(Ok(())));
    let status = if database && redis {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({"ready": database && redis, "database": database, "redis": redis})),
    )
}

async fn start_company(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    if ready(State(state.clone())).await.0 != StatusCode::OK {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    ctc_engine::company::request(&state.pool, true)
        .await
        .map(Json)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

async fn stop_company(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    ctc_engine::company::request(&state.pool, false)
        .await
        .map(Json)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

async fn events(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    let rows: Vec<(Value,)> =
        sqlx::query_as("SELECT envelope FROM audit_events ORDER BY sequence DESC LIMIT 100")
            .fetch_all(&state.pool)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(
        json!({"schema_version": 1, "events": rows.into_iter().map(|r| r.0).collect::<Vec<_>>()}),
    ))
}

async fn metrics(
    State(state): State<AppState>,
) -> ([(axum::http::HeaderName, &'static str); 1], String) {
    ([(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")], format!(
        "# TYPE ctc_outbox_published_total counter\nctc_outbox_published_total {}\n# TYPE ctc_outbox_errors_total counter\nctc_outbox_errors_total {}\n",
        state.published_events.load(Ordering::Relaxed), state.publish_errors.load(Ordering::Relaxed)
    ))
}

pub async fn record_event(pool: &PgPool, event: &EventEnvelope) -> Result<(), Error> {
    let envelope = serde_json::to_value(event)?;
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO audit_events(event_id, occurred_at, schema_version, envelope) VALUES ($1, ($2::text)::timestamptz, $3, $4) ON CONFLICT(event_id) DO NOTHING")
        .bind(event.event_id).bind(event.occurred_at.to_rfc3339()).bind(event.schema_version as i16).bind(envelope)
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO event_outbox(event_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(event.event_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// At-least-once delivery: consumers MUST deduplicate by event_id before effects.
pub async fn publish_batch(state: &AppState) -> Result<usize, Error> {
    let mut tx = state.pool.begin().await?;
    let rows = sqlx::query("SELECT o.event_id, a.envelope FROM event_outbox o JOIN audit_events a USING(event_id) WHERE o.published_at IS NULL ORDER BY a.sequence LIMIT 100 FOR UPDATE OF o SKIP LOCKED")
        .fetch_all(&mut *tx).await?;
    let mut connection = state.redis.clone();
    for row in &rows {
        let id: uuid::Uuid = row.get("event_id");
        let envelope: Value = row.get("envelope");
        tokio::time::timeout(
            Duration::from_secs(3),
            redis::cmd("XADD")
                .arg(EVENT_STREAM)
                .arg("*")
                .arg("event_id")
                .arg(id.to_string())
                .arg("envelope")
                .arg(envelope.to_string())
                .query_async::<String>(&mut connection),
        )
        .await??;
        sqlx::query("UPDATE event_outbox SET published_at = now() WHERE event_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    state
        .published_events
        .fetch_add(rows.len() as u64, Ordering::Relaxed);
    Ok(rows.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_is_fail_closed() {
        assert!(validate_trading_mode("simulated").is_ok());
        for mode in ["live", "production", "testnet", "demo", "", "SIMULATED"] {
            assert!(validate_trading_mode(mode).is_err());
        }
    }
}
