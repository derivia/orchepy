use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use tracing::{error, info};

use crate::api::events::internal_create_and_trigger_event;
use crate::api::AppState;
use crate::models::case::{Case, CaseHistory, CreateCase};
use crate::models::event::CreateEvent;
use crate::repositories::{CaseRepository, WorkflowRepository};

use super::automation_handler::execute_and_apply_automations;

pub async fn create_case(
    State(state): State<AppState>,
    Json(payload): Json<CreateCase>,
) -> impl IntoResponse {
    let pool = &state.pool;
    let webhook_sender = &state.webhook_sender;

    let workflow_repo = WorkflowRepository::new(pool);
    let workflow = match workflow_repo.find_active_by_id(payload.workflow_id).await {
        Ok(Some(wf)) => wf,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Workflow not found or inactive"})),
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

    let initial_phase = payload
        .initial_phase
        .unwrap_or(workflow.initial_phase.clone());

    if !workflow.has_phase(&initial_phase) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Phase '{}' not found in workflow", initial_phase)})),
        );
    }

    let mut case = Case::new(
        payload.workflow_id,
        initial_phase.clone(),
        payload.data,
        payload.metadata,
    );

    let case_repo = CaseRepository::new(pool);

    if let Err(err) = case_repo.create(&case).await {
        error!("Failed to create case: {}", err);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create case"})),
        );
    }

    info!("Created case {} in phase '{}'", case.id, case.current_phase);

    let history = CaseHistory::new(
        case.id,
        None,
        initial_phase.clone(),
        Some("Case created".to_string()),
        Some("system".to_string()),
    );

    if let Err(err) = case_repo.create_history(&history).await {
        error!("Failed to create history entry: {}", err);
    }

    if let Some(automations_config) = &workflow.automations {
        let automations_to_run: Vec<_> = automations_config
            .get_on_enter_automations(&case.current_phase)
            .into_iter()
            .collect();

        match execute_and_apply_automations(
            pool,
            &automations_to_run,
            &case,
            None,
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
    tokio::spawn(async move {
        info!("Submitting internal event for case.created: {}", case_clone_for_event.id);
        let event_payload = CreateEvent {
            event_type: "case.created".to_string(),
            data: json!({
                "case_id": case_clone_for_event.id,
                "workflow_id": case_clone_for_event.workflow_id,
                "to_phase": case_clone_for_event.current_phase,
                "from_phase": null,
                "case_data": case_clone_for_event.data,
            }),
            metadata: case_clone_for_event.metadata,
        };

        if let Err(e) = internal_create_and_trigger_event(&pool_clone, event_payload).await {
            error!("Failed to submit internal case.created event: {}", e.message);
        }
    });

    let webhook_on_create = std::env::var("WEBHOOK_ON_CASE_CREATE")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if webhook_on_create {
        if let Some(webhook_url) = workflow.webhook_url {
            let case_clone = case.clone();
            let webhook_sender_clone = webhook_sender.clone();
            tokio::spawn(async move {
                if let Err(err) = webhook_sender_clone
                    .send_case_moved_with_retry(&webhook_url, &case_clone, None, 3)
                    .await
                {
                    error!("Failed to send webhook: {}", err);
                }
            });
        }
    }

    (StatusCode::CREATED, Json(json!(case)))
}
