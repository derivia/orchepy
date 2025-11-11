use axum::http::StatusCode;
use axum::Json;
use serde_json::json;
use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;

use crate::engine::AutomationExecutor;
use crate::models::automation::{AutomationResult, PhaseAutomation};
use crate::models::case::{Case, CaseHistory};
use crate::models::{CaseModification, Workflow};

pub async fn apply_automation_modifications(
    pool: &PgPool,
    case_id: Uuid,
    workflow: &Workflow,
    automation_result: AutomationResult,
    automation_type: &str,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if automation_result.modifications.is_empty() {
        return Ok(());
    }

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to start transaction for {} automation modifications: {}", automation_type, e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to start transaction"}))));
        }
    };

    let mut current_phase_query = sqlx::query_scalar::<_, String>(
        "SELECT current_phase FROM orchepy_cases WHERE id = $1"
    )
    .bind(case_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        error!("Failed to fetch current phase: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Failed to fetch case state"})))
    })?;

    for modification in automation_result.modifications {
        match modification {
            CaseModification::MoveToPhase { phase } => {
                if !workflow.phases.contains(&phase) {
                    error!("{} automation tried to move case {} to non-existent phase: {}", automation_type, case_id, phase);
                    continue;
                }

                let from_phase = current_phase_query.clone();

                if let Err(e) = sqlx::query(
                    "UPDATE orchepy_cases SET current_phase = $1, previous_phase = $2, phase_entered_at = NOW(), updated_at = NOW() WHERE id = $3"
                )
                .bind(&phase)
                .bind(&from_phase)
                .bind(case_id)
                .execute(&mut *tx)
                .await {
                    error!("Failed to apply {} MoveToPhase automation for case {}: {}", automation_type, case_id, e);
                } else {
                    info!("{} automation moved case {} from '{}' to '{}'", automation_type, case_id, from_phase, phase);

                    let history = CaseHistory::new(
                        case_id,
                        Some(from_phase),
                        phase.clone(),
                        Some(format!("{} automation", automation_type)),
                        Some("system".to_string()),
                    );

                    if let Err(err) = sqlx::query(
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
                    .execute(&mut *tx)
                    .await
                    {
                        error!("Failed to create history entry for {} automation: {}", automation_type, err);
                    }

                    current_phase_query = phase;
                }
            }
            CaseModification::SetField { field, value } => {
                let parts: Vec<&str> = field.split('.').collect();
                if parts.is_empty() {
                    error!("Invalid field path: {}", field);
                    continue;
                }

                match parts[0] {
                    "data" => {
                        let path = parts[1..].join(".");
                        if path.is_empty() {
                            error!("Invalid data field path: {}", field);
                            continue;
                        }

                        if let Err(e) = sqlx::query(
                            "UPDATE orchepy_cases SET data = jsonb_set(data, $1, $2, true), updated_at = NOW() WHERE id = $3"
                        )
                        .bind(format!("{{{}}}", path))
                        .bind(&value)
                        .bind(case_id)
                        .execute(&mut *tx)
                        .await {
                            error!("Failed to apply {} SetField automation for case {}: {}", automation_type, case_id, e);
                        } else {
                            info!("{} automation set field '{}' to {:?} for case {}", automation_type, field, value, case_id);
                        }
                    }
                    _ => {
                        error!("Unsupported field path for automation: {}", field);
                    }
                }
            }
        }
    }

    if let Err(e) = tx.commit().await {
        error!("Failed to commit {} automation modifications: {}", automation_type, e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to commit {} automation modifications", automation_type)}))));
    }

    Ok(())
}

pub async fn execute_and_apply_automations(
    pool: &PgPool,
    automations: &[&PhaseAutomation],
    case: &Case,
    from_phase: Option<&str>,
    workflow: &Workflow,
    automation_type: &str,
) -> Result<Option<Case>, (StatusCode, Json<serde_json::Value>)> {
    if automations.is_empty() {
        return Ok(None);
    }

    let executor = AutomationExecutor::new();

    match executor.execute_automations(automations, case, from_phase).await {
        Ok(automation_result) => {
            if !automation_result.modifications.is_empty() {
                apply_automation_modifications(pool, case.id, workflow, automation_result, automation_type).await?;

                match sqlx::query_as::<_, Case>("SELECT * FROM orchepy_cases WHERE id = $1")
                    .bind(case.id)
                    .fetch_one(pool)
                    .await
                {
                    Ok(updated_case) => Ok(Some(updated_case)),
                    Err(e) => {
                        error!("Failed to re-fetch case after {} automation modifications: {}", automation_type, e);
                        Ok(None)
                    }
                }
            } else {
                Ok(None)
            }
        }
        Err(e) => {
            error!("Failed to execute {} automations: {}", automation_type, e);
            Ok(None)
        }
    }
}
