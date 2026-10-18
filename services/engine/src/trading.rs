use crate::{db, Error};
use chrono::{DateTime, Utc};
use ctc_domain::risk::{d, fill_price, quantity, veto, Exposure, RiskLimits};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::time::Duration;
use uuid::Uuid;

pub trait ExchangeAdapter {
    fn execute(
        &self,
        reference: Decimal,
        quantity: Decimal,
        buy: bool,
        limits: &RiskLimits,
    ) -> SimulatedFill;
}
pub struct SimulatedExchangeAdapter;
pub struct SimulatedFill {
    pub price: Decimal,
    pub fee: Decimal,
    pub slippage_cost: Decimal,
}
impl ExchangeAdapter for SimulatedExchangeAdapter {
    fn execute(
        &self,
        reference: Decimal,
        quantity: Decimal,
        buy: bool,
        limits: &RiskLimits,
    ) -> SimulatedFill {
        let price = fill_price(reference, buy, limits);
        SimulatedFill {
            price,
            fee: quantity * price * limits.fee_bps / d("10000"),
            slippage_cost: quantity * (price - reference).abs(),
        }
    }
}
async fn exposure(
    tx: &mut Transaction<'_, Postgres>,
    symbol: &str,
    exclude: Uuid,
) -> Result<Exposure, Error> {
    let row = sqlx::query("SELECT * FROM wallet WHERE id=1 FOR UPDATE")
        .fetch_one(&mut **tx)
        .await?;
    let cash: Decimal = row.get("cash");
    let peak: Decimal = row.get("peak_equity");
    let mut day_start: Decimal = row.get("day_start_equity");
    let day: chrono::NaiveDate = row.get("trading_day");
    let (total,asset,count,stale):(Decimal,Decimal,i64,i64)=sqlx::query_as("SELECT COALESCE(sum(COALESCE(p.quantity*q.price,p.cost)),0),COALESCE(sum(CASE WHEN p.symbol=$1 THEN COALESCE(p.quantity*q.price,p.cost) ELSE 0 END),0),count(*),count(*) FILTER(WHERE q.observed_at IS NULL OR q.observed_at<now()-interval '30 seconds') FROM positions p LEFT JOIN quotes q USING(symbol)")
        .bind(symbol).fetch_one(&mut **tx).await?;
    let (reserved,new_positions):(Decimal,i64)=sqlx::query_as("SELECT COALESCE(sum(reserved),0),count(*) FILTER(WHERE side='BUY' AND NOT EXISTS(SELECT 1 FROM positions p WHERE p.symbol=i.symbol)) FROM intents i WHERE status='APPROVED' AND id<>$1")
        .bind(exclude).fetch_one(&mut **tx).await?;
    let equity = cash + total;
    let peak = peak.max(equity);
    if day != Utc::now().date_naive() {
        day_start = equity;
    }
    sqlx::query("UPDATE wallet SET peak_equity=$1,day_start_equity=$2,trading_day=$3,updated_at=now() WHERE id=1").bind(peak).bind(day_start).bind(Utc::now().date_naive()).execute(&mut **tx).await?;
    Ok(Exposure {
        cash,
        reserved,
        equity,
        asset,
        total,
        positions: (count + new_positions) as usize,
        daily_loss: if day_start > d("0") {
            ((day_start - equity) / day_start).max(d("0"))
        } else {
            d("1")
        },
        drawdown: if stale > 0 {
            d("1")
        } else if peak > d("0") {
            ((peak - equity) / peak).max(d("0"))
        } else {
            d("1")
        },
    })
}
fn order_filters(filters: &Value, qty: Decimal, notional: Decimal) -> Result<(), String> {
    let list = filters.as_array().ok_or("Filtros Binance ausentes")?;
    let mut lot_seen = false;
    let mut notional_seen = false;
    for f in list {
        let kind = f["filterType"].as_str().unwrap_or("");
        if kind == "LOT_SIZE" || kind == "MARKET_LOT_SIZE" {
            lot_seen = true;
            let min = db::decimal(&f["minQty"]).map_err(|_| "minQty inválido")?;
            let max = db::decimal(&f["maxQty"]).map_err(|_| "maxQty inválido")?;
            let step = db::decimal(&f["stepSize"]).map_err(|_| "stepSize inválido")?;
            if qty < min || (max > d("0") && qty > max) || (step > d("0") && qty % step != d("0")) {
                return Err(format!("Filtro Binance {kind}"));
            }
        }
        if kind == "MIN_NOTIONAL" || kind == "NOTIONAL" {
            notional_seen = true;
            let min = db::decimal(&f["minNotional"]).map_err(|_| "minNotional inválido")?;
            if notional < min {
                return Err("Notional abaixo do mínimo".into());
            }
            if let Ok(max) = db::decimal(&f["maxNotional"]) {
                if max > d("0") && notional > max {
                    return Err("Notional acima do máximo".into());
                }
            }
        }
    }
    if !lot_seen || !notional_seen {
        return Err("Filtros obrigatórios ausentes".into());
    }
    Ok(())
}
fn lot_step(filters: &Value) -> Result<Decimal, Error> {
    let list = filters.as_array().ok_or("missing filters")?;
    let lot = list
        .iter()
        .find(|f| f["filterType"] == "LOT_SIZE")
        .ok_or("LOT_SIZE missing")?;
    db::decimal(&lot["stepSize"])
}
async fn reject(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    symbol: &str,
    reason: &str,
) -> Result<(), Error> {
    sqlx::query("UPDATE intents SET status='REJECTED',risk_reason=$2,reserved=0 WHERE id=$1")
        .bind(id)
        .bind(reason)
        .execute(&mut **tx)
        .await?;
    db::activity(
        tx,
        "risk",
        "risk_rejected",
        json!({"intent_id":id,"symbol":symbol,"reason":reason}),
    )
    .await
}
pub async fn risk_once(pool: &PgPool, l: &RiskLimits) -> Result<Option<Value>, Error> {
    let mut tx = pool.begin().await?;
    // Global wallet lock comes first in BOTH risk and execution, serializing reservations.
    sqlx::query("SELECT id FROM wallet WHERE id=1 FOR UPDATE")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE intents SET status='EXPIRED',reserved=0,risk_reason='Prazo expirado' WHERE status IN ('APPROVED','PENDING') AND expires_at<=now()").execute(&mut *tx).await?;
    let row=sqlx::query("SELECT * FROM intents WHERE status='PENDING' ORDER BY created_at LIMIT 1 FOR UPDATE SKIP LOCKED").fetch_optional(&mut *tx).await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Ok(None);
    };
    let id: Uuid = row.get("id");
    let symbol: String = row.get("symbol");
    let side: String = row.get("side");
    let created: DateTime<Utc> = row.get("created_at");
    let q=sqlx::query("SELECT q.*,s.filters,s.eligible,s.updated_at AS filters_updated FROM quotes q JOIN market_symbols s USING(symbol) WHERE symbol=$1").bind(&symbol).fetch_optional(&mut *tx).await?;
    let Some(q) = q else {
        reject(&mut tx, id, &symbol, "Sem cotação real").await?;
        tx.commit().await?;
        return Ok(Some(
            json!({"symbol":symbol,"state":"BLOCKED","reason":"Sem cotação real"}),
        ));
    };
    let price: Decimal = q.get("price");
    let observed: DateTime<Utc> = q.get("observed_at");
    let filters: Value = q.get("filters");
    let filters_updated: DateTime<Utc> = q.get("filters_updated");
    let e = exposure(&mut tx, &symbol, id).await?;
    let existing: Option<(Decimal,)> =
        sqlx::query_as("SELECT quantity FROM positions WHERE symbol=$1")
            .bind(&symbol)
            .fetch_optional(&mut *tx)
            .await?;
    let buy = side == "BUY";
    let fp = fill_price(price, buy, l);
    let qty = if buy {
        quantity(l.order_budget, fp, lot_step(&filters)?, l.fee_bps)
    } else {
        existing.map(|r| r.0).unwrap_or(d("0"))
    };
    let reserve = if buy {
        qty * fp * (d("1") + l.fee_bps / d("10000")) * d("1.01")
    } else {
        d("0")
    };
    let approval = (|| -> Result<(), String> {
        if Utc::now().signed_duration_since(observed).num_seconds() > 30
            || observed > Utc::now() + chrono::Duration::seconds(5)
        {
            return Err("Cotação fora da validade".into());
        }
        if Utc::now()
            .signed_duration_since(filters_updated)
            .num_hours()
            >= 2
        {
            return Err("Filtros desatualizados".into());
        }
        if buy {
            if !q.get::<bool, _>("eligible") {
                return Err("Ativo inelegível".into());
            }
            if existing.is_some() {
                return Err("Já existe posição; sem piramidagem no MVP".into());
            }
            veto(
                l,
                &e,
                reserve,
                Utc::now().signed_duration_since(created).num_seconds(),
                Utc::now().signed_duration_since(observed).num_seconds(),
            )?;
        } else if qty <= d("0") {
            return Err("Não há posição para vender".into());
        }
        order_filters(&filters, qty, qty * fp)?;
        Ok(())
    })();
    let output = match approval {
        Err(reason) => {
            reject(&mut tx, id, &symbol, &reason).await?;
            json!({"symbol":symbol,"state":"BLOCKED","reason":reason})
        }
        Ok(()) => {
            let snapshot = json!({"limits":l,"price":price,"quote_time":observed,"equity":e.equity,"cash":e.cash,"reserved_before":e.reserved,"filters":filters,"rules_version":"risk-1.0.0"});
            sqlx::query("UPDATE intents SET status='APPROVED',quantity=$2,reserved=$3,approved_at=now(),risk_reason='Limites e filtros aprovados',risk_snapshot=$4 WHERE id=$1")
                .bind(id).bind(qty).bind(reserve).bind(snapshot).execute(&mut *tx).await?;
            db::activity(&mut tx,"risk","risk_approved",json!({"intent_id":id,"symbol":symbol,"side":side,"quantity":qty,"reserved":reserve})).await?;
            json!({"symbol":symbol,"state":"APPROVING","side":side,"quantity":qty,"intent_id":id})
        }
    };
    tx.commit().await?;
    Ok(Some(output))
}
pub async fn execution_once(pool: &PgPool, l: &RiskLimits) -> Result<Option<Value>, Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT id FROM wallet WHERE id=1 FOR UPDATE")
        .execute(&mut *tx)
        .await?;
    let row=sqlx::query("SELECT i.*,q.price,q.observed_at FROM intents i LEFT JOIN quotes q USING(symbol) WHERE status='APPROVED' ORDER BY approved_at LIMIT 1 FOR UPDATE OF i SKIP LOCKED").fetch_optional(&mut *tx).await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Ok(None);
    };
    let id: Uuid = row.get("id");
    let symbol: String = row.get("symbol");
    let side: String = row.get("side");
    let qty: Decimal = row.get("quantity");
    let expires: DateTime<Utc> = row.get("expires_at");
    let snap: Value = row.get("risk_snapshot");
    let price: Option<Decimal> = row.get("price");
    let observed: Option<DateTime<Utc>> = row.get("observed_at");
    let reference = db::decimal(&snap["price"])?;
    if expires <= Utc::now()
        || price.is_none()
        || observed.is_none_or(|t| Utc::now().signed_duration_since(t).num_seconds() > 30)
        || price.is_some_and(|p| (p / reference - d("1")).abs() > d("0.01"))
        || snap["limits"] != serde_json::to_value(l)?
    {
        sqlx::query("UPDATE intents SET status='EXPIRED',reserved=0,risk_reason='Revalidação: prazo, preço, cotação ou configuração mudou' WHERE id=$1").bind(id).execute(&mut *tx).await?;
        db::activity(
            &mut tx,
            "execution",
            "execution_expired",
            json!({"intent_id":id,"symbol":symbol}),
        )
        .await?;
        tx.commit().await?;
        return Ok(Some(
            json!({"symbol":symbol,"state":"BLOCKED","reason":"Aprovação expirada ou preço alterado"}),
        ));
    }
    let price = price.unwrap();
    let buy = side == "BUY";
    let fill = SimulatedExchangeAdapter.execute(price, qty, buy, l);
    let gross = qty * fill.price;
    let e = exposure(&mut tx, &symbol, id).await?;
    let previous: Option<(Decimal, Decimal, String)> =
        sqlx::query_as("SELECT quantity,cost,strategy FROM positions WHERE symbol=$1")
            .bind(&symbol)
            .fetch_optional(&mut *tx)
            .await?;
    if buy {
        if let Err(reason) = veto(l, &e, gross + fill.fee, 0, 0) {
            reject(&mut tx, id, &symbol, &format!("Revalidação: {reason}")).await?;
            tx.commit().await?;
            return Ok(Some(
                json!({"symbol":symbol,"state":"BLOCKED","reason":reason}),
            ));
        }
    } else if previous.as_ref().is_none_or(|p| p.0 != qty) {
        reject(&mut tx, id, &symbol, "Posição mudou após aprovação").await?;
        tx.commit().await?;
        return Ok(None);
    }
    order_filters(&snap["filters"], qty, gross)?;
    let decision_id: Option<Uuid> = row.get("decision_id");
    let strategy = if buy {
        let decision: Option<(Value,)> = sqlx::query_as("SELECT detail FROM decisions WHERE id=$1")
            .bind(decision_id)
            .fetch_optional(&mut *tx)
            .await?;
        decision
            .and_then(|r| {
                r.0["analysis"]["signals"].as_array().and_then(|a| {
                    a.iter()
                        .max_by(|x, y| db::number(&x["score"]).total_cmp(&db::number(&y["score"])))
                        .and_then(|s| s["strategy"].as_str())
                        .map(str::to_owned)
                })
            })
            .unwrap_or_else(|| "committee".into())
    } else {
        previous.as_ref().unwrap().2.clone()
    };
    let realized = if buy {
        d("0")
    } else {
        gross - fill.fee - previous.as_ref().unwrap().1
    };
    // Insert is guarded by a DB trigger requiring the exact risk-approved intent.
    sqlx::query("INSERT INTO fills(id,intent_id,symbol,side,quantity,price,reference_price,fee,slippage_cost,realized_pnl,strategy) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
        .bind(Uuid::new_v4()).bind(id).bind(&symbol).bind(&side).bind(qty).bind(fill.price).bind(price).bind(fill.fee).bind(fill.slippage_cost).bind(realized).bind(&strategy).execute(&mut *tx).await?;
    if buy {
        sqlx::query("INSERT INTO positions(symbol,quantity,cost,strategy) VALUES($1,$2,$3,$4)")
            .bind(&symbol)
            .bind(qty)
            .bind(gross + fill.fee)
            .bind(&strategy)
            .execute(&mut *tx)
            .await?;
    } else {
        sqlx::query("DELETE FROM positions WHERE symbol=$1")
            .bind(&symbol)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE wallet SET cash=cash+$1,realized_pnl=realized_pnl+$2,fees=fees+$3,updated_at=now() WHERE id=1")
        .bind(if buy{-gross-fill.fee}else{gross-fill.fee}).bind(realized).bind(fill.fee).execute(&mut *tx).await?;
    sqlx::query("UPDATE intents SET status='FILLED',reserved=0 WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    db::activity(&mut tx,"execution","paper_fill",json!({"intent_id":id,"symbol":symbol,"side":side,"quantity":qty,"price":fill.price,"fee":fill.fee,"realized_pnl":realized,"simulated":true})).await?;
    tx.commit().await?;
    Ok(Some(
        json!({"symbol":symbol,"side":side,"state":"EXECUTING","quantity":qty,"price":fill.price,"simulated":true}),
    ))
}
async fn monitor(pool: &PgPool, l: &RiskLimits) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    let e = exposure(&mut tx, "", Uuid::nil()).await?;
    sqlx::query("INSERT INTO equity_history(observed_at,equity,cash,exposure) VALUES(now(),$1,$2,$3) ON CONFLICT DO NOTHING").bind(e.equity).bind(e.cash).bind(e.total).execute(&mut *tx).await?;
    let rows:Vec<(String,Decimal,Decimal,Decimal)>=sqlx::query_as("SELECT p.symbol,p.quantity,p.cost,q.price FROM positions p JOIN quotes q USING(symbol) WHERE q.observed_at>now()-interval '30 seconds'").fetch_all(&mut *tx).await?;
    for (symbol, qty, cost, price) in rows {
        let change = qty * price / cost - d("1");
        if change <= -l.stop_loss_fraction || change >= l.take_profit_fraction {
            let reason = if change < d("0") {
                "Stop-loss"
            } else {
                "Take-profit"
            };
            let id = Uuid::new_v4();
            let result=sqlx::query("INSERT INTO intents(id,symbol,side,reason,expires_at) VALUES($1,$2,'SELL',$3,now()+interval '120 seconds') ON CONFLICT DO NOTHING")
                .bind(id).bind(&symbol).bind(reason).execute(&mut *tx).await?;
            if result.rows_affected() > 0 {
                db::activity(
                    &mut tx,
                    "risk",
                    "protective_exit",
                    json!({"intent_id":id,"symbol":symbol,"reason":reason}),
                )
                .await?;
            }
        }
    }
    tx.commit().await?;
    Ok(())
}
pub async fn risk_loop(pool: PgPool, l: RiskLimits) -> Result<(), Error> {
    let mut counter = 0;
    loop {
        let result = async {
            if counter % 5 == 0 {
                monitor(&pool, &l).await?;
            }
            let result = risk_once(&pool, &l).await?;
            let state = result
                .as_ref()
                .and_then(|v| v["state"].as_str())
                .unwrap_or("MONITORING");
            db::agent(
                &pool,
                "risk",
                "Sofia",
                "Gerente de risco / veto",
                "risk",
                state,
                result.clone().unwrap_or(
                    json!({"task":"Exposição, drawdown, perdas diárias e stops","limits":l}),
                ),
            )
            .await?;
            db::agent(
                &pool,
                "audit",
                "Otávio",
                "Auditoria e performance",
                "audit",
                "MONITORING",
                json!({"task":"Ledger, reservas e equity persistidos"}),
            )
            .await?;
            Ok::<(), Error>(())
        }
        .await;
        if let Err(e) = result {
            tracing::error!(error=%e,"risk cycle failed");
            let _ = db::agent(
                &pool,
                "risk",
                "Sofia",
                "Gerente de risco / veto",
                "risk",
                "ERROR",
                json!({"error":e.to_string()}),
            )
            .await;
        }
        counter += 1;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
pub async fn execution_loop(pool: PgPool, l: RiskLimits) -> Result<(), Error> {
    loop {
        match execution_once(&pool, &l).await {
            Ok(result) => {
                let state = result
                    .as_ref()
                    .and_then(|v| v["state"].as_str())
                    .unwrap_or("IDLE");
                db::agent(&pool,"execution","Theo","Executor de ordens fictícias","execution",state,result.clone().unwrap_or(json!({"task":"Aguardando aprovação de risco","adapter":"SimulatedExchangeAdapter"}))).await?;
            }
            Err(e) => {
                tracing::error!(error=%e,"execution failed; transaction rolled back");
                let _ = db::agent(
                    &pool,
                    "execution",
                    "Theo",
                    "Executor de ordens fictícias",
                    "execution",
                    "ERROR",
                    json!({"error":e.to_string()}),
                )
                .await;
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filters_reject_wrong_step() {
        let f = json!([{"filterType":"LOT_SIZE","minQty":"0.01","maxQty":"100","stepSize":"0.01"},{"filterType":"MIN_NOTIONAL","minNotional":"5"}]);
        assert!(order_filters(&f, d("0.011"), d("10")).is_err());
        assert!(order_filters(&f, d("0.01"), d("10")).is_ok());
    }
    #[test]
    fn simulator_never_fills_at_favorable_slippage() {
        let l = RiskLimits::default();
        let a = SimulatedExchangeAdapter.execute(d("100"), d("1"), true, &l);
        let b = SimulatedExchangeAdapter.execute(d("100"), d("1"), false, &l);
        assert!(a.price > d("100"));
        assert!(b.price < d("100"));
        assert!(a.fee > d("0"));
    }
}
