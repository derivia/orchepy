use orchepy::engine::AutomationExecutor;
use orchepy::models::automation::*;
use orchepy::models::case::Case;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_move_to_phase_action() {
    let executor = AutomationExecutor::new();

    let automation = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Review".to_string(),
        actions: vec![AutomationAction::MoveToPhase {
            name: Some("Auto approve".to_string()),
            phase: "Approved".to_string(),
        }],
    };

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({"amount": 5000}),
        None,
    );

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await
        .unwrap();

    assert_eq!(result.modifications.len(), 1);
    match &result.modifications[0] {
        CaseModification::MoveToPhase { phase } => {
            assert_eq!(phase, "Approved");
        }
        _ => panic!("Expected MoveToPhase modification"),
    }
}

#[tokio::test]
async fn test_set_field_action() {
    let executor = AutomationExecutor::new();

    let automation = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Processing".to_string(),
        actions: vec![AutomationAction::SetField {
            name: Some("Set processed flag".to_string()),
            field: "data.processed".to_string(),
            value: json!(true),
        }],
    };

    let case = Case::new(
        Uuid::new_v4(),
        "Processing".to_string(),
        json!({"amount": 1000}),
        None,
    );

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await
        .unwrap();

    assert_eq!(result.modifications.len(), 1);
    match &result.modifications[0] {
        CaseModification::SetField { field, value } => {
            assert_eq!(field, "data.processed");
            assert_eq!(value, &json!(true));
        }
        _ => panic!("Expected SetField modification"),
    }
}

#[tokio::test]
async fn test_conditional_simple_true() {
    let executor = AutomationExecutor::new();

    let automation = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Review".to_string(),
        actions: vec![AutomationAction::Conditional {
            name: Some("Check amount".to_string()),
            condition: Condition::Simple {
                field: "data.amount".to_string(),
                operator: ">".to_string(),
                value: json!(1000),
            },
            then: vec![AutomationAction::MoveToPhase {
                name: None,
                phase: "Manager Review".to_string(),
            }],
            r#else: None,
        }],
    };

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({"amount": 5000}),
        None,
    );

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await
        .unwrap();

    assert_eq!(result.modifications.len(), 1);
    match &result.modifications[0] {
        CaseModification::MoveToPhase { phase } => {
            assert_eq!(phase, "Manager Review");
        }
        _ => panic!("Expected MoveToPhase modification"),
    }
}

#[tokio::test]
async fn test_conditional_simple_false() {
    let executor = AutomationExecutor::new();

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
                phase: "Manager Review".to_string(),
            }],
            r#else: Some(vec![AutomationAction::MoveToPhase {
                name: None,
                phase: "Auto Approved".to_string(),
            }]),
        }],
    };

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({"amount": 500}),
        None,
    );

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await
        .unwrap();

    assert_eq!(result.modifications.len(), 1);
    match &result.modifications[0] {
        CaseModification::MoveToPhase { phase } => {
            assert_eq!(phase, "Auto Approved");
        }
        _ => panic!("Expected MoveToPhase modification"),
    }
}

#[tokio::test]
async fn test_conditional_complex_and_true() {
    let executor = AutomationExecutor::new();

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
                        value: json!(1000),
                    },
                    SimpleCondition {
                        field: "status".to_string(),
                        operator: "==".to_string(),
                        value: json!("active"),
                    },
                ],
            },
            then: vec![AutomationAction::SetField {
                name: None,
                field: "data.priority".to_string(),
                value: json!("high"),
            }],
            r#else: None,
        }],
    };

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({"amount": 5000}),
        None,
    );

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await
        .unwrap();

    assert_eq!(result.modifications.len(), 1);
    match &result.modifications[0] {
        CaseModification::SetField { field, value } => {
            assert_eq!(field, "data.priority");
            assert_eq!(value, &json!("high"));
        }
        _ => panic!("Expected SetField modification"),
    }
}

#[tokio::test]
async fn test_conditional_complex_and_false() {
    let executor = AutomationExecutor::new();

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
                        value: json!(1000),
                    },
                    SimpleCondition {
                        field: "status".to_string(),
                        operator: "==".to_string(),
                        value: json!("completed"),
                    },
                ],
            },
            then: vec![AutomationAction::SetField {
                name: None,
                field: "data.priority".to_string(),
                value: json!("high"),
            }],
            r#else: None,
        }],
    };

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({"amount": 5000}),
        None,
    );

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await
        .unwrap();

    assert_eq!(result.modifications.len(), 0);
}

#[tokio::test]
async fn test_conditional_complex_or_true() {
    let executor = AutomationExecutor::new();

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
                        value: json!(50000),
                    },
                    SimpleCondition {
                        field: "data.vip".to_string(),
                        operator: "==".to_string(),
                        value: json!(true),
                    },
                ],
            },
            then: vec![AutomationAction::MoveToPhase {
                name: None,
                phase: "VIP Review".to_string(),
            }],
            r#else: None,
        }],
    };

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({"amount": 100, "vip": true}),
        None,
    );

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await
        .unwrap();

    assert_eq!(result.modifications.len(), 1);
    match &result.modifications[0] {
        CaseModification::MoveToPhase { phase } => {
            assert_eq!(phase, "VIP Review");
        }
        _ => panic!("Expected MoveToPhase modification"),
    }
}

