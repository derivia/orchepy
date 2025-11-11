use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::{json, to_value}; 
use tracing::{error, info};
use uuid::Uuid;

use crate::api::{response::ApiError, AppState}; 
use crate::models::workflow::{CreateWorkflow, UpdateWorkflow, Workflow};

pub async fn create_workflow(
    State(state): State<AppState>,
    Json(payload): Json<CreateWorkflow>,
) -> Result<impl IntoResponse, ApiError> {
    
    let pool = &state.pool;
    let workflow = match Workflow::new(payload) {
        Ok(wf) => wf,
        
        Err(err) => return Ok((StatusCode::BAD_REQUEST, Json(json!({"error": err})))),
    };

    match sqlx::query(
        "INSERT INTO orchepy_workflows (id, name, phases, initial_phase, webhook_url, description, active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
    )
    .bind(workflow.id)
    .bind(&workflow.name)
    .bind(to_value(&workflow.phases)?) 
    .bind(&workflow.initial_phase)
    .bind(&workflow.webhook_url)
    .bind(&workflow.description)
    .bind(workflow.active)
    .bind(workflow.created_at)
    .bind(workflow.updated_at)
    .execute(pool)
    .await
    {
        Ok(_) => {
            info!("Created workflow {} ({})", workflow.id, workflow.name);
            Ok((StatusCode::CREATED, Json(json!(workflow)))) 
        }
        Err(err) => {
            error!("Failed to create workflow: {}", err);
            Err(ApiError { 
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: format!("Failed to create workflow: {}", err),
            })
        }
    }
}

pub async fn get_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    
    let pool = &state.pool;
    match sqlx::query_as::<_, Workflow>("SELECT * FROM orchepy_workflows WHERE id = $1")
        .bind(workflow_id)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(workflow)) => Ok((StatusCode::OK, Json(json!(workflow)))), 
        Ok(None) => Ok((
            
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Workflow not found"})),
        )),
        Err(err) => {
            error!("Failed to fetch workflow: {}", err);
            Err(ApiError {
                
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to fetch workflow".to_string(),
            })
        }
    }
}

pub async fn list_workflows(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    
    let pool = &state.pool;
    match sqlx::query_as::<_, Workflow>("SELECT * FROM orchepy_workflows ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
    {
        Ok(workflows) => Ok((StatusCode::OK, Json(json!(workflows)))), 
        Err(err) => {
            error!("Failed to list workflows: {}", err);
            Err(ApiError {
                
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to list workflows".to_string(),
            })
        }
    }
}

pub async fn update_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
    Json(payload): Json<UpdateWorkflow>,
) -> Result<impl IntoResponse, ApiError> {
    
    let pool = &state.pool;

    let mut workflow = match sqlx::query_as::<_, Workflow>("SELECT * FROM orchepy_workflows WHERE id = $1")
        .bind(workflow_id)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(wf)) => wf,
        Ok(None) => {
            return Ok((
                
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Workflow not found"})),
            ));
        }
        Err(err) => {
            error!("Failed to fetch workflow: {}", err);
            return Err(ApiError {
                
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to fetch workflow".to_string(),
            });
        }
    };

    if let Some(name) = payload.name {
        workflow.name = name;
    }
    if let Some(phases) = payload.phases {
        if phases.is_empty() {
            return Ok((
                
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Phases list cannot be empty"})),
            ));
        }
        workflow.phases = phases;
    }
    if let Some(initial_phase) = payload.initial_phase {
        if !workflow.has_phase(&initial_phase) {
            return Ok((
                
                StatusCode::BAD_REQUEST,
                Json(
                    json!({"error": format!("Initial phase '{}' must be in phases list", initial_phase)}),
                ),
            ));
        }
        workflow.initial_phase = initial_phase;
    }
    if let Some(webhook_url) = payload.webhook_url {
        workflow.webhook_url = Some(webhook_url);
    }
    if let Some(description) = payload.description {
        workflow.description = Some(description);
    }
    if let Some(active) = payload.active {
        workflow.active = active;
    }
    if let Some(automations) = payload.automations {
        workflow.automations = Some(automations);
    }
    if let Some(sla_config) = payload.sla_config {
        workflow.sla_config = Some(sla_config);
    }

    workflow.updated_at = chrono::Utc::now();

    match sqlx::query(
        "UPDATE orchepy_workflows SET name = $1, phases = $2, initial_phase = $3, webhook_url = $4, description = $5, active = $6, automations = $7, sla_config = $8, updated_at = $9 WHERE id = $10"
    )
    .bind(&workflow.name)
    .bind(to_value(&workflow.phases)?)
    .bind(&workflow.initial_phase)
    .bind(&workflow.webhook_url)
    .bind(&workflow.description)
    .bind(workflow.active)
    .bind(to_value(&workflow.automations)?)
    .bind(to_value(&workflow.sla_config)?)
    .bind(workflow.updated_at)
    .bind(workflow_id)
    .execute(pool)
    .await
    {
        Ok(_) => {
            info!("Updated workflow {}", workflow_id);
            Ok((StatusCode::OK, Json(json!(workflow)))) 
        }
        Err(err) => {
            error!("Failed to update workflow: {}", err);
            Err(ApiError { 
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to update workflow".to_string(),
            })
        }
    }
}

pub async fn delete_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    
    let pool = &state.pool;
    match sqlx::query("DELETE FROM orchepy_workflows WHERE id = $1")
        .bind(workflow_id)
        .execute(pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                info!("Deleted workflow {}", workflow_id);
                Ok((StatusCode::NO_CONTENT, Json(json!({})))) 
            } else {
                Ok((
                    
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "Workflow not found"})),
                ))
            }
        }
        Err(err) => {
            error!("Failed to delete workflow: {}", err);
            Err(ApiError {
                
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to delete workflow".to_string(),
            })
        }
    }
}
