use orchepy::engine::AutomationExecutor;
use orchepy::models::{
    automation::{
        AutomationAction, AutomationTrigger, Condition, LogicalOperator, PhaseAutomation,
        SimpleCondition,
    },
    Case,
};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_conditional_execution_simple() {
    let executor = AutomationExecutor::new();

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({
            "amount": 15000,
            "customer": "Test Corp"
        }),
        None,
    );

    let automation = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Review".to_string(),
        actions: vec![AutomationAction::Conditional {
            name: Some("Check amount".to_string()),
            condition: Condition::Simple {
                field: "data.amount".to_string(),
                operator: ">".to_string(),
                value: json!(10000),
            },
            then: vec![AutomationAction::MoveToPhase {
                name: None,
                phase: "Manager Approval".to_string(),
            }],
            r#else: Some(vec![AutomationAction::MoveToPhase {
                name: None,
                phase: "Auto Approved".to_string(),
            }]),
        }],
    };

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_conditional_execution_complex_and() {
    let executor = AutomationExecutor::new();

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({
            "amount": 15000,
            "priority": "high"
        }),
        None,
    );

    let automation = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Review".to_string(),
        actions: vec![AutomationAction::Conditional {
            name: Some("Complex check".to_string()),
            condition: Condition::Complex {
                operator: LogicalOperator::And,
                conditions: vec![
                    SimpleCondition {
                        field: "data.amount".to_string(),
                        operator: ">".to_string(),
                        value: json!(10000),
                    },
                    SimpleCondition {
                        field: "data.priority".to_string(),
                        operator: "==".to_string(),
                        value: json!("high"),
                    },
                ],
            },
            then: vec![AutomationAction::SetField {
                name: None,
                field: "data.escalated".to_string(),
                value: json!(true),
            }],
            r#else: None,
        }],
    };

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_conditional_execution_complex_or() {
    let executor = AutomationExecutor::new();

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({
            "amount": 5000,
            "urgent": true
        }),
        None,
    );

    let automation = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Review".to_string(),
        actions: vec![AutomationAction::Conditional {
            name: Some("OR check".to_string()),
            condition: Condition::Complex {
                operator: LogicalOperator::Or,
                conditions: vec![
                    SimpleCondition {
                        field: "data.amount".to_string(),
                        operator: ">".to_string(),
                        value: json!(10000),
                    },
                    SimpleCondition {
                        field: "data.urgent".to_string(),
                        operator: "==".to_string(),
                        value: json!(true),
                    },
                ],
            },
            then: vec![AutomationAction::MoveToPhase {
                name: None,
                phase: "Priority Queue".to_string(),
            }],
            r#else: None,
        }],
    };

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_contains_operator() {
    let executor = AutomationExecutor::new();

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({
            "description": "Urgent: Please review ASAP"
        }),
        None,
    );

    let automation = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Review".to_string(),
        actions: vec![AutomationAction::Conditional {
            name: Some("Check description".to_string()),
            condition: Condition::Simple {
                field: "data.description".to_string(),
                operator: "contains".to_string(),
                value: json!("Urgent"),
            },
            then: vec![AutomationAction::SetField {
                name: None,
                field: "data.priority".to_string(),
                value: json!("high"),
            }],
            r#else: None,
        }],
    };

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await;

    assert!(result.is_ok());
}
