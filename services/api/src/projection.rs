use crate::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::types::{chrono, Decimal};
use std::time::Duration;
use uuid::Uuid;

pub async fn snapshot(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    ctc_engine::db::snapshot(&state.pool)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!(error=%e,"snapshot failed");
            StatusCode::SERVICE_UNAVAILABLE
        })
}
pub async fn decision(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    let row: Option<(Value,)> = sqlx::query_as("SELECT to_jsonb(d) FROM decisions d WHERE id=$1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    row.map(|r| Json(r.0)).ok_or(StatusCode::NOT_FOUND)
}
pub async fn websocket(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.max_message_size(8192).on_upgrade(move |mut socket| async move {
        // Capture the stream cursor BEFORE the snapshot, so events produced
        // during snapshot construction are not missed. Never issue blocking
        // Redis commands on the shared multiplexed connection.
        let mut redis = state.redis.clone();
        let tail = redis::cmd("XREVRANGE").arg(ctc_contracts::EVENT_STREAM)
            .arg("+").arg("-").arg("COUNT").arg(1)
            .query_async::<Vec<(String, Vec<String>)>>(&mut redis).await;
        let Ok(tail) = tail else { return; };
        let mut last_id=tail.first().map(|entry|entry.0.clone()).unwrap_or_else(||"0-0".into());
        // Send one authoritative projection at connection time. Repeating this
        // multi-query snapshot every few seconds overwhelms browsers and DB.
        let Ok(snapshot) = ctc_engine::db::snapshot(&state.pool).await else { return; };
        if socket.send(Message::Text(snapshot.to_string().into())).await.is_err() { return; }
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop{
            tokio::select!{
                _=interval.tick()=>{
                    if socket.send(Message::Ping(Vec::new().into())).await.is_err(){break;}
                },
                reply = async {
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    let mut redis = state.redis.clone();
                    redis::cmd("XREAD").arg("COUNT").arg(100)
                        .arg("STREAMS").arg(ctc_contracts::EVENT_STREAM).arg(&last_id)
                        .query_async::<Option<redis::streams::StreamReadReply>>(&mut redis).await
                } => {
                    if let Ok(Some(reply)) = reply {
                        let mut events = vec![];
                        for stream in reply.keys { for entry in stream.ids {
                            last_id = entry.id.clone();
                            if let Some(raw) = entry.get::<String>("envelope") {
                                if let Ok(event) = serde_json::from_str::<Value>(&raw) { events.push(event); }
                            }
                        }}
                        if !events.is_empty() && socket.send(Message::Text(json!({"type":"events","schema_version":1,"events":events}).to_string().into())).await.is_err(){break;}
                    }
                },
                message=socket.recv()=>{match message {Some(Ok(Message::Ping(p)))=>{if socket.send(Message::Pong(p)).await.is_err(){break;}},Some(Ok(Message::Close(_)))|None|Some(Err(_))=>break,_=>{}}}
            }
        }
    })
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BacktestRequest {
    symbol: String,
    split: String,
}
pub async fn backtest(
    State(state): State<AppState>,
    Json(request): Json<BacktestRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let invalid = |message: &str| (StatusCode::BAD_REQUEST, Json(json!({"error":message})));
    if request.symbol.len() > 24
        || !request.symbol.ends_with("USDT")
        || !request
            .symbol
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return Err(invalid("Símbolo inválido"));
    }
    let candles = ctc_engine::db::candles(&state.pool, &request.symbol, 3000)
        .await
        .map_err(|_| invalid("Histórico indisponível"))?;
    let (limits,): (Value,) = sqlx::query_as("SELECT limits FROM wallet WHERE id=1")
        .fetch_one(&state.pool)
        .await
        .map_err(|_| invalid("Carteira indisponível"))?;
    let mut limits: ctc_domain::risk::RiskLimits =
        serde_json::from_value(limits).map_err(|_| invalid("Limites inválidos"))?;
    let (fx_rate, fx_time): (Decimal, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as("SELECT price,observed_at FROM fx_quotes WHERE pair='USDTBRL' AND observed_at BETWEEN now()-interval '60 seconds' AND now()+interval '5 seconds'")
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| invalid("Cotação de câmbio indisponível"))?
            .ok_or_else(|| invalid("Aguardando primeira cotação USDT/BRL"))?;
    if fx_rate <= Decimal::ZERO || (chrono::Utc::now() - fx_time).num_seconds() > 60 {
        return Err(invalid("Cotação USDT/BRL desatualizada"));
    }
    limits.initial_balance /= fx_rate;
    limits.order_budget /= fx_rate;
    let hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&candles).map_err(|_| invalid("Dados inválidos"))?)
    );
    let split = request.split.clone();
    let report =
        tokio::task::spawn_blocking(move || ctc_domain::backtest::run(&candles, &limits, &split))
            .await
            .map_err(|_| invalid("Falha no cálculo"))?
            .map_err(|e| invalid(&e))?;
    let report = serde_json::to_value(report).map_err(|_| invalid("Resultado inválido"))?;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO backtests(id,symbol,report,dataset_hash) VALUES($1,$2,$3,$4)")
        .bind(id)
        .bind(&request.symbol)
        .bind(&report)
        .bind(&hash)
        .execute(&state.pool)
        .await
        .map_err(|_| invalid("Falha ao persistir resultado"))?;
    Ok(Json(
        json!({"id":id,"symbol":request.symbol,"report":report,"dataset_hash":hash,
            "valuation":{"source":"binance-public-ticker","pair":"USDTBRL","rate":fx_rate,"observed_at":fx_time,
                "starting_capital_brl":1000,"currency":"USDT","historical_fx_included":false}}),
    ))
}
