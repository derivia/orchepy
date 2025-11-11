use crate::api::response::ApiError;
use crate::engine::{Executor, Matcher};
use crate::models::{event::CreateEvent, Event, Flow};
use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;

use super::AppState;

#[axum::debug_handler]
pub async fn create_event(
    State(state): State<AppState>,
    Json(payload): Json<CreateEvent>,
) -> Result<Json<Value>, ApiError> {
    let pool = &state.pool;
    info!("Received event via API: {}", payload.event_type);
    let (event_id, execution_ids, matched_count) =
        internal_create_and_trigger_event(&pool, payload).await?;

    Ok(Json(json!({
        "event_id": event_id,
        "executions": execution_ids,
        "matched_flows": matched_count
    })))
}

pub(crate) async fn internal_create_and_trigger_event(
    pool: &PgPool,
    payload: CreateEvent,
) -> Result<(Uuid, Vec<Uuid>, usize), ApiError> {
    let event = Event::new(payload);

    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO orchepy_events (id, event_type, data, metadata, received_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(&event.id)
    .bind(&event.event_type)
    .bind(&event.data)
    .bind(&event.metadata)
    .bind(&event.received_at)
    .execute(pool)
    .await
    {
        error!("Failed to save event: {}", e);
        return Err(ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Failed to save event".to_string(),
        });
    }

    let flows = match sqlx::query_as::<_, Flow>(
        r#"
        SELECT id, name, trigger, steps, active, created_at, updated_at
        FROM orchepy_flows
        WHERE active = true
        "#,
    )
    .fetch_all(pool)
    .await
    {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to load flows: {}", e);
            return Err(ApiError {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to load flows".to_string(),
            });
        }
    };

    let matched = Matcher::match_flows(&event, &flows);
    let matched_count = matched.len();
    info!("Matched {} flow(s) for event {}", matched_count, event.id);

    let executor = Executor::new();
    let mut execution_ids = Vec::new();

    for flow in matched {
        info!("Triggering flow: {} for event {}", flow.name, event.id);

        match executor.execute(flow, &event).await {
            Ok(execution) => {
                execution_ids.push(execution.id);

                if let Err(e) = sqlx::query(
                    r#"
                    INSERT INTO orchepy_executions
                    (id, flow_id, event_id, status, current_step, steps_status, started_at, completed_at, error)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    "#,
                )
                .bind(&execution.id)
                .bind(&execution.flow_id)
                .bind(&execution.event_id)
                .bind(&execution.status)
                .bind(&execution.current_step)
                .bind(&execution.steps_status)
                .bind(&execution.started_at)
                .bind(&execution.completed_at)
                .bind(&execution.error)
                .execute(pool)
                .await
                {
                    error!("Failed to save execution: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to execute flow '{}': {}", flow.name, e);
            }
        }
    }

    Ok((event.id, execution_ids, matched_count))
}
