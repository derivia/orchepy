use crate::api::response::ApiError;
use crate::models::execution::Execution;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use tracing::error;
use uuid::Uuid;

use super::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    status: Option<String>,
    flow_id: Option<Uuid>,
    limit: Option<i64>,
}

pub async fn list_executions(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Execution>>, ApiError> {
    let pool = &state.pool;
    let mut sql = String::from(
        r#"
        SELECT id, flow_id, event_id, status, current_step, steps_status,
               started_at, completed_at, error
        FROM orchepy_executions
        WHERE 1=1
        "#,
    );

    let mut params: Vec<String> = Vec::new();

    if let Some(status) = &query.status {
        params.push(format!("status = '{}'", status));
    }

    if let Some(flow_id) = query.flow_id {
        params.push(format!("flow_id = '{}'", flow_id));
    }

    if !params.is_empty() {
        sql.push_str(" AND ");
        sql.push_str(&params.join(" AND "));
    }

    sql.push_str(" ORDER BY started_at DESC");

    if let Some(limit) = query.limit {
        sql.push_str(&format!(" LIMIT {}", limit));
    } else {
        sql.push_str(" LIMIT 100");
    }

    match sqlx::query_as::<_, Execution>(&sql).fetch_all(pool).await {
        Ok(executions) => Ok(Json(executions)),
        Err(e) => {
            error!("Failed to list executions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR.into())
        }
    }
}

pub async fn get_execution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Execution>, ApiError> {
    let pool = &state.pool;
    match sqlx::query_as::<_, Execution>(
        r#"
        SELECT id, flow_id, event_id, status, current_step, steps_status,
               started_at, completed_at, error
        FROM orchepy_executions
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    {
        Ok(execution) => Ok(Json(execution)),
        Err(sqlx::Error::RowNotFound) => Err(StatusCode::NOT_FOUND.into()),
        Err(e) => {
            error!("Failed to get execution: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR.into())
        }
    }
}

pub async fn retry_execution(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    Err(StatusCode::NOT_IMPLEMENTED.into())
}
