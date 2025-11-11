use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::automation::{WorkflowAutomations, WorkflowSlaConfig};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Workflow {
    pub id: Uuid,
    pub name: String,

    #[sqlx(json)]
    pub phases: Vec<String>,

    pub initial_phase: String,

    pub webhook_url: Option<String>,

    pub active: bool,

    pub description: Option<String>,

    #[sqlx(json)]
    pub automations: Option<WorkflowAutomations>,

    #[sqlx(json)]
    pub sla_config: Option<WorkflowSlaConfig>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkflow {
    pub name: String,
    pub phases: Vec<String>,
    pub initial_phase: String,
    pub webhook_url: Option<String>,
    pub description: Option<String>,
    pub automations: Option<WorkflowAutomations>,
    pub sla_config: Option<WorkflowSlaConfig>,
    #[serde(default = "default_active")]
    pub active: bool,
}

fn default_active() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkflow {
    pub name: Option<String>,
    pub phases: Option<Vec<String>>,
    pub initial_phase: Option<String>,
    pub webhook_url: Option<String>,
    pub description: Option<String>,
    pub automations: Option<WorkflowAutomations>,
    pub sla_config: Option<WorkflowSlaConfig>,
    pub active: Option<bool>,
}

impl Workflow {
    pub fn new(create: CreateWorkflow) -> Result<Self, String> {
        if !create.phases.contains(&create.initial_phase) {
            return Err(format!(
                "Initial phase '{}' must be in phases list",
                create.initial_phase
            ));
        }

        if create.phases.is_empty() {
            return Err("Phases list cannot be empty".to_string());
        }

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            name: create.name,
            phases: create.phases,
            initial_phase: create.initial_phase,
            webhook_url: create.webhook_url,
            description: create.description,
            automations: create.automations,
            sla_config: create.sla_config,
            active: create.active,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn has_phase(&self, phase_name: &str) -> bool {
        self.phases.iter().any(|p| p == phase_name)
    }

    pub fn phase_index(&self, phase_name: &str) -> Option<usize> {
        self.phases.iter().position(|p| p == phase_name)
    }

    pub fn next_phase(&self, current_phase: &str) -> Option<String> {
        self.phase_index(current_phase)
            .and_then(|idx| self.phases.get(idx + 1))
            .cloned()
    }

    pub fn previous_phase(&self, current_phase: &str) -> Option<String> {
        self.phase_index(current_phase)
            .and_then(|idx| {
                if idx > 0 {
                    self.phases.get(idx - 1)
                } else {
                    None
                }
            })
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_creation() {
        let create = CreateWorkflow {
            name: "Invoice Processing".to_string(),
            phases: vec![
                "OCR".to_string(),
                "Validation".to_string(),
                "SAP".to_string(),
                "Approved".to_string(),
            ],
            initial_phase: "OCR".to_string(),
            webhook_url: Some("https://backend.com/webhook".to_string()),
            description: Some("Invoice workflow".to_string()),
            automations: None,
            sla_config: None,
            active: true,
        };

        let workflow = Workflow::new(create).unwrap();
        assert_eq!(workflow.name, "Invoice Processing");
        assert_eq!(workflow.phases.len(), 4);
        assert_eq!(workflow.initial_phase, "OCR");
    }

    #[test]
    fn test_invalid_initial_phase() {
        let create = CreateWorkflow {
            name: "Test".to_string(),
            phases: vec!["A".to_string(), "B".to_string()],
            initial_phase: "C".to_string(),
            webhook_url: None,
            description: None,
            automations: None,
            sla_config: None,
            active: true,
        };

        let result = Workflow::new(create);
        assert!(result.is_err());
    }

    #[test]
    fn test_phase_navigation() {
        let workflow = Workflow {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            phases: vec![
                "First".to_string(),
                "Second".to_string(),
                "Third".to_string(),
            ],
            initial_phase: "First".to_string(),
            webhook_url: None,
            active: true,
            description: None,
            automations: None,
            sla_config: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(workflow.next_phase("First"), Some("Second".to_string()));
        assert_eq!(workflow.next_phase("Second"), Some("Third".to_string()));
        assert_eq!(workflow.next_phase("Third"), None);

        assert_eq!(workflow.previous_phase("Third"), Some("Second".to_string()));
        assert_eq!(workflow.previous_phase("Second"), Some("First".to_string()));
        assert_eq!(workflow.previous_phase("First"), None);
    }
}
