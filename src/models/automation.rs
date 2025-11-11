use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum CaseModification {
    MoveToPhase { phase: String },
    SetField { field: String, value: serde_json::Value },
}

#[derive(Debug, Clone, Default)]
pub struct AutomationResult {
    pub modifications: Vec<CaseModification>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowSlaConfig {
    #[serde(flatten)]
    pub phase_slas: HashMap<String, PhaseSla>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseSla {
    pub hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AutomationTrigger {
    OnEnter,
    OnExit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnError {
    Stop,
    Continue,
}

impl Default for OnError {
    fn default() -> Self {
        Self::Stop
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    #[serde(default = "default_delay_ms")]
    pub delay_ms: u64,
}

fn default_max_attempts() -> u32 {
    3
}

fn default_delay_ms() -> u64 {
    1000
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_attempts: default_max_attempts(),
            delay_ms: default_delay_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AutomationAction {
    Webhook {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,

        url: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        method: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        fields: Option<Vec<String>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        use_response_from: Option<String>,

        #[serde(default)]
        retry: RetryConfig,

        #[serde(default)]
        on_error: OnError,
    },

    Delay {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,

        duration_ms: u64,
    },

    Conditional {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,

        #[serde(flatten)]
        condition: Condition,

        then: Vec<AutomationAction>,

        #[serde(skip_serializing_if = "Option::is_none")]
        r#else: Option<Vec<AutomationAction>>,
    },

    MoveToPhase {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,

        phase: String,
    },

    SetField {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,

        field: String,

        value: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    Simple {
        field: String,
        operator: String,
        value: serde_json::Value,
    },
    Complex {
        operator: LogicalOperator,
        conditions: Vec<SimpleCondition>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogicalOperator {
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleCondition {
    pub field: String,
    #[serde(rename = "op")]
    pub operator: String,
    pub value: serde_json::Value,
}

impl AutomationAction {
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Webhook { id, .. } => id.as_deref(),
            _ => None,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Webhook { name, .. } => name.as_deref(),
            Self::Delay { name, .. } => name.as_deref(),
            Self::Conditional { name, .. } => name.as_deref(),
            Self::MoveToPhase { name, .. } => name.as_deref(),
            Self::SetField { name, .. } => name.as_deref(),
        }
    }

    pub fn on_error(&self) -> OnError {
        match self {
            Self::Webhook { on_error, .. } => on_error.clone(),
            _ => OnError::Continue,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseAutomation {
    pub trigger: AutomationTrigger,

    pub phase: String,

    pub actions: Vec<AutomationAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowAutomations {
    #[serde(default)]
    pub automations: Vec<PhaseAutomation>,
}

impl WorkflowAutomations {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_on_enter_automations(&self, phase: &str) -> Vec<&PhaseAutomation> {
        self.automations
            .iter()
            .filter(|a| a.trigger == AutomationTrigger::OnEnter && a.phase == phase)
            .collect()
    }

    pub fn get_on_exit_automations(&self, phase: &str) -> Vec<&PhaseAutomation> {
        self.automations
            .iter()
            .filter(|a| a.trigger == AutomationTrigger::OnExit && a.phase == phase)
            .collect()
    }

    pub fn get_actions(&self, trigger: AutomationTrigger, phase: &str) -> Vec<&AutomationAction> {
        self.automations
            .iter()
            .filter(|a| a.trigger == trigger && a.phase == phase)
            .flat_map(|a| &a.actions)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automation_filtering() {
        let automations = WorkflowAutomations {
            automations: vec![
                PhaseAutomation {
                    trigger: AutomationTrigger::OnEnter,
                    phase: "Qualified".to_string(),
                    actions: vec![AutomationAction::Webhook {
                        id: None,
                        name: Some("Notify CRM".to_string()),
                        url: "https://example.com/webhook".to_string(),
                        method: Some("POST".to_string()),
                        headers: None,
                        fields: None,
                        use_response_from: None,
                        retry: RetryConfig::default(),
                        on_error: OnError::Stop,
                    }],
                },
                PhaseAutomation {
                    trigger: AutomationTrigger::OnExit,
                    phase: "Qualified".to_string(),
                    actions: vec![AutomationAction::Delay {
                        name: None,
                        duration_ms: 1000,
                    }],
                },
            ],
        };

        let on_enter = automations.get_on_enter_automations("Qualified");
        assert_eq!(on_enter.len(), 1);

        let on_exit = automations.get_on_exit_automations("Qualified");
        assert_eq!(on_exit.len(), 1);
    }

    #[test]
    fn test_json_serialization() {
        let automation = PhaseAutomation {
            trigger: AutomationTrigger::OnEnter,
            phase: "OCR".to_string(),
            actions: vec![
                AutomationAction::Webhook {
                    id: Some("ocr_result".to_string()),
                    name: Some("Process OCR".to_string()),
                    url: "https://ocr.com/process".to_string(),
                    method: Some("POST".to_string()),
                    headers: Some(HashMap::from([(
                        "Authorization".to_string(),
                        "Bearer xxx".to_string(),
                    )])),
                    fields: Some(vec!["case_id".to_string(), "data".to_string()]),
                    use_response_from: None,
                    retry: RetryConfig {
                        enabled: true,
                        max_attempts: 3,
                        delay_ms: 1000,
                    },
                    on_error: OnError::Stop,
                },
                AutomationAction::Delay {
                    name: Some("Wait".to_string()),
                    duration_ms: 5000,
                },
            ],
        };

        let json = serde_json::to_string_pretty(&automation).unwrap();
        println!("{}", json);

        let deserialized: PhaseAutomation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.phase, "OCR");
        assert_eq!(deserialized.actions.len(), 2);
    }

    #[test]
    fn test_conditional_simple_serialization() {
        let action = AutomationAction::Conditional {
            name: Some("Check amount".to_string()),
            condition: Condition::Simple {
                field: "data.amount".to_string(),
                operator: ">".to_string(),
                value: serde_json::json!(10000),
            },
            then: vec![AutomationAction::MoveToPhase {
                name: None,
                phase: "Approved".to_string(),
            }],
            r#else: Some(vec![AutomationAction::MoveToPhase {
                name: None,
                phase: "Rejected".to_string(),
            }]),
        };

        let json = serde_json::to_string_pretty(&action).unwrap();
        let deserialized: AutomationAction = serde_json::from_str(&json).unwrap();

        match deserialized {
            AutomationAction::Conditional { then, r#else, .. } => {
                assert_eq!(then.len(), 1);
                assert!(r#else.is_some());
            }
            _ => panic!("Expected Conditional action"),
        }
    }

    #[test]
    fn test_conditional_complex_serialization() {
        let action = AutomationAction::Conditional {
            name: Some("Complex check".to_string()),
            condition: Condition::Complex {
                operator: LogicalOperator::And,
                conditions: vec![
                    SimpleCondition {
                        field: "data.amount".to_string(),
                        operator: ">".to_string(),
                        value: serde_json::json!(10000),
                    },
                    SimpleCondition {
                        field: "status".to_string(),
                        operator: "==".to_string(),
                        value: serde_json::json!("active"),
                    },
                ],
            },
            then: vec![],
            r#else: None,
        };

        let json = serde_json::to_string_pretty(&action).unwrap();
        let deserialized: AutomationAction = serde_json::from_str(&json).unwrap();

        match deserialized {
            AutomationAction::Conditional { condition, .. } => match condition {
                Condition::Complex { conditions, .. } => {
                    assert_eq!(conditions.len(), 2);
                }
                _ => panic!("Expected Complex condition"),
            },
            _ => panic!("Expected Conditional action"),
        }
    }

    #[test]
    fn test_set_field_action() {
        let action = AutomationAction::SetField {
            name: Some("Set approval".to_string()),
            field: "data.approved_by".to_string(),
            value: serde_json::json!("system"),
        };

        let json = serde_json::to_string_pretty(&action).unwrap();
        let deserialized: AutomationAction = serde_json::from_str(&json).unwrap();

        match deserialized {
            AutomationAction::SetField { field, value, .. } => {
                assert_eq!(field, "data.approved_by");
                assert_eq!(value, serde_json::json!("system"));
            }
            _ => panic!("Expected SetField action"),
        }
    }

    #[test]
    fn test_move_to_phase_action() {
        let action = AutomationAction::MoveToPhase {
            name: Some("Auto approve".to_string()),
            phase: "Approved".to_string(),
        };

        let json = serde_json::to_string_pretty(&action).unwrap();
        let deserialized: AutomationAction = serde_json::from_str(&json).unwrap();

        match deserialized {
            AutomationAction::MoveToPhase { phase, .. } => {
                assert_eq!(phase, "Approved");
            }
            _ => panic!("Expected MoveToPhase action"),
        }
    }

    #[test]
    fn test_sla_config() {
        let sla = WorkflowSlaConfig {
            phase_slas: HashMap::from([
                (
                    "Review".to_string(),
                    PhaseSla { hours: 24 },
                ),
                (
                    "Approval".to_string(),
                    PhaseSla { hours: 48 },
                ),
            ]),
        };

        let json = serde_json::to_string_pretty(&sla).unwrap();
        let deserialized: WorkflowSlaConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.phase_slas.len(), 2);
        assert_eq!(deserialized.phase_slas.get("Review").unwrap().hours, 24);
        assert_eq!(deserialized.phase_slas.get("Approval").unwrap().hours, 48);
    }
}
