use orchepy::models::case::{Case, CaseStatus};
use orchepy::models::Workflow;
use orchepy::repositories::{CaseRepository, WorkflowRepository};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[allow(dead_code)]
const MIGRATIONS: &str = "src/db/migrations";

async fn setup_test_workflow(pool: &PgPool) -> Workflow {
    let phases = vec![
        "New".to_string(),
        "In Progress".to_string(),
        "Review".to_string(),
        "Done".to_string(),
    ];

    let workflow = Workflow {
        id: Uuid::new_v4(),
        name: "Test Workflow".to_string(),
        phases: phases.clone(),
        initial_phase: "New".to_string(),
        webhook_url: None,
        active: true,
        description: None,
        automations: None,
        sla_config: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let repo = WorkflowRepository::new(pool);
    repo.create(&workflow).await.unwrap();

    workflow
}

async fn create_test_case(pool: &PgPool, workflow_id: Uuid) -> Case {
    let case = Case::new(
        workflow_id,
        "New".to_string(),
        json!({"amount": 1000}),
        Some(json!({"source": "test"})),
    );

    let repo = CaseRepository::new(pool);
    repo.create(&case).await.unwrap();

    case
}

#[sqlx::test(migrations = "src/db/migrations")]
async fn test_case_creation(pool: PgPool) {
    let workflow = setup_test_workflow(&pool).await;
    let case = create_test_case(&pool, workflow.id).await;

    let repo = CaseRepository::new(&pool);
    let fetched_case = repo.find_by_id(case.id).await.unwrap().unwrap();

    assert_eq!(fetched_case.id, case.id);
    assert_eq!(fetched_case.workflow_id, workflow.id);
    assert_eq!(fetched_case.current_phase, "New");
    assert_eq!(fetched_case.previous_phase, None);
    assert_eq!(fetched_case.status, CaseStatus::Active);
}

#[sqlx::test(migrations = "src/db/migrations")]
async fn test_case_phase_transition(pool: PgPool) {
    let workflow = setup_test_workflow(&pool).await;
    let case = create_test_case(&pool, workflow.id).await;

    let repo = CaseRepository::new(&pool);
    repo.update_phase(case.id, "In Progress", Some("New")).await.unwrap();

    let updated_case = repo.find_by_id(case.id).await.unwrap().unwrap();

    assert_eq!(updated_case.current_phase, "In Progress");
    assert_eq!(updated_case.previous_phase, Some("New".to_string()));
}

#[sqlx::test(migrations = "src/db/migrations")]
async fn test_case_data_update(pool: PgPool) {
    let workflow = setup_test_workflow(&pool).await;
    let case = create_test_case(&pool, workflow.id).await;

    let new_data = json!({"amount": 2000, "updated": true});
    let repo = CaseRepository::new(&pool);
    repo.update_data(case.id, &new_data).await.unwrap();

    let updated_case = repo.find_by_id(case.id).await.unwrap().unwrap();

    assert_eq!(updated_case.data, new_data);
    assert_eq!(updated_case.data["amount"], 2000);
    assert_eq!(updated_case.data["updated"], true);
}

#[sqlx::test(migrations = "src/db/migrations")]
async fn test_jsonb_set_field(pool: PgPool) {
    let workflow = setup_test_workflow(&pool).await;
    let case = create_test_case(&pool, workflow.id).await;

    let repo = CaseRepository::new(&pool);
    repo.set_field(case.id, "processed", &json!(true)).await.unwrap();

    let updated_case = repo.find_by_id(case.id).await.unwrap().unwrap();

    assert_eq!(updated_case.data["processed"], true);
    assert_eq!(updated_case.data["amount"], 1000);
}

#[sqlx::test(migrations = "src/db/migrations")]
async fn test_case_history_creation(pool: PgPool) {
    let workflow = setup_test_workflow(&pool).await;
    let case = create_test_case(&pool, workflow.id).await;

    let history = orchepy::models::case::CaseHistory::new(
        case.id,
        Some("New".to_string()),
        "In Progress".to_string(),
        Some("User action".to_string()),
        Some("user@test.com".to_string()),
    );

    let repo = CaseRepository::new(&pool);
    repo.create_history(&history).await.unwrap();

    let histories = repo.get_history(case.id).await.unwrap();
    assert_eq!(histories.len(), 1);
    assert_eq!(histories[0].to_phase, "In Progress");
}

#[sqlx::test(migrations = "src/db/migrations")]
async fn test_list_cases_by_workflow(pool: PgPool) {
    let workflow = setup_test_workflow(&pool).await;
    let _case1 = create_test_case(&pool, workflow.id).await;
    let _case2 = create_test_case(&pool, workflow.id).await;

    let repo = CaseRepository::new(&pool);
    let cases = repo.list_by_workflow(workflow.id, 10, 0).await.unwrap();

    assert_eq!(cases.len(), 2);
}

#[sqlx::test(migrations = "src/db/migrations")]
async fn test_list_cases_by_phase(pool: PgPool) {
    let workflow = setup_test_workflow(&pool).await;
    let case1 = create_test_case(&pool, workflow.id).await;
    let _case2 = create_test_case(&pool, workflow.id).await;

    let repo = CaseRepository::new(&pool);
    repo.update_phase(case1.id, "Review", Some("New")).await.unwrap();

    let cases_in_review = repo.list_by_workflow_and_phase(workflow.id, "Review", 10, 0).await.unwrap();

    assert_eq!(cases_in_review.len(), 1);
    assert_eq!(cases_in_review[0].id, case1.id);
}

#[sqlx::test(migrations = "src/db/migrations")]
async fn test_phase_entered_at_tracking(pool: PgPool) {
    let workflow = setup_test_workflow(&pool).await;
    let case = create_test_case(&pool, workflow.id).await;

    let initial_phase_entered_at = case.phase_entered_at;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let repo = CaseRepository::new(&pool);
    repo.update_phase(case.id, "In Progress", Some("New")).await.unwrap();

    let updated_case = repo.find_by_id(case.id).await.unwrap().unwrap();

    assert_ne!(updated_case.phase_entered_at, initial_phase_entered_at);
    assert!(updated_case.phase_entered_at > initial_phase_entered_at);
}

#[sqlx::test(migrations = "src/db/migrations")]
async fn test_case_status_transitions(pool: PgPool) {
    let workflow = setup_test_workflow(&pool).await;
    let case = create_test_case(&pool, workflow.id).await;

    assert_eq!(case.status, CaseStatus::Active);

    let repo = CaseRepository::new(&pool);
    repo.update_status(case.id, &CaseStatus::Completed).await.unwrap();

    let updated_case = repo.find_by_id(case.id).await.unwrap().unwrap();

    assert_eq!(updated_case.status, CaseStatus::Completed);
}

#[sqlx::test(migrations = "src/db/migrations")]
async fn test_metadata_handling(pool: PgPool) {
    let workflow = setup_test_workflow(&pool).await;
    let case = create_test_case(&pool, workflow.id).await;

    assert!(case.metadata.is_some());
    assert_eq!(case.metadata.as_ref().unwrap()["source"], "test");

    let new_data = case.data.clone();
    let repo = CaseRepository::new(&pool);
    repo.update_data(case.id, &new_data).await.unwrap();

    let updated_case = repo.find_by_id(case.id).await.unwrap().unwrap();
    assert!(updated_case.metadata.is_some());
}
