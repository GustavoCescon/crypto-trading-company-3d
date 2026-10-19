use crate::{db, Error};
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, Transaction};

async fn transition(
    tx: &mut Transaction<'_, Postgres>,
    state: &str,
    message: &str,
) -> Result<(), Error> {
    sqlx::query("UPDATE company_runtime SET state=$1,message=$2,updated_at=now() WHERE id=1")
        .bind(state)
        .bind(message)
        .execute(&mut **tx)
        .await?;
    db::activity(
        tx,
        "company",
        "company_state",
        json!({"state":state,"message":message,"simulated":true}),
    )
    .await
}

pub async fn request(pool: &PgPool, start: bool) -> Result<Value, Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT id FROM wallet WHERE id=1 FOR UPDATE")
        .execute(&mut *tx)
        .await?;
    let (state,): (String,) =
        sqlx::query_as("SELECT state FROM company_runtime WHERE id=1 FOR UPDATE")
            .fetch_one(&mut *tx)
            .await?;
    if start && matches!(state.as_str(), "STOPPED" | "ERROR") {
        transition(
            &mut tx,
            "STARTING",
            "Verificando carteira, risco e conexão com o mercado...",
        )
        .await?;
        sqlx::query("UPDATE company_runtime SET requested_at=now() WHERE id=1")
            .execute(&mut *tx)
            .await?;
    } else if !start && matches!(state.as_str(), "RUNNING" | "STARTING" | "ERROR") {
        transition(&mut tx,"STOPPING","Encerrando expediente. As posições serão fechadas de forma simulada quando houver cotações válidas.").await?;
        sqlx::query("UPDATE company_runtime SET requested_at=now() WHERE id=1")
            .execute(&mut *tx)
            .await?;
        let cancelled = sqlx::query("UPDATE intents SET status='EXPIRED',reserved=0,risk_reason='Entrada cancelada: empresa encerrando expediente' WHERE side='BUY' AND status IN ('PENDING','APPROVED')").execute(&mut *tx).await?.rows_affected();
        db::activity(
            &mut tx,
            "company",
            "entries_cancelled",
            json!({"count":cancelled}),
        )
        .await?;
    }
    let (result,): (Value,) =
        sqlx::query_as("SELECT to_jsonb(c) FROM company_runtime c WHERE id=1")
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(result)
}

/// Called by the risk worker, independent of browser connections.
pub async fn reconcile(pool: &PgPool) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT id FROM wallet WHERE id=1 FOR UPDATE")
        .execute(&mut *tx)
        .await?;
    let (state,expired): (String,bool) = sqlx::query_as("SELECT state,requested_at<now()-interval '3 minutes' FROM company_runtime WHERE id=1 FOR UPDATE").fetch_one(&mut *tx).await?;
    if state == "STARTING" {
        let (ready,): (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM wallet WHERE id=1 AND cash>=0) AND EXISTS(SELECT 1 FROM quotes WHERE observed_at BETWEEN now()-interval '30 seconds' AND now()+interval '5 seconds') AND EXISTS(SELECT 1 FROM fx_quotes WHERE pair='USDTBRL' AND observed_at BETWEEN now()-interval '60 seconds' AND now()+interval '5 seconds') AND (SELECT count(*) FROM agents WHERE id IN ('risk','execution','scanner') AND state<>'ERROR' AND heartbeat>now()-interval '90 seconds')=3").fetch_one(&mut *tx).await?;
        if ready {
            transition(
                &mut tx,
                "RUNNING",
                "Empresa ligada. Operações exclusivamente fictícias habilitadas.",
            )
            .await?;
        } else if expired {
            transition(&mut tx,"ERROR","Não foi possível validar os serviços ou as cotações. Novas entradas permanecem bloqueadas; tente ligar novamente após a recuperação.").await?;
        }
    } else if state == "STOPPING" {
        let (positions,pending): (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM positions),(SELECT count(*) FROM intents WHERE status IN ('PENDING','APPROVED'))").fetch_one(&mut *tx).await?;
        if positions == 0 && pending == 0 {
            transition(
                &mut tx,
                "STOPPED",
                "Todas as operações foram encerradas. Empresa desligada.",
            )
            .await?;
        } else {
            let message = format!("Encerrando expediente: {positions} posições e {pending} ordens ainda acompanhadas. Risco e monitoramento continuam ativos.");
            sqlx::query(
                "UPDATE company_runtime SET message=$1,updated_at=now() WHERE id=1 AND message<>$1",
            )
            .bind(message)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
