pub mod cases;
pub mod events;
pub mod executions;
pub mod flows;
pub mod health;
pub mod response;
pub mod ui;
pub mod workflows;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use sqlx::PgPool;

use crate::services::WebhookSender;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub webhook_sender: WebhookSender,
}

pub fn build_router(pool: PgPool, webhook_sender: WebhookSender) -> Router {
    let state = AppState {
        pool,
        webhook_sender,
    };

    Router::new()
        .route("/", get(ui::dashboard_handler))
        .route("/health", get(health::health_check))
        .route("/workflows", get(workflows::list_workflows))
        .route("/workflows", post(workflows::create_workflow))
        .route("/workflows/{id}", get(workflows::get_workflow))
        .route("/workflows/{id}", put(workflows::update_workflow))
        .route("/workflows/{id}", delete(workflows::delete_workflow))
        .route("/cases", get(cases::list_cases))
        .route("/cases", post(cases::create_case))
        .route("/cases/{id}", get(cases::get_case))
        .route("/cases/{id}/data", patch(cases::update_case_data))
        .route("/cases/{id}/move", put(cases::move_case))
        .route("/cases/{id}/history", get(cases::get_case_history))
        .route("/events", post(events::create_event))
        .route("/flows", get(flows::list_flows))
        .route("/flows", post(flows::create_flow))
        .route("/flows/{id}", get(flows::get_flow))
        .route("/flows/{id}", put(flows::update_flow))
        .route("/flows/{id}", delete(flows::delete_flow))
        .route("/executions", get(executions::list_executions))
        .route("/executions/{id}", get(executions::get_execution))
        .with_state(state)
}
