use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use ctc_api::{publish_batch, record_event, router, AppState, MIGRATOR};
use ctc_contracts::{EventEnvelope, EVENT_STREAM};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

#[tokio::test]
#[ignore = "requires disposable TimescaleDB and Redis; run with Compose test profile"]
async fn durable_event_delivery_and_http_contract() {
    let pool = PgPoolOptions::new()
        .connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    let redis = redis::Client::open(std::env::var("REDIS_URL").unwrap())
        .unwrap()
        .get_connection_manager()
        .await
        .unwrap();
    let state = AppState::new(pool.clone(), redis);
    let event = EventEnvelope::service_started("integration-test", "0.1.0");
    record_event(&pool, &event).await.unwrap();
    record_event(&pool, &event).await.unwrap();
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM audit_events WHERE event_id = $1")
        .bind(event.event_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "retry must not create another audit record");
    assert!(sqlx::query("DELETE FROM audit_events WHERE event_id = $1")
        .bind(event.event_id)
        .execute(&pool)
        .await
        .is_err());
    for _ in 0..20 {
        publish_batch(&state).await.unwrap();
        let (published,): (bool,) =
            sqlx::query_as("SELECT published_at IS NOT NULL FROM event_outbox WHERE event_id = $1")
                .bind(event.event_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        if published {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let entries: redis::streams::StreamRangeReply = redis::cmd("XRANGE")
        .arg(EVENT_STREAM)
        .arg("-")
        .arg("+")
        .query_async(&mut state.redis.clone())
        .await
        .unwrap();
    assert!(entries
        .ids
        .iter()
        .any(|entry| entry.get::<String>("event_id") == Some(event.event_id.to_string())));
    let app = router(state.clone());
    for path in [
        "/health/live",
        "/health/ready",
        "/api/v1/system",
        "/api/v1/events",
        "/metrics",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/system")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
    let system: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(system["execution_enabled"], false);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/orders")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Liveness survives a lost database, readiness fails closed.
    pool.close().await;
    let app = router(state);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/live")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
