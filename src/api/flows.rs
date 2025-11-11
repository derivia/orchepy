use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use tracing::{error, info};
use uuid::Uuid;

use crate::api::{response::ApiError, AppState}; 
use crate::models::flow::{CreateFlow, Flow, UpdateFlow};

pub async fn create_flow(
    State(state): State<AppState>,
    Json(payload): Json<CreateFlow>,
) -> Result<impl IntoResponse, ApiError> {
    let pool = &state.pool;
    let flow = Flow::new(payload);

    match sqlx::query(
        "INSERT INTO orchepy_flows (id, name, trigger, steps, active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(flow.id)
    .bind(&flow.name)
    .bind(serde_json::to_value(&flow.trigger)?) 
    .bind(serde_json::to_value(&flow.steps)?) 
    .bind(flow.active)
    .bind(flow.created_at)
    .bind(flow.updated_at)
    .execute(pool)
    .await
    {
        Ok(_) => {
            info!("Created flow {} ({})", flow.id, flow.name);
            Ok((StatusCode::CREATED, Json(json!(flow)))) 
        }
        Err(err) => {
            error!("Failed to create flow: {}", err);
            Err(ApiError {
                
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to create flow".to_string(),
            })
        }
    }
}

pub async fn get_flow(
    State(state): State<AppState>,
    Path(flow_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    
    let pool = &state.pool;
    match sqlx::query_as::<_, Flow>("SELECT * FROM orchepy_flows WHERE id = $1")
        .bind(flow_id)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(flow)) => Ok((StatusCode::OK, Json(json!(flow)))), 
        Ok(None) => Ok((
            
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Flow not found"})),
        )),
        Err(err) => {
            error!("Failed to fetch flow: {}", err);
            Err(ApiError {
                
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to fetch flow".to_string(),
            })
        }
    }
}

pub async fn list_flows(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    
    let pool = &state.pool;
    match sqlx::query_as::<_, Flow>("SELECT * FROM orchepy_flows ORDER BY created_at DESC")
        .fetch_all(pool)
        .await
    {
        Ok(flows) => Ok((StatusCode::OK, Json(json!(flows)))), 
        Err(err) => {
            error!("Failed to list flows: {}", err);
            Err(ApiError {
                
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to list flows".to_string(),
            })
        }
    }
}

pub async fn update_flow(
    State(state): State<AppState>,
    Path(flow_id): Path<Uuid>,
    Json(payload): Json<UpdateFlow>,
) -> Result<impl IntoResponse, ApiError> {
    let pool = &state.pool;

    let mut flow = match sqlx::query_as::<_, Flow>("SELECT * FROM orchepy_flows WHERE id = $1")
        .bind(flow_id)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(f)) => f,
        Ok(None) => {
            return Ok((
                
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Flow not found"})),
            ));
        }
        Err(err) => {
            error!("Failed to fetch flow: {}", err);
            return Err(ApiError {
                
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to fetch flow".to_string(),
            });
        }
    };

    if let Some(name) = payload.name {
        flow.name = name;
    }
    if let Some(trigger) = payload.trigger {
        flow.trigger = trigger;
    }
    if let Some(steps) = payload.steps {
        flow.steps = steps;
    }
    if let Some(active) = payload.active {
        flow.active = active;
    }

    flow.updated_at = chrono::Utc::now();

    match sqlx::query(
        "UPDATE orchepy_flows SET name = $1, trigger = $2, steps = $3, active = $4, updated_at = $5 WHERE id = $6"
    )
    .bind(&flow.name)
    .bind(serde_json::to_value(&flow.trigger)?) 
    .bind(serde_json::to_value(&flow.steps)?)   
    .bind(flow.active)
    .bind(flow.updated_at)
    .bind(flow_id)
    .execute(pool)
    .await
    {
        Ok(_) => {
            info!("Updated flow {}", flow_id);
            Ok((StatusCode::OK, Json(json!(flow)))) 
        }
        Err(err) => {
            error!("Failed to update flow: {}", err);
            Err(ApiError { 
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to update flow".to_string(),
            })
        }
    }
}

pub async fn delete_flow(
    State(state): State<AppState>,
    Path(flow_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    
    let pool = &state.pool;
    match sqlx::query("DELETE FROM orchepy_flows WHERE id = $1")
        .bind(flow_id)
        .execute(pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                info!("Deleted flow {}", flow_id);
                Ok((StatusCode::NO_CONTENT, Json(json!({})))) 
            } else {
                Ok((
                    
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "Flow not found"})),
                ))
            }
        }
        Err(err) => {
            error!("Failed to delete flow: {}", err);
            Err(ApiError {
                
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to delete flow".to_string(),
            })
        }
    }
}
