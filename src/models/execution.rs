use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Execution {
    pub id: Uuid,
    #[sqlx(rename = "flow_id")]
    pub flow_id: Uuid,
    pub event_id: Uuid,

    pub status: ExecutionStatus,

    pub current_step: Option<String>,

    #[sqlx(json)]
    pub steps_status: serde_json::Value,

    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,

    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "execution_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Pending,

    Running,

    Completed,

    Failed,

    Retrying,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepStatus {
    pub status: StepExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub attempts: u32,
    pub response: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepExecutionStatus {
    Running,
    Completed,
    Failed,
    Skipped,
}

impl Execution {
    pub fn new(flow_id: Uuid, event_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            flow_id,
            event_id,
            status: ExecutionStatus::Pending,
            current_step: None,
            steps_status: serde_json::json!({}),
            started_at: Utc::now(),
            completed_at: None,
            error: None,
        }
    }
}