#[tokio::test]
async fn test_delay_action() {
    let executor = AutomationExecutor::new();

    let automation = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Processing".to_string(),
        actions: vec![AutomationAction::Delay {
            name: Some("Wait briefly".to_string()),
            duration_ms: 10,
        }],
    };

    let case = Case::new(
        Uuid::new_v4(),
        "Processing".to_string(),
        json!({}),
        None,
    );

    let start = std::time::Instant::now();
    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await
        .unwrap();
    let elapsed = start.elapsed();

    assert!(elapsed.as_millis() >= 10);
    assert_eq!(result.modifications.len(), 0);
}

#[tokio::test]
async fn test_multiple_actions_sequential() {
    let executor = AutomationExecutor::new();

    let automation = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Processing".to_string(),
        actions: vec![
            AutomationAction::SetField {
                name: Some("Set status".to_string()),
                field: "data.status".to_string(),
                value: json!("processing"),
            },
            AutomationAction::SetField {
                name: Some("Set timestamp".to_string()),
                field: "data.processed_at".to_string(),
                value: json!("2024-01-01"),
            },
            AutomationAction::MoveToPhase {
                name: Some("Move to done".to_string()),
                phase: "Done".to_string(),
            },
        ],
    };

    let case = Case::new(
        Uuid::new_v4(),
        "Processing".to_string(),
        json!({}),
        None,
    );

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await
        .unwrap();

    assert_eq!(result.modifications.len(), 3);
}

#[tokio::test]
async fn test_contains_operator() {
    let executor = AutomationExecutor::new();

    let automation = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Review".to_string(),
        actions: vec![AutomationAction::Conditional {
            name: Some("Check email".to_string()),
            condition: Condition::Simple {
                field: "data.email".to_string(),
                operator: "contains".to_string(),
                value: json!("@company.com"),
            },
            then: vec![AutomationAction::SetField {
                name: None,
                field: "data.is_internal".to_string(),
                value: json!(true),
            }],
            r#else: None,
        }],
    };

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({"email": "user@company.com"}),
        None,
    );

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await
        .unwrap();

    assert_eq!(result.modifications.len(), 1);
}

#[tokio::test]
async fn test_equals_operator_with_equal_sign() {
    let executor = AutomationExecutor::new();

    let automation = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Review".to_string(),
        actions: vec![AutomationAction::Conditional {
            name: Some("Check type".to_string()),
            condition: Condition::Simple {
                field: "data.type".to_string(),
                operator: "=".to_string(),
                value: json!("urgent"),
            },
            then: vec![AutomationAction::SetField {
                name: None,
                field: "data.priority".to_string(),
                value: json!("high"),
            }],
            r#else: None,
        }],
    };

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({"type": "urgent"}),
        None,
    );

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await
        .unwrap();

    assert_eq!(result.modifications.len(), 1);
}

#[tokio::test]
async fn test_not_equals_operator() {
    let executor = AutomationExecutor::new();

    let automation = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Review".to_string(),
        actions: vec![AutomationAction::Conditional {
            name: Some("Check status".to_string()),
            condition: Condition::Simple {
                field: "status".to_string(),
                operator: "!=".to_string(),
                value: json!("active"),
            },
            then: vec![AutomationAction::MoveToPhase {
                name: None,
                phase: "Archived".to_string(),
            }],
            r#else: None,
        }],
    };

    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({}),
        None,
    );

    let result = executor
        .execute_automations(&[&automation], &case, None)
        .await
        .unwrap();

    assert_eq!(result.modifications.len(), 0);
}

#[tokio::test]
async fn test_comparison_operators() {
    let executor = AutomationExecutor::new();
    let case = Case::new(
        Uuid::new_v4(),
        "Review".to_string(),
        json!({"score": 75}),
        None,
    );

    let automation_gte = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Review".to_string(),
        actions: vec![AutomationAction::Conditional {
            name: None,
            condition: Condition::Simple {
                field: "data.score".to_string(),
                operator: ">=".to_string(),
                value: json!(70),
            },
            then: vec![AutomationAction::SetField {
                name: None,
                field: "data.pass".to_string(),
                value: json!(true),
            }],
            r#else: None,
        }],
    };

    let result = executor
        .execute_automations(&[&automation_gte], &case, None)
        .await
        .unwrap();
    assert_eq!(result.modifications.len(), 1);

    let automation_lte = PhaseAutomation {
        trigger: AutomationTrigger::OnEnter,
        phase: "Review".to_string(),
        actions: vec![AutomationAction::Conditional {
            name: None,
            condition: Condition::Simple {
                field: "data.score".to_string(),
                operator: "<=".to_string(),
                value: json!(80),
            },
            then: vec![AutomationAction::SetField {
                name: None,
                field: "data.in_range".to_string(),
                value: json!(true),
            }],
            r#else: None,
        }],
    };

    let result = executor
        .execute_automations(&[&automation_lte], &case, None)
        .await
        .unwrap();
    assert_eq!(result.modifications.len(), 1);
}
