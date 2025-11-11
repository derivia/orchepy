use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use sqlx::QueryBuilder;
use tracing::error;
use uuid::Uuid;

use crate::api::AppState;
use crate::models::case::{Case, CaseHistory, ListCasesQuery, UpdateCaseData};

pub async fn list_cases(
    State(state): State<AppState>,
    Query(query): Query<ListCasesQuery>,
) -> impl IntoResponse {
    let pool = &state.pool;

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let mut query_builder = QueryBuilder::new("SELECT * FROM orchepy_cases WHERE 1=1");

    if let Some(workflow_id) = query.workflow_id {
        query_builder.push(" AND workflow_id = ");
        query_builder.push_bind(workflow_id);
    }

    if let Some(current_phase) = &query.current_phase {
        query_builder.push(" AND current_phase = ");
        query_builder.push_bind(current_phase);
    }

    if let Some(status) = &query.status {
        query_builder.push(" AND status = ");
        query_builder.push_bind(status);
    }

    query_builder.push(" ORDER BY created_at DESC LIMIT ");
    query_builder.push_bind(limit as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset as i64);

    match query_builder.build_query_as::<Case>().fetch_all(pool).await {
        Ok(cases) => (StatusCode::OK, Json(json!(cases))),
        Err(err) => {
            error!("Failed to fetch cases: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch cases"})),
            )
        }
    }
}

pub async fn get_case(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
) -> impl IntoResponse {
    let pool = &state.pool;

    match sqlx::query_as::<_, Case>("SELECT * FROM orchepy_cases WHERE id = $1")
        .bind(case_id)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(case)) => (StatusCode::OK, Json(json!(case))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Case not found"})),
        ),
        Err(err) => {
            error!("Failed to fetch case: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch case"})),
            )
        }
    }
}

pub async fn update_case_data(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
    Json(payload): Json<UpdateCaseData>,
) -> impl IntoResponse {
    let pool = &state.pool;

    match sqlx::query(
        "UPDATE orchepy_cases SET data = $1, updated_at = NOW() WHERE id = $2 RETURNING id",
    )
    .bind(&payload.data)
    .bind(case_id)
    .fetch_optional(pool)
    .await
    {
        Ok(Some(_)) => (StatusCode::OK, Json(json!({"message": "Case data updated"}))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Case not found"})),
        ),
        Err(err) => {
            error!("Failed to update case data: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to update case data"})),
            )
        }
    }
}

pub async fn get_case_history(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
) -> impl IntoResponse {
    let pool = &state.pool;
    match sqlx::query_as::<_, CaseHistory>(
        "SELECT * FROM orchepy_case_history WHERE case_id = $1 ORDER BY transitioned_at DESC",
    )
    .bind(case_id)
    .fetch_all(pool)
    .await
    {
        Ok(history) => (StatusCode::OK, Json(json!(history))),
        Err(err) => {
            error!("Failed to fetch case history: {}", err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch case history"})),
            )
        }
    }
}
