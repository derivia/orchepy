use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Event {
    pub id: Uuid,

    #[sqlx(rename = "event_type")]
    pub event_type: String,

    #[sqlx(json)]
    pub data: serde_json::Value,

    #[sqlx(json)]
    pub metadata: Option<serde_json::Value>,

    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEvent {
    pub event_type: String,
    pub data: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
}

impl Event {
    pub fn new(create: CreateEvent) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: create.event_type,
            data: create.data,
            metadata: create.metadata,
            received_at: Utc::now(),
        }
    }
}
