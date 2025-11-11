use crate::models::step::Step;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowTrigger {
    pub event_type: String,
    #[serde(default)]
    pub filters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Flow {
    pub id: Uuid,
    pub name: String,

    #[sqlx(json)]
    pub trigger: FlowTrigger,

    #[sqlx(json)]
    pub steps: Vec<Step>,

    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFlow {
    pub name: String,
    pub trigger: FlowTrigger,
    pub steps: Vec<Step>,
    #[serde(default = "default_active")]
    pub active: bool,
}

fn default_active() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct UpdateFlow {
    pub name: Option<String>,
    pub trigger: Option<FlowTrigger>,
    pub steps: Option<Vec<Step>>,
    pub active: Option<bool>,
}

impl Flow {
    pub fn new(create: CreateFlow) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: create.name,
            trigger: create.trigger,
            steps: create.steps,
            active: create.active,
            created_at: now,
            updated_at: now,
        }
    }
}
