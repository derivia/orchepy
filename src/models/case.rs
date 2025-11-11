use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub id: Uuid,
    pub workflow_id: Uuid,

    pub current_phase: String,

    pub previous_phase: Option<String>,

    pub data: serde_json::Value,

    pub status: CaseStatus,

    pub metadata: Option<serde_json::Value>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,

    pub phase_entered_at: DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for Case {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        Ok(Self {
            id: row.try_get("id")?,
            workflow_id: row.try_get("workflow_id")?,
            current_phase: row.try_get("current_phase")?,
            previous_phase: row.try_get("previous_phase")?,
            data: row.try_get("data")?,
            status: row.try_get("status")?,
            metadata: row.try_get("metadata").ok(),
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            completed_at: row.try_get("completed_at")?,
            phase_entered_at: row.try_get("phase_entered_at")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "case_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum CaseStatus {
    Active,
    Completed,
    Failed,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CaseHistory {
    pub id: Uuid,
    pub case_id: Uuid,

    pub from_phase: Option<String>,

    pub to_phase: String,

    pub reason: Option<String>,

    pub triggered_by: Option<String>,

    pub transitioned_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCase {
    pub workflow_id: Uuid,

    pub data: serde_json::Value,

    pub metadata: Option<serde_json::Value>,

    pub initial_phase: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCaseData {
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct MoveCase {
    pub to_phase: String,

    pub reason: Option<String>,

    pub triggered_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListCasesQuery {
    pub workflow_id: Option<Uuid>,
    pub current_phase: Option<String>,
    pub status: Option<CaseStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Case {
    pub fn new(
        workflow_id: Uuid,
        initial_phase: String,
        data: serde_json::Value,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workflow_id,
            current_phase: initial_phase,
            previous_phase: None,
            data,
            status: CaseStatus::Active,
            metadata,
            created_at: now,
            updated_at: now,
            completed_at: None,
            phase_entered_at: now,
        }
    }

    pub fn move_to_phase(&mut self, new_phase: String) {
        self.previous_phase = Some(self.current_phase.clone());
        self.current_phase = new_phase;
        self.updated_at = Utc::now();
        self.phase_entered_at = Utc::now();
    }

    pub fn update_data(&mut self, new_data: serde_json::Value) {
        if let serde_json::Value::Object(ref mut map) = self.data {
            if let serde_json::Value::Object(new_map) = new_data {
                for (key, value) in new_map {
                    map.insert(key, value);
                }
            }
        }
        self.updated_at = Utc::now();
    }

    pub fn complete(&mut self) {
        self.status = CaseStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn fail(&mut self) {
        self.status = CaseStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

impl CaseHistory {
    pub fn new(
        case_id: Uuid,
        from_phase: Option<String>,
        to_phase: String,
        reason: Option<String>,
        triggered_by: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            case_id,
            from_phase,
            to_phase,
            reason,
            triggered_by,
            transitioned_at: Utc::now(),
        }
    }
}
