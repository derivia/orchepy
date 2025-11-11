use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Workflow;

pub struct WorkflowRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> WorkflowRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, workflow: &Workflow) -> Result<()> {
        sqlx::query(
            "INSERT INTO orchepy_workflows (id, name, phases, initial_phase, webhook_url, description, automations, sla_config, active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
        )
        .bind(workflow.id)
        .bind(&workflow.name)
        .bind(serde_json::to_value(&workflow.phases)?)
        .bind(&workflow.initial_phase)
        .bind(&workflow.webhook_url)
        .bind(&workflow.description)
        .bind(serde_json::to_value(&workflow.automations)?)
        .bind(serde_json::to_value(&workflow.sla_config)?)
        .bind(workflow.active)
        .bind(workflow.created_at)
        .bind(workflow.updated_at)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Workflow>> {
        let workflow = sqlx::query_as::<_, Workflow>(
            "SELECT * FROM orchepy_workflows WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(workflow)
    }

    pub async fn find_active_by_id(&self, id: Uuid) -> Result<Option<Workflow>> {
        let workflow = sqlx::query_as::<_, Workflow>(
            "SELECT * FROM orchepy_workflows WHERE id = $1 AND active = true"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?;

        Ok(workflow)
    }

    pub async fn list_all(&self) -> Result<Vec<Workflow>> {
        let workflows = sqlx::query_as::<_, Workflow>(
            "SELECT * FROM orchepy_workflows ORDER BY created_at DESC"
        )
        .fetch_all(self.pool)
        .await?;

        Ok(workflows)
    }

    pub async fn list_active(&self) -> Result<Vec<Workflow>> {
        let workflows = sqlx::query_as::<_, Workflow>(
            "SELECT * FROM orchepy_workflows WHERE active = true ORDER BY created_at DESC"
        )
        .fetch_all(self.pool)
        .await?;

        Ok(workflows)
    }

    pub async fn update(&self, workflow: &Workflow) -> Result<()> {
        sqlx::query(
            "UPDATE orchepy_workflows SET name = $1, phases = $2, initial_phase = $3, webhook_url = $4, description = $5, automations = $6, sla_config = $7, active = $8, updated_at = $9 WHERE id = $10"
        )
        .bind(&workflow.name)
        .bind(serde_json::to_value(&workflow.phases)?)
        .bind(&workflow.initial_phase)
        .bind(&workflow.webhook_url)
        .bind(&workflow.description)
        .bind(serde_json::to_value(&workflow.automations)?)
        .bind(serde_json::to_value(&workflow.sla_config)?)
        .bind(workflow.active)
        .bind(workflow.updated_at)
        .bind(workflow.id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM orchepy_workflows WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn set_active(&self, id: Uuid, active: bool) -> Result<()> {
        sqlx::query("UPDATE orchepy_workflows SET active = $1, updated_at = NOW() WHERE id = $2")
            .bind(active)
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }
}
