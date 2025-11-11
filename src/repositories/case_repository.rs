use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::case::{Case, CaseHistory, CaseStatus};

pub struct CaseRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> CaseRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, case: &Case) -> Result<()> {
        sqlx::query(
            "INSERT INTO orchepy_cases (id, workflow_id, current_phase, previous_phase, data, status, metadata, created_at, updated_at, phase_entered_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind(case.id)
        .bind(case.workflow_id)
        .bind(&case.current_phase)
        .bind(&case.previous_phase)
        .bind(&case.data)
        .bind(&case.status)
        .bind(&case.metadata)
        .bind(case.created_at)
        .bind(case.updated_at)
        .bind(case.phase_entered_at)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Case>> {
        let case = sqlx::query_as::<_, Case>("SELECT * FROM orchepy_cases WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(case)
    }

    pub async fn list_by_workflow(&self, workflow_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Case>> {
        let cases = sqlx::query_as::<_, Case>(
            "SELECT * FROM orchepy_cases WHERE workflow_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(workflow_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await?;

        Ok(cases)
    }

    pub async fn list_by_workflow_and_phase(
        &self,
        workflow_id: Uuid,
        phase: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Case>> {
        let cases = sqlx::query_as::<_, Case>(
            "SELECT * FROM orchepy_cases WHERE workflow_id = $1 AND current_phase = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
        )
        .bind(workflow_id)
        .bind(phase)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await?;

        Ok(cases)
    }

    pub async fn list_by_status(
        &self,
        workflow_id: Uuid,
        status: &CaseStatus,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Case>> {
        let cases = sqlx::query_as::<_, Case>(
            "SELECT * FROM orchepy_cases WHERE workflow_id = $1 AND status = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4"
        )
        .bind(workflow_id)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool)
        .await?;

        Ok(cases)
    }

    pub async fn update_phase(
        &self,
        id: Uuid,
        current_phase: &str,
        previous_phase: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE orchepy_cases SET current_phase = $1, previous_phase = $2, phase_entered_at = NOW(), updated_at = NOW() WHERE id = $3"
        )
        .bind(current_phase)
        .bind(previous_phase)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_data(&self, id: Uuid, data: &serde_json::Value) -> Result<()> {
        sqlx::query("UPDATE orchepy_cases SET data = $1, updated_at = NOW() WHERE id = $2")
            .bind(data)
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    pub async fn update_status(&self, id: Uuid, status: &CaseStatus) -> Result<()> {
        sqlx::query("UPDATE orchepy_cases SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    pub async fn set_field(&self, id: Uuid, path: &str, value: &serde_json::Value) -> Result<()> {
        let query = format!(
            "UPDATE orchepy_cases SET data = jsonb_set(data, '{{{}}}', $1, true), updated_at = NOW() WHERE id = $2",
            path
        );
        sqlx::query(&query)
            .bind(value)
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }

    pub async fn create_history(&self, history: &CaseHistory) -> Result<()> {
        sqlx::query(
            "INSERT INTO orchepy_case_history (id, case_id, from_phase, to_phase, reason, triggered_by, transitioned_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(history.id)
        .bind(history.case_id)
        .bind(&history.from_phase)
        .bind(&history.to_phase)
        .bind(&history.reason)
        .bind(&history.triggered_by)
        .bind(history.transitioned_at)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_history(&self, case_id: Uuid) -> Result<Vec<CaseHistory>> {
        let history = sqlx::query_as::<_, CaseHistory>(
            "SELECT * FROM orchepy_case_history WHERE case_id = $1 ORDER BY transitioned_at DESC"
        )
        .bind(case_id)
        .fetch_all(self.pool)
        .await?;

        Ok(history)
    }

    pub async fn count_by_workflow(&self, workflow_id: Uuid) -> Result<i64> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM orchepy_cases WHERE workflow_id = $1"
        )
        .bind(workflow_id)
        .fetch_one(self.pool)
        .await?;

        Ok(count)
    }
}
