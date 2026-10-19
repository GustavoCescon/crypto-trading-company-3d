use crate::Error;
use ctc_domain::{
    risk::{d, RiskLimits},
    strategies::Candle,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, Transaction};

pub async fn activity(
    tx: &mut Transaction<'_, Postgres>,
    producer: &str,
    kind: &str,
    data: Value,
) -> Result<(), Error> {
    sqlx::query("SELECT emit_activity($1,$2,$3)")
        .bind(producer)
        .bind(kind)
        .bind(data)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub async fn agent(
    pool: &PgPool,
    id: &str,
    name: &str,
    role: &str,
    room: &str,
    state: &str,
    detail: Value,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    let previous: Option<(String, Value)> =
        sqlx::query_as("SELECT state,detail FROM agents WHERE id=$1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let changed = previous
        .as_ref()
        .is_none_or(|(s, d)| s != state || d != &detail);
    sqlx::query("INSERT INTO agents(id,name,role,room,state,detail) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(id) DO UPDATE SET state=$5,detail=$6,heartbeat=now(),updated_at=CASE WHEN agents.state<>$5 OR agents.detail<>$6 THEN now() ELSE agents.updated_at END,tasks=agents.tasks+CASE WHEN $7 THEN 1 ELSE 0 END")
        .bind(id).bind(name).bind(role).bind(room).bind(state).bind(&detail).bind(changed).execute(&mut *tx).await?;
    if changed {
        activity(
            &mut tx,
            id,
            "agent_state",
            json!({"agent_id":id,"state":state,"detail":detail}),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
pub fn limits_from_env() -> Result<RiskLimits, Error> {
    let mut l = RiskLimits::default();
    if let Ok(currency) = std::env::var("ACCOUNT_CURRENCY") {
        if currency != "BRL" {
            return Err("ACCOUNT_CURRENCY must be BRL".into());
        }
    }
    macro_rules! env_decimal {
        ($field:ident,$key:expr) => {
            if let Ok(v) = std::env::var($key) {
                l.$field = v.parse()?;
            }
        };
    }
    env_decimal!(initial_balance, "INITIAL_BALANCE");
    env_decimal!(order_budget, "ORDER_BUDGET");
    env_decimal!(max_asset_fraction, "MAX_ASSET_EXPOSURE");
    env_decimal!(max_total_fraction, "MAX_TOTAL_EXPOSURE");
    env_decimal!(daily_loss_fraction, "DAILY_LOSS_LIMIT");
    env_decimal!(max_drawdown_fraction, "MAX_DRAWDOWN");
    env_decimal!(stop_loss_fraction, "STOP_LOSS");
    env_decimal!(take_profit_fraction, "TAKE_PROFIT");
    env_decimal!(take_profit_brl, "TAKE_PROFIT_BRL");
    env_decimal!(fee_bps, "FEE_BPS");
    env_decimal!(spread_bps, "SPREAD_BPS");
    env_decimal!(slippage_bps, "SLIPPAGE_BPS");
    if let Ok(v) = std::env::var("MAX_POSITIONS") {
        l.max_positions = v.parse()?;
    }
    if l.initial_balance != d("1000")
        || l.max_total_fraction > d("0.30")
        || l.max_asset_fraction > d("0.30")
        || l.max_positions > 5
    {
        return Err("Configured capital and risk must respect BRL 1,000, 30% total exposure, 30% per asset and at most 5 positions".into());
    }
    l.validate()?;
    Ok(l)
}
pub async fn init_wallet(pool: &PgPool, l: &RiskLimits) -> Result<(), Error> {
    sqlx::query("INSERT INTO wallet(id,initial_balance,cash,peak_equity,day_start_equity,limits) VALUES(1,$1,$1,$1,$1,$2) ON CONFLICT(id) DO UPDATE SET limits=$2")
        .bind(l.initial_balance).bind(serde_json::to_value(l)?).execute(pool).await?;
    Ok(())
}
pub async fn candles(pool: &PgPool, symbol: &str, limit: i64) -> Result<Vec<Candle>, Error> {
    let rows:Vec<(i64,f64,f64,f64,f64,f64)>=sqlx::query_as("SELECT (extract(epoch FROM open_time)*1000)::bigint,open::float8,high::float8,low::float8,close::float8,volume::float8 FROM (SELECT * FROM market_candles WHERE symbol=$1 AND timeframe='1m' AND close_time<now() ORDER BY open_time DESC LIMIT $2) c ORDER BY open_time")
        .bind(symbol).bind(limit).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|(time, open, high, low, close, volume)| Candle {
            time,
            open,
            high,
            low,
            close,
            volume,
        })
        .collect())
}
pub async fn snapshot(pool: &PgPool) -> Result<Value, Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let (cursor,): (i64,) = sqlx::query_as("SELECT COALESCE(max(sequence),0) FROM audit_events")
        .fetch_one(&mut *tx)
        .await?;
    let agents:Vec<(Value,)>=sqlx::query_as("SELECT to_jsonb(a)||jsonb_build_object('online',state<>'RETIRED' AND heartbeat>now()-interval '90 seconds') FROM agents a ORDER BY room,id").fetch_all(&mut *tx).await?;
    let ranking:Vec<(Value,)>=sqlx::query_as("SELECT r.data||jsonb_build_object('score',r.score,'updated_at',r.updated_at,'price',q.price::text,'quote_time',q.observed_at) FROM rankings r LEFT JOIN quotes q USING(symbol) ORDER BY r.score DESC,symbol LIMIT 50").fetch_all(&mut *tx).await?;
    let fx: Option<(Decimal, chrono::DateTime<chrono::Utc>)> =
        sqlx::query_as("SELECT price,observed_at FROM fx_quotes WHERE pair='USDTBRL'")
            .fetch_optional(&mut *tx)
            .await?;
    let fx_rate = fx.as_ref().map(|v| v.0).unwrap_or(Decimal::ZERO);
    let fx_stale = fx.as_ref().is_none_or(|v| {
        (chrono::Utc::now() - v.1).num_seconds() > 60
            || v.1 > chrono::Utc::now() + chrono::Duration::seconds(5)
    });
    let positions:Vec<(Value,)>=sqlx::query_as("SELECT jsonb_build_object('symbol',p.symbol,'quantity',p.quantity::text,'cost',p.cost::text,'average_price',(p.cost/NULLIF(p.quantity*COALESCE(f.price,0),0))::text,'price',q.price::text,'quote_currency','USDT','value',(p.quantity*COALESCE(q.price*f.price,p.cost))::text,'currency','BRL','unrealized_pnl',(p.quantity*COALESCE(q.price*f.price,p.cost)-p.cost)::text,'opened_at',p.opened_at,'strategy',p.strategy,'stale',q.observed_at IS NULL OR q.observed_at<now()-interval '30 seconds' OR f.observed_at IS NULL OR f.observed_at<now()-interval '60 seconds') FROM positions p LEFT JOIN quotes q USING(symbol) LEFT JOIN fx_quotes f ON f.pair='USDTBRL' ORDER BY p.symbol").fetch_all(&mut *tx).await?;
    let wallet: Option<(Decimal, Decimal, Decimal, Decimal, Decimal, Value)> = sqlx::query_as(
        "SELECT initial_balance,cash,realized_pnl,fees,peak_equity,limits FROM wallet WHERE id=1",
    )
    .fetch_optional(&mut *tx)
    .await?;
    let (exposure,unrealized):(Decimal,Decimal)=sqlx::query_as("SELECT COALESCE(sum(COALESCE(p.quantity*q.price*f.price,p.cost)),0),COALESCE(sum(COALESCE(p.quantity*q.price*f.price,p.cost)-p.cost),0) FROM positions p LEFT JOIN quotes q USING(symbol) LEFT JOIN fx_quotes f ON f.pair='USDTBRL'").fetch_one(&mut *tx).await?;
    let (reserved,): (Decimal,) =
        sqlx::query_as("SELECT COALESCE(sum(reserved),0) FROM intents WHERE status='APPROVED'")
            .fetch_one(&mut *tx)
            .await?;
    let (company,): (Value,) =
        sqlx::query_as("SELECT to_jsonb(c) FROM company_runtime c WHERE id=1")
            .fetch_one(&mut *tx)
            .await?;
    let account=wallet.map(|(initial,cash,realized,fees,peak,limits)|json!({"initial_balance":initial,"currency":"BRL","fx_rate":if fx_rate>Decimal::ZERO{Some(fx_rate)}else{None},"fx_source":"binance-public-ticker","fx_stale":fx_stale,"fx_observed_at":fx.as_ref().map(|v|v.1),"cash":cash,"available":cash-reserved,"reserved":reserved,"equity":cash+exposure,"exposure":exposure,"unrealized_pnl":unrealized,"realized_pnl":realized,"fees":fees,"drawdown":if peak>d("0"){((peak-cash-exposure)/peak).max(d("0"))}else{d("0")},"limits":limits,"simulated":true}));
    let decisions: Vec<(Value,)> = sqlx::query_as(
        "SELECT to_jsonb(d)||jsonb_build_object('detail',detail-'inputs') FROM (SELECT * FROM decisions ORDER BY created_at DESC LIMIT 80) d",
    )
    .fetch_all(&mut *tx)
    .await?;
    let orders:Vec<(Value,)>=sqlx::query_as("SELECT to_jsonb(i)||jsonb_build_object('quantity',quantity::text,'reserved',reserved::text) FROM (SELECT * FROM intents ORDER BY created_at DESC LIMIT 80) i").fetch_all(&mut *tx).await?;
    let fills:Vec<(Value,)>=sqlx::query_as("SELECT to_jsonb(f)||jsonb_build_object('quantity',quantity::text,'price',price::text,'fee',fee::text,'realized_pnl',realized_pnl::text,'slippage_cost',slippage_cost::text,'total_brl',(quantity*price*fx_rate)::text) FROM (SELECT * FROM fills ORDER BY created_at DESC LIMIT 100) f").fetch_all(&mut *tx).await?;
    let teams:Vec<(Value,)>=sqlx::query_as("SELECT jsonb_build_object('strategy',strategy,'trades',count(*),'closed',count(*) FILTER(WHERE side='SELL'),'wins',count(*) FILTER(WHERE side='SELL' AND realized_pnl>0),'realized_pnl',sum(realized_pnl)::text,'fees',sum(fee)::text) FROM fills GROUP BY strategy ORDER BY sum(realized_pnl) DESC").fetch_all(&mut *tx).await?;
    let org_teams:Vec<(Value,)>=sqlx::query_as("SELECT jsonb_build_object('id',t.id,'name',t.name,'department_id',t.department_id,'state',t.state,'created_at',t.created_at,'manager_agent_id',t.manager_agent_id,'manager_name',m.name,'agent_count',count(a.id) FILTER(WHERE a.state<>'RETIRED'),'performance_points',COALESCE(sum(a.performance_points) FILTER(WHERE a.state<>'RETIRED'),0)::text,'penalty_points',COALESCE(sum(a.penalty_points) FILTER(WHERE a.state<>'RETIRED'),0)::text) FROM company_teams t LEFT JOIN agents a ON a.team_id=t.id LEFT JOIN agents m ON m.id=t.manager_agent_id GROUP BY t.id,m.name ORDER BY t.state,t.name").fetch_all(&mut *tx).await?;
    let latest_management:Option<(Value,)>=sqlx::query_as("SELECT jsonb_build_object('id',id,'model',model,'summary',summary,'recommendations',recommendations,'applied',applied,'created_at',created_at) FROM management_reviews ORDER BY created_at DESC LIMIT 1").fetch_optional(&mut *tx).await?;
    let ai_runtime:Option<(Value,)>=sqlx::query_as("SELECT jsonb_build_object('model',model,'memory_mb',memory_mb,'cpu_cores',cpu_cores,'max_agents',max_agents,'review_minutes',review_minutes,'updated_at',updated_at) FROM ai_runtime_state WHERE id=1").fetch_optional(&mut *tx).await?;
    let strategy_policies:Vec<(Value,)>=sqlx::query_as("SELECT jsonb_build_object('strategy',strategy,'enabled',enabled,'weight',weight::text,'rationale',rationale,'updated_at',updated_at) FROM strategy_policies ORDER BY strategy").fetch_all(&mut *tx).await?;
    let penalties:Vec<(Value,)>=sqlx::query_as("SELECT jsonb_build_object('agent_id',agent_id,'fill_id',fill_id,'points',points::text,'loss_brl',loss_brl::text,'reason',reason,'created_at',created_at) FROM management_penalties ORDER BY created_at DESC LIMIT 20").fetch_all(&mut *tx).await?;
    let news: Vec<(Value,)> = sqlx::query_as(
        "SELECT to_jsonb(n) FROM (SELECT * FROM news ORDER BY collected_at DESC LIMIT 30) n",
    )
    .fetch_all(&mut *tx)
    .await?;
    let events:Vec<(Value,)>=sqlx::query_as("SELECT envelope||jsonb_build_object('sequence',sequence) FROM audit_events ORDER BY sequence DESC LIMIT 80").fetch_all(&mut *tx).await?;
    let equity:Vec<(Value,)>=sqlx::query_as("SELECT jsonb_build_object('time',observed_at,'equity',equity::text) FROM (SELECT * FROM equity_history ORDER BY observed_at DESC LIMIT 240) e ORDER BY observed_at").fetch_all(&mut *tx).await?;
    let backtests: Vec<(Value,)> = sqlx::query_as(
        "SELECT to_jsonb(b) FROM (SELECT * FROM backtests ORDER BY created_at DESC LIMIT 10) b",
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    fn values(rows: Vec<(Value,)>) -> Vec<Value> {
        rows.into_iter().map(|r| r.0).collect()
    }
    Ok(
        json!({"schema_version":1,"type":"snapshot","cursor":cursor,"server_time":chrono::Utc::now(),"company":company,"account":account,"agents":values(agents),"ranking":values(ranking),"positions":values(positions),"decisions":values(decisions),"orders":values(orders),"fills":values(fills),"teams":values(teams),"org_teams":values(org_teams),"management":{"latest_review":latest_management.map(|r|r.0),"runtime":ai_runtime.map(|r|r.0),"strategy_policies":values(strategy_policies),"penalties":values(penalties)},"news":values(news),"events":values(events),"equity_history":values(equity),"backtests":values(backtests),"simulation":true}),
    )
}
pub fn number(v: &Value) -> f64 {
    v.as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| v.as_f64())
        .filter(|v| v.is_finite())
        .unwrap_or(0.0)
}
pub fn decimal(v: &Value) -> Result<Decimal, Error> {
    Ok(v.as_str().ok_or("expected decimal string")?.parse()?)
}
pub fn as_float(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(0.0)
}
