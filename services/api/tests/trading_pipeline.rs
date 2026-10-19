use ctc_domain::risk::{d, RiskLimits};
use ctc_engine::{
    db,
    trading::{execution_once, risk_once},
};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions,
};
use std::str::FromStr;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires disposable Postgres/TimescaleDB via Compose"]
async fn risk_gate_idempotency_concurrency_and_ledger() {
    let opts = PgConnectOptions::from_str(&std::env::var("DATABASE_URL").unwrap())
        .unwrap()
        .disable_statement_logging();
    let admin = PgPoolOptions::new()
        .connect_with(opts.clone())
        .await
        .unwrap();
    let database = format!("ctc_test_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {database}"))
        .execute(&admin)
        .await
        .unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect_with(opts.database(&database))
        .await
        .unwrap();
    ctc_api::MIGRATOR.run(&pool).await.unwrap();
    let l = RiskLimits::default();
    db::init_wallet(&pool, &l).await.unwrap();
    // Existing trading pipeline runs only after the company is explicitly ready.
    sqlx::query("UPDATE company_runtime SET state='RUNNING' WHERE id=1")
        .execute(&pool)
        .await
        .unwrap();
    let filters = serde_json::json!([{"filterType":"LOT_SIZE","minQty":"0.01","maxQty":"100000","stepSize":"0.01"},{"filterType":"MIN_NOTIONAL","minNotional":"5"}]);
    for symbol in ["AAAUSDT", "BBBUSDT", "CCCUSDT"] {
        sqlx::query(
            "INSERT INTO market_symbols(symbol,base_asset,eligible,filters) VALUES($1,$1,true,$2)",
        )
        .bind(symbol)
        .bind(&filters)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO quotes(symbol,price,bid,ask,observed_at,source) VALUES($1,100,99.99,100.01,now(),'isolated-test-fixture')").bind(symbol).execute(&pool).await.unwrap();
    }
    let buy = Uuid::new_v4();
    sqlx::query("INSERT INTO intents(id,symbol,side,reason,expires_at) VALUES($1,'AAAUSDT','BUY','test',now()+interval '2 minutes')").bind(buy).execute(&pool).await.unwrap();
    assert!(
        execution_once(&pool, &l).await.unwrap().is_none(),
        "pending intent cannot execute"
    );
    assert!(sqlx::query("INSERT INTO fills(id,intent_id,symbol,side,quantity,price,reference_price,fee,slippage_cost,realized_pnl,strategy) VALUES($1,$2,'AAAUSDT','BUY',1,100,100,0,0,0,'test')").bind(Uuid::new_v4()).bind(buy).execute(&pool).await.is_err(),"database refuses bypass");
    risk_once(&pool, &l).await.unwrap();
    let (a, b) = tokio::join!(execution_once(&pool, &l), execution_once(&pool, &l));
    assert_eq!(
        usize::from(a.unwrap().is_some()) + usize::from(b.unwrap().is_some()),
        1
    );
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM fills WHERE intent_id=$1")
        .bind(buy)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    sqlx::query("UPDATE quotes SET price=110,bid=109.99,ask=110.01,observed_at=now() WHERE symbol='AAAUSDT'").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO intents(id,symbol,side,reason,expires_at) VALUES($1,'AAAUSDT','SELL','test',now()+interval '2 minutes')").bind(Uuid::new_v4()).execute(&pool).await.unwrap();
    risk_once(&pool, &l).await.unwrap();
    execution_once(&pool, &l).await.unwrap();
    let (consistent, profitable): (bool, bool) =
        sqlx::query_as("SELECT cash=initial_balance+realized_pnl,realized_pnl>0 FROM wallet")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(consistent && profitable);
    let (fees_match,): (bool,) =
        sqlx::query_as("SELECT fees=(SELECT sum(fee) FROM fills) FROM wallet")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(fees_match);
    let limits = RiskLimits {
        max_positions: 1,
        ..l.clone()
    };
    for symbol in ["BBBUSDT", "CCCUSDT"] {
        sqlx::query("INSERT INTO intents(id,symbol,side,reason,expires_at) VALUES($1,$2,'BUY','concurrent',now()+interval '2 minutes')").bind(Uuid::new_v4()).bind(symbol).execute(&pool).await.unwrap();
    }
    let (a, b) = tokio::join!(risk_once(&pool, &limits), risk_once(&pool, &limits));
    a.unwrap();
    b.unwrap();
    let (approved,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM intents WHERE status='APPROVED'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(approved, 1, "reserved position counts toward max positions");
    sqlx::query("UPDATE quotes SET observed_at=now()-interval '2 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    execution_once(&pool, &limits).await.unwrap();
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM fills")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2, "stale quote never executes");
    assert!(d("10000") > d("0"));
    let stop = ctc_engine::company::request(&pool, false).await.unwrap();
    assert_eq!(stop["state"], "STOPPING");
    let repeated = ctc_engine::company::request(&pool, false).await.unwrap();
    assert_eq!(
        stop["requested_at"], repeated["requested_at"],
        "stop is idempotent"
    );
    assert!(sqlx::query("INSERT INTO intents(id,symbol,side,reason,expires_at) VALUES($1,'AAAUSDT','BUY','must fail while stopping',now()+interval '2 minutes')").bind(Uuid::new_v4()).execute(&pool).await.is_err());
    ctc_engine::company::reconcile(&pool).await.unwrap();
    let (state,): (String,) = sqlx::query_as("SELECT state FROM company_runtime")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "STOPPED");
    assert_eq!(
        ctc_engine::company::request(&pool, true).await.unwrap()["state"],
        "STARTING"
    );
    ctc_engine::company::reconcile(&pool).await.unwrap();
    let (state,): (String,) = sqlx::query_as("SELECT state FROM company_runtime")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        state, "STARTING",
        "stale quotes and missing worker heartbeats cannot start company"
    );
    sqlx::query("UPDATE company_runtime SET requested_at=now()-interval '4 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    ctc_engine::company::reconcile(&pool).await.unwrap();
    let (state,): (String,) = sqlx::query_as("SELECT state FROM company_runtime")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "ERROR");
    ctc_engine::company::request(&pool, true).await.unwrap();
    sqlx::query("UPDATE quotes SET observed_at=now()")
        .execute(&pool)
        .await
        .unwrap();
    for id in ["risk", "execution", "scanner"] {
        db::agent(
            &pool,
            id,
            id,
            id,
            "risk",
            "MONITORING",
            serde_json::json!({}),
        )
        .await
        .unwrap();
    }
    ctc_engine::company::reconcile(&pool).await.unwrap();
    let (state,): (String,) = sqlx::query_as("SELECT state FROM company_runtime")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "RUNNING");
    sqlx::query(
        "INSERT INTO positions(symbol,quantity,cost,strategy) VALUES('AAAUSDT',1,110,'test')",
    )
    .execute(&pool)
    .await
    .unwrap();
    ctc_engine::company::request(&pool, false).await.unwrap();
    ctc_engine::company::reconcile(&pool).await.unwrap();
    let (state,): (String,) = sqlx::query_as("SELECT state FROM company_runtime")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "STOPPING", "cannot stop with open positions");
    // A service restart uses the same persisted state, and another start cannot
    // override a pending shutdown.
    assert_eq!(
        ctc_engine::company::request(&pool, true).await.unwrap()["state"],
        "STOPPING"
    );
    sqlx::query("INSERT INTO intents(id,symbol,side,reason,expires_at) VALUES($1,'AAAUSDT','SELL','shutdown exit',now()+interval '2 minutes')").bind(Uuid::new_v4()).execute(&pool).await.unwrap();
    risk_once(&pool, &l).await.unwrap();
    execution_once(&pool, &l).await.unwrap();
    ctc_engine::company::reconcile(&pool).await.unwrap();
    let (state,): (String,) = sqlx::query_as("SELECT state FROM company_runtime")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        state, "STOPPED",
        "protective sells remain possible during shutdown"
    );
    pool.close().await;
    sqlx::query(&format!("DROP DATABASE {database}"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
