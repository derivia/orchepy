use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use tracing::{error, info};
use uuid::Uuid;

use crate::api::events::internal_create_and_trigger_event;
use crate::api::AppState;
use crate::models::case::{CaseHistory, MoveCase};
use crate::models::event::CreateEvent;
use crate::repositories::{CaseRepository, WorkflowRepository};

use super::automation_handler::execute_and_apply_automations;

pub async fn move_case(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
    Json(payload): Json<MoveCase>,
) -> impl IntoResponse {
    let pool = &state.pool;
    let webhook_sender = &state.webhook_sender;

    let case_repo = CaseRepository::new(pool);
    let workflow_repo = WorkflowRepository::new(pool);

    let mut case = match case_repo.find_by_id(case_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Case not found"})),
            )
        }
        Err(err) => {
            error!("Failed to fetch case: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch case"})),
            );
        }
    };

    let workflow = match workflow_repo.find_by_id(case.workflow_id).await {
        Ok(Some(wf)) => wf,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Workflow not found"})),
            )
        }
        Err(err) => {
            error!("Failed to fetch workflow: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch workflow"})),
            );
        }
    };

    if !workflow.has_phase(&payload.to_phase) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Phase '{}' not found in workflow", payload.to_phase)})),
        );
    }

    if case.current_phase == payload.to_phase {
        return (
            StatusCode::OK,
            Json(json!({"message": "Case already in target phase", "case": case})),
        );
    }

    let from_phase = case.current_phase.clone();
    case.move_to_phase(payload.to_phase.clone());

    if let Err(err) = case_repo.update_phase(case_id, &case.current_phase, case.previous_phase.as_deref()).await {
        error!("Failed to move case: {}", err);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to move case"})),
        );
    }

    info!(
        "Moved case {} from '{}' to '{}'",
        case_id, from_phase, case.current_phase
    );

    let history = CaseHistory::new(
        case_id,
        Some(from_phase.clone()),
        payload.to_phase.clone(),
        payload.reason,
        payload.triggered_by,
    );

    if let Err(err) = case_repo.create_history(&history).await {
        error!("Failed to create history entry: {}", err);
    }

    if let Some(automations_config) = &workflow.automations {
        let on_exit_automations: Vec<_> = automations_config
            .get_on_exit_automations(&from_phase)
            .into_iter()
            .collect();

        match execute_and_apply_automations(
            pool,
            &on_exit_automations,
            &case,
            Some(&from_phase),
            &workflow,
            "on_exit",
        )
        .await
        {
            Ok(Some(updated_case)) => {
                case = updated_case;
            }
            Ok(None) => {}
            Err(response) => return response,
        }

        let on_enter_automations: Vec<_> = automations_config
            .get_on_enter_automations(&case.current_phase)
            .into_iter()
            .collect();

        match execute_and_apply_automations(
            pool,
            &on_enter_automations,
            &case,
            Some(&from_phase),
            &workflow,
            "on_enter",
        )
        .await
        {
            Ok(Some(updated_case)) => {
                case = updated_case;
            }
            Ok(None) => {}
            Err(response) => return response,
        }
    }

    let pool_clone = pool.clone();
    let case_clone_for_event = case.clone();
    let from_phase_for_event = from_phase.clone();
    tokio::spawn(async move {
        info!("Submitting internal event for case.moved: {}", case_clone_for_event.id);
        let event_payload = CreateEvent {
            event_type: "case.moved".to_string(),
            data: json!({
                "case_id": case_clone_for_event.id,
                "workflow_id": case_clone_for_event.workflow_id,
                "to_phase": case_clone_for_event.current_phase,
                "from_phase": from_phase_for_event,
                "case_data": case_clone_for_event.data,
            }),
            metadata: case_clone_for_event.metadata,
        };

        if let Err(e) = internal_create_and_trigger_event(&pool_clone, event_payload).await {
            error!("Failed to submit internal case.moved event: {}", e.message);
        }
    });

    let webhook_on_move = std::env::var("WEBHOOK_ON_CASE_MOVE")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if webhook_on_move {
        if let Some(webhook_url) = workflow.webhook_url {
            let case_clone = case.clone();
            let webhook_sender_clone = webhook_sender.clone();
            let from_phase_for_webhook = from_phase.clone();
            tokio::spawn(async move {
                if let Err(err) = webhook_sender_clone
                    .send_case_moved_with_retry(&webhook_url, &case_clone, Some(from_phase_for_webhook), 3)
                    .await
                {
                    error!("Failed to send webhook: {}", err);
                }
            });
        }
    }

    (StatusCode::OK, Json(json!(case)))
}
