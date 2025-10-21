use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, Pool, Postgres};

use crate::SessionRecord;

pub type SessionPool = Pool<Postgres>;

pub async fn init_pool(database_url: &str) -> Result<Pool<Postgres>> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .with_context(|| format!("connect to {}", database_url))?;

    pool.execute(
        r#"
        CREATE TABLE IF NOT EXISTS session_records (
            session_id TEXT NOT NULL,
            recorded_at TIMESTAMPTZ NOT NULL,
            query TEXT,
            verdict TEXT,
            requires_manual_review BOOLEAN NOT NULL,
            math_status TEXT,
            math_alert_required BOOLEAN NOT NULL,
            math_stdout TEXT,
            math_stderr TEXT,
            trace_path TEXT,
            sandbox_failure_streak INTEGER,
            domain_label TEXT,
            confidence_bucket TEXT,
            consent_provided BOOLEAN,
            math_outputs JSONB,
            PRIMARY KEY (session_id, recorded_at)
        );
        "#,
    )
    .await?;

    Ok(pool)
}

pub async fn insert_records(pool: &Pool<Postgres>, records: &[SessionRecord]) -> Result<()> {
    if records.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await?;

    for record in records {
        let math_outputs =
            serde_json::to_value(&record.math_outputs).context("serialize math outputs")?;
        let recorded_at = chrono::DateTime::parse_from_rfc3339(&record.timestamp)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .context("parse timestamp")?;
        tx.execute(
            sqlx::query(
                r#"
                INSERT INTO session_records (
                    session_id,
                    recorded_at,
                    query,
                    verdict,
                    requires_manual_review,
                    math_status,
                    math_alert_required,
                    math_stdout,
                    math_stderr,
                    trace_path,
                    sandbox_failure_streak,
                    domain_label,
                    confidence_bucket,
                    consent_provided,
                    math_outputs
                ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
                ON CONFLICT (session_id, recorded_at) DO NOTHING
                "#,
            )
            .bind(&record.session_id)
            .bind(recorded_at)
            .bind(&record.query)
            .bind(&record.verdict)
            .bind(record.requires_manual_review)
            .bind(&record.math_status)
            .bind(record.math_alert_required)
            .bind(&record.math_stdout)
            .bind(&record.math_stderr)
            .bind(&record.trace_path)
            .bind(record.sandbox_failure_streak.map(|v| v as i32))
            .bind(&record.domain_label)
            .bind(&record.confidence_bucket)
            .bind(record.consent_provided)
            .bind(math_outputs),
        )
        .await?;
    }

    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn creates_table_and_inserts() -> Result<()> {
        let Some(url) = std::env::var("PIPELINE_TEST_DATABASE_URL").ok() else {
            // Skip when a test database is not provisioned (e.g. CI without Postgres service).
            return Ok(());
        };
        let pool = init_pool(&url).await?;
        sqlx::query("TRUNCATE session_records")
            .execute(&pool)
            .await
            .ok();
        insert_records(&pool, &[]).await?;
        Ok(())
    }
}
