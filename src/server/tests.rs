//! Tests for the Asana MCP server.

use super::*;
use crate::client::AsanaClient;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

/// Custom matcher that matches requests without an "offset" query parameter.
struct NoOffset;

impl Match for NoOffset {
    fn matches(&self, request: &Request) -> bool {
        !request.url.query_pairs().any(|(k, _)| k == "offset")
    }
}

/// Custom matcher for query parameter value.
struct QueryParam {
    key: &'static str,
    value: &'static str,
}

impl Match for QueryParam {
    fn matches(&self, request: &Request) -> bool {
        request
            .url
            .query_pairs()
            .any(|(k, v)| k == self.key && v == self.value)
    }
}

fn test_server(mock_uri: &str) -> AsanaServer {
    let client = AsanaClient::new("test-token")
        .unwrap()
        .with_base_url(mock_uri);
    AsanaServer::with_client(client)
}

fn get_response_text(result: &CallToolResult) -> &str {
    &result.content[0]
        .as_text()
        .expect("Expected text content")
        .text
}

fn get_params(resource_type: ResourceType, gid: &str) -> Parameters<GetParams> {
    Parameters(GetParams {
        resource_type,
        gid: Some(gid.to_string()),
        depth: None,
        subtask_depth: None,
        include_subtasks: None,
        include_dependencies: None,
        include_comments: None,
        detail_level: DetailLevel::Default,
        extra_fields: None,
        opt_fields: None,
    })
}

fn get_params_with_fields(
    resource_type: ResourceType,
    gid: &str,
    detail_level: DetailLevel,
    extra_fields: Option<Vec<&str>>,
    opt_fields: Option<Vec<&str>>,
) -> Parameters<GetParams> {
    Parameters(GetParams {
        resource_type,
        gid: Some(gid.to_string()),
        depth: None,
        subtask_depth: None,
        include_subtasks: None,
        include_dependencies: None,
        include_comments: None,
        detail_level,
        extra_fields: extra_fields.map(|f| f.into_iter().map(String::from).collect()),
        opt_fields: opt_fields.map(|f| f.into_iter().map(String::from).collect()),
    })
}

// ============================================================================
// Workspaces Tests
// ============================================================================

#[tokio::test]
async fn test_workspaces_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "123", "name": "My Workspace", "is_organization": true},
                {"gid": "456", "name": "Another Workspace", "is_organization": false}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_workspaces(Parameters(WorkspacesParams {}))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("My Workspace"));
    assert!(text.contains("Another Workspace"));
}

// ============================================================================
// Get Project Tests
// ============================================================================

#[tokio::test]
async fn test_get_project_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects/proj123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "proj123",
                "name": "Test Project",
                "archived": false,
                "public": true,
                "notes": "Project description"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::Project, "proj123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Test Project"));
    assert!(text.contains("proj123"));
}

#[tokio::test]
async fn test_get_project_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects/missing"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::Project, "missing"))
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("Failed to get project"));
}

// ============================================================================
// Detail Level and Field Selection Tests
// ============================================================================

/// Custom matcher for opt_fields query parameter with dynamic string.
struct OptFieldsEquals(String);

impl Match for OptFieldsEquals {
    fn matches(&self, request: &Request) -> bool {
        request
            .url
            .query_pairs()
            .any(|(k, v)| k == "opt_fields" && v == self.0)
    }
}

#[tokio::test]
async fn test_get_project_with_minimal_detail_level() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects/proj123"))
        .and(OptFieldsEquals(MINIMAL_FIELDS.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "proj123",
                "name": "Test Project",
                "resource_type": "project"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = get_params_with_fields(
        ResourceType::Project,
        "proj123",
        DetailLevel::Minimal,
        None,
        None,
    );

    let result = server.asana_get(params).await.unwrap();
    assert!(get_response_text(&result).contains("Test Project"));
}

#[tokio::test]
async fn test_get_project_with_extra_fields() {
    let mock_server = MockServer::start().await;

    let expected_fields = format!("{},due_on,owner.name", MINIMAL_FIELDS);
    Mock::given(method("GET"))
        .and(path("/projects/proj123"))
        .and(OptFieldsEquals(expected_fields))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "proj123",
                "name": "Test Project",
                "resource_type": "project",
                "due_on": "2024-12-31",
                "owner": {"name": "Test Owner"}
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = get_params_with_fields(
        ResourceType::Project,
        "proj123",
        DetailLevel::Minimal,
        Some(vec!["due_on", "owner.name"]),
        None,
    );

    let result = server.asana_get(params).await.unwrap();
    let text = get_response_text(&result);
    assert!(text.contains("Test Project"));
    assert!(text.contains("2024-12-31"));
}

#[tokio::test]
async fn test_opt_fields_overrides_detail_level() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects/proj123"))
        .and(OptFieldsEquals("gid,custom_field".to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "proj123",
                "custom_field": "custom_value"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = get_params_with_fields(
        ResourceType::Project,
        "proj123",
        DetailLevel::Minimal,
        Some(vec!["due_on"]),
        Some(vec!["gid", "custom_field"]),
    );

    let result = server.asana_get(params).await.unwrap();
    assert!(get_response_text(&result).contains("custom_value"));
}

#[tokio::test]
async fn test_task_search_with_minimal_detail_level() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/tasks/search"))
        .and(OptFieldsEquals(MINIMAL_FIELDS.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "task1", "name": "Task One", "resource_type": "task"},
                {"gid": "task2", "name": "Task Two", "resource_type": "task"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(TaskSearchParams {
        workspace_gid: Some("ws123".to_string()),
        detail_level: DetailLevel::Minimal,
        ..Default::default()
    });

    let result = server.asana_task_search(params).await.unwrap();
    let text = get_response_text(&result);
    assert!(text.contains("Task One"));
    assert!(text.contains("Task Two"));
}

// ============================================================================
// Recursive Portfolio Tests
// ============================================================================

#[tokio::test]
async fn test_get_portfolio_depth_zero_returns_no_items() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/portfolios/port123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "port123",
                "name": "Test Portfolio"
            }
        })))
        .mount(&mock_server)
        .await;

    // Note: No items endpoint mock - shouldn't be called with depth=0

    let server = test_server(&mock_server.uri());
    let mut params = get_params(ResourceType::Portfolio, "port123");
    params.0.depth = Some(0); // depth=0 means no items

    let result = server.asana_get(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Test Portfolio"));
    assert!(text.contains("\"items\": []"));
}

#[tokio::test]
async fn test_get_portfolio_depth_one_fetches_direct_children() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/portfolios/port123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "port123",
                "name": "Parent Portfolio"
            }
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/portfolios/port123/items"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "proj1", "resource_type": "project", "name": "Project 1"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/projects/proj1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "proj1",
                "name": "Project 1 Full"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let mut params = get_params(ResourceType::Portfolio, "port123");
    params.0.depth = Some(1);

    let result = server.asana_get(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Parent Portfolio"));
    assert!(text.contains("Project 1 Full"));
}

#[tokio::test]
async fn test_get_portfolio_unlimited_depth_traverses_nested() {
    let mock_server = MockServer::start().await;

    // Parent portfolio
    Mock::given(method("GET"))
        .and(path("/portfolios/parent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "parent", "name": "Parent"}
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/portfolios/parent/items"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "child", "resource_type": "portfolio", "name": "Child"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    // Child portfolio
    Mock::given(method("GET"))
        .and(path("/portfolios/child"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "child", "name": "Child Portfolio"}
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/portfolios/child/items"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "proj1", "resource_type": "project", "name": "Project"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/projects/proj1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "proj1", "name": "Nested Project"}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let mut params = get_params(ResourceType::Portfolio, "parent");
    params.0.depth = Some(-1); // Unlimited

    let result = server.asana_get(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Parent"));
    assert!(text.contains("Child Portfolio"));
    assert!(text.contains("Nested Project"));
}

// ============================================================================
// Task With Context Tests
// ============================================================================

#[tokio::test]
async fn test_get_task_with_all_context() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/tasks/task123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "task123",
                "name": "Test Task",
                "completed": false
            }
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/tasks/task123/subtasks"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"gid": "sub1", "name": "Subtask 1", "completed": false, "num_subtasks": 0}],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/tasks/task123/dependencies"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"gid": "dep1", "name": "Blocker Task"}],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/tasks/task123/dependents"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/tasks/task123/stories"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "story1", "resource_subtype": "comment_added", "text": "Hello"},
                {"gid": "story2", "resource_subtype": "added_to_project", "text": null}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::Task, "task123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Test Task"));
    assert!(text.contains("Subtask 1"));
    assert!(text.contains("Blocker Task"));
    assert!(text.contains("Hello")); // Comment
    assert!(!text.contains("added_to_project")); // System story filtered
}

#[tokio::test]
async fn test_get_task_without_context() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/tasks/task123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "task123",
                "name": "Test Task",
                "completed": false
            }
        })))
        .mount(&mock_server)
        .await;

    // No subtasks/dependencies/stories mocks - shouldn't be called

    let server = test_server(&mock_server.uri());
    let params = Parameters(GetParams {
        resource_type: ResourceType::Task,
        gid: Some("task123".to_string()),
        depth: None,
        subtask_depth: None,
        include_subtasks: Some(false),
        include_dependencies: Some(false),
        include_comments: Some(false),
        detail_level: DetailLevel::Default,
        extra_fields: None,
        opt_fields: None,
    });

    let result = server.asana_get(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Test Task"));
    // When include_* flags are false, empty arrays are omitted from serialization
    // (due to #[serde(skip_serializing_if = "Vec::is_empty")])
    assert!(!text.contains("\"subtasks\""));
    assert!(!text.contains("\"dependencies\""));
    assert!(!text.contains("\"comments\""));
}

// ============================================================================
// Get Tasks Recursive Tests
// ============================================================================

#[tokio::test]
async fn test_get_tasks_from_project_no_subtasks() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects/proj123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "proj123"}
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/projects/proj123/tasks"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "task1", "name": "Task 1", "num_subtasks": 0},
                {"gid": "task2", "name": "Task 2", "num_subtasks": 0}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let mut params = get_params(ResourceType::ProjectTasks, "proj123");
    params.0.subtask_depth = Some(0);

    let result = server.asana_get(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Task 1"));
    assert!(text.contains("Task 2"));
}

#[tokio::test]
async fn test_get_tasks_from_project_with_subtask_expansion() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects/proj123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "proj123"}
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/projects/proj123/tasks"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "task1", "name": "Parent Task", "num_subtasks": 2}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/tasks/task1/subtasks"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "sub1", "name": "Subtask 1", "num_subtasks": 0},
                {"gid": "sub2", "name": "Subtask 2", "num_subtasks": 0}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let mut params = get_params(ResourceType::ProjectTasks, "proj123");
    params.0.subtask_depth = Some(1);

    let result = server.asana_get(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Parent Task"));
    assert!(text.contains("Subtask 1"));
    assert!(text.contains("Subtask 2"));
}

#[tokio::test]
async fn test_get_tasks_detects_portfolio_after_project_404() {
    let mock_server = MockServer::start().await;

    // Project returns 404
    Mock::given(method("GET"))
        .and(path("/projects/port123"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    // Portfolio succeeds
    Mock::given(method("GET"))
        .and(path("/portfolios/port123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "port123", "name": "Portfolio"}
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/portfolios/port123/items"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"gid": "proj1", "resource_type": "project", "name": "Project"}],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/projects/proj1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "proj1"}
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/projects/proj1/tasks"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"gid": "task1", "name": "Portfolio Task", "num_subtasks": 0}],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let mut params = get_params(ResourceType::ProjectTasks, "port123");
    params.0.subtask_depth = Some(0);
    params.0.depth = Some(1);

    let result = server.asana_get(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Portfolio Task"));
}

// ============================================================================
// Create Tests
// ============================================================================

#[tokio::test]
async fn test_create_task_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tasks"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {
                "gid": "new_task",
                "name": "New Task",
                "completed": false
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::Task,
        workspace_gid: Some("ws123".to_string()),
        name: Some("New Task".to_string()),
        project_gid: None,
        task_gid: None,
        team_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        text: None,
        custom_fields: None,
        source_gid: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("new_task"));
    assert!(text.contains("New Task"));
}

#[tokio::test]
async fn test_create_subtask_requires_task_gid() {
    let mock_server = MockServer::start().await;
    let server = test_server(&mock_server.uri());

    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::Subtask,
        task_gid: None, // Missing required field
        workspace_gid: None,
        name: Some("Subtask".to_string()),
        project_gid: None,
        team_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        text: None,
        custom_fields: None,
        source_gid: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("task_gid is required"));
}

#[tokio::test]
async fn test_create_project_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/projects"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {
                "gid": "new_proj",
                "name": "New Project"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::Project,
        workspace_gid: Some("ws123".to_string()),
        name: Some("New Project".to_string()),
        project_gid: None,
        task_gid: None,
        team_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        text: None,
        custom_fields: None,
        source_gid: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("New Project"));
}

#[tokio::test]
async fn test_create_comment_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tasks/task123/stories"))
        .and(body_json(
            serde_json::json!({"data": {"text": "Hello world"}}),
        ))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {
                "gid": "story123",
                "text": "Hello world",
                "resource_subtype": "comment_added"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::Comment,
        task_gid: Some("task123".to_string()),
        text: Some("Hello world".to_string()),
        workspace_gid: None,
        name: None,
        project_gid: None,
        team_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        custom_fields: None,
        source_gid: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Hello world"));
}

// ============================================================================
// Update Tests
// ============================================================================

#[tokio::test]
async fn test_update_task_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/tasks/task123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "task123",
                "name": "Updated Task",
                "completed": true
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(UpdateParams {
        resource_type: UpdateResourceType::Task,
        gid: "task123".to_string(),
        name: Some("Updated Task".to_string()),
        completed: Some(true),
        notes: None,
        html_notes: None,
        html_text: None,
        due_on: None,
        start_on: None,
        assignee: None,
        color: None,
        archived: None,
        privacy_setting: None,
        public: None,
        text: None,
        title: None,
        status_type: None,
        custom_fields: None,
        opt_fields: None,
    });

    let result = server.asana_update(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Updated Task"));
    assert!(text.contains("true")); // completed: true
}

#[tokio::test]
async fn test_update_section_requires_name() {
    let mock_server = MockServer::start().await;
    let server = test_server(&mock_server.uri());

    let params = Parameters(UpdateParams {
        resource_type: UpdateResourceType::Section,
        gid: "section123".to_string(),
        name: None, // Missing required field
        notes: None,
        html_notes: None,
        html_text: None,
        completed: None,
        due_on: None,
        start_on: None,
        assignee: None,
        color: None,
        archived: None,
        privacy_setting: None,
        public: None,
        text: None,
        title: None,
        status_type: None,
        custom_fields: None,
        opt_fields: None,
    });

    let result = server.asana_update(params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("name is required"));
}

// ============================================================================
// Link Tests
// ============================================================================

#[tokio::test]
async fn test_link_task_to_project() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tasks/task123/addProject"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Add,
        relationship: RelationshipType::TaskProject,
        target_gid: "task123".to_string(),
        item_gid: Some("proj456".to_string()),
        item_gids: None,
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("success"));
    assert!(text.contains("Task added to project"));
}

#[tokio::test]
async fn test_link_add_dependencies() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tasks/task123/addDependencies"))
        .and(body_json(serde_json::json!({
            "data": {"dependencies": ["dep1", "dep2"]}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Add,
        relationship: RelationshipType::TaskDependency,
        target_gid: "task123".to_string(),
        item_gid: None,
        item_gids: Some(vec!["dep1".to_string(), "dep2".to_string()]),
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Dependencies added"));
}

#[tokio::test]
async fn test_link_requires_item_gid() {
    let mock_server = MockServer::start().await;
    let server = test_server(&mock_server.uri());

    let params = Parameters(LinkParams {
        action: LinkAction::Add,
        relationship: RelationshipType::TaskProject,
        target_gid: "task123".to_string(),
        item_gid: None,
        item_gids: None, // Both missing
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("item_gid"));
}

// ============================================================================
// Status Update (Single) Tests
// ============================================================================

#[tokio::test]
async fn test_get_status_update() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/status_updates/status123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "status123",
                "title": "Week 5 Update",
                "text": "Everything is on track",
                "status_type": "on_track"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::StatusUpdate, "status123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Week 5 Update"));
    assert!(text.contains("on_track"));
}

// ============================================================================
// Status Updates List Tests
// ============================================================================

#[tokio::test]
async fn test_status_updates_project_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/status_updates"))
        .and(query_param("parent", "proj123"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"gid": "status1", "title": "On track"}],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::StatusUpdates, "proj123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("On track"));
}

#[tokio::test]
async fn test_status_updates_portfolio_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/status_updates"))
        .and(query_param("parent", "port123"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"gid": "status2", "title": "Portfolio status"}],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::StatusUpdates, "port123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Portfolio status"));
}

// ============================================================================
// Alias Backward Compatibility Tests
// ============================================================================

#[tokio::test]
async fn test_resource_type_alias_favorites() {
    // Test that the old "favorites" name still works via alias
    let params: GetParams =
        serde_json::from_str(r#"{"resource_type": "favorites", "gid": "ws123"}"#).unwrap();
    assert_eq!(params.resource_type, ResourceType::WorkspaceFavorites);
}

#[tokio::test]
async fn test_resource_type_alias_tasks() {
    let params: GetParams =
        serde_json::from_str(r#"{"resource_type": "tasks", "gid": "proj123"}"#).unwrap();
    assert_eq!(params.resource_type, ResourceType::ProjectTasks);
}

#[tokio::test]
async fn test_resource_type_new_name_workspace_favorites() {
    let params: GetParams =
        serde_json::from_str(r#"{"resource_type": "workspace_favorites", "gid": "ws123"}"#)
            .unwrap();
    assert_eq!(params.resource_type, ResourceType::WorkspaceFavorites);
}

// ============================================================================
// Additional Get Tests - Complete Coverage
// ============================================================================

#[tokio::test]
async fn test_get_workspace_favorites() {
    let mock_server = MockServer::start().await;

    // Mock favorite projects list (Asana API requires resource_type parameter)
    Mock::given(method("GET"))
        .and(path("/users/me/favorites"))
        .and(QueryParam {
            key: "resource_type",
            value: "project",
        })
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "proj1", "resource_type": "project", "name": "My Project"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    // Mock favorite portfolios list
    Mock::given(method("GET"))
        .and(path("/users/me/favorites"))
        .and(QueryParam {
            key: "resource_type",
            value: "portfolio",
        })
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "port1", "resource_type": "portfolio", "name": "My Portfolio"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    // Mock project fetch
    Mock::given(method("GET"))
        .and(path("/projects/proj1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "proj1", "name": "My Project", "color": "blue"}
        })))
        .mount(&mock_server)
        .await;

    // Mock portfolio fetch
    Mock::given(method("GET"))
        .and(path("/portfolios/port1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "port1", "name": "My Portfolio"}
        })))
        .mount(&mock_server)
        .await;

    // Mock portfolio items (empty for depth=0)
    Mock::given(method("GET"))
        .and(path("/portfolios/port1/items"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(GetParams {
        resource_type: ResourceType::WorkspaceFavorites,
        gid: Some("ws123".to_string()),
        depth: Some(0),
        subtask_depth: None,
        include_subtasks: None,
        include_dependencies: None,
        include_comments: None,
        detail_level: DetailLevel::Default,
        extra_fields: None,
        opt_fields: None,
    });

    let result = server.asana_get(params).await.unwrap();
    let text = get_response_text(&result);

    // Response should include both projects and portfolios with type info
    assert!(text.contains("My Project"));
    assert!(text.contains("My Portfolio"));
}

#[tokio::test]
async fn test_get_task_subtasks() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/tasks/task123/subtasks"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "sub1", "name": "Subtask 1", "completed": false},
                {"gid": "sub2", "name": "Subtask 2", "completed": true}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::TaskSubtasks, "task123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Subtask 1"));
    assert!(text.contains("Subtask 2"));
}

#[tokio::test]
async fn test_get_task_comments() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/tasks/task123/stories"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "story1", "resource_subtype": "comment_added", "text": "Great work!"},
                {"gid": "story2", "resource_subtype": "assigned", "text": "Assigned to John"},
                {"gid": "story3", "resource_subtype": "comment_added", "text": "Thanks!"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::TaskComments, "task123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    // Only comments should be returned, not assignment stories
    assert!(text.contains("Great work!"));
    assert!(text.contains("Thanks!"));
    assert!(!text.contains("Assigned to John"));
}

#[tokio::test]
async fn test_get_workspace() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "ws123", "name": "My Workspace", "is_organization": true}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::Workspace, "ws123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("My Workspace"));
    assert!(text.contains("is_organization"));
}

#[tokio::test]
async fn test_get_workspace_templates() {
    let mock_server = MockServer::start().await;

    // When gid is provided, it's treated as team_gid
    Mock::given(method("GET"))
        .and(path("/teams/team123/project_templates"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "tmpl1", "name": "Sprint Template"},
                {"gid": "tmpl2", "name": "Onboarding Template"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::WorkspaceTemplates, "team123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Sprint Template"));
    assert!(text.contains("Onboarding Template"));
}

#[tokio::test]
async fn test_get_all_templates_no_gid() {
    let mock_server = MockServer::start().await;

    // When no gid is provided, lists all accessible templates
    Mock::given(method("GET"))
        .and(path("/project_templates"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "tmpl1", "name": "Global Template"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::WorkspaceTemplates, ""))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Global Template"));
}

#[tokio::test]
async fn test_get_project_template() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/project_templates/tmpl123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "tmpl123",
                "name": "Sprint Template",
                "description": "A template for sprints",
                "requested_dates": [{"gid": "date1", "name": "Sprint Start"}],
                "requested_roles": [{"gid": "role1", "name": "Sprint Lead"}]
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::ProjectTemplate, "tmpl123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Sprint Template"));
    assert!(text.contains("Sprint Start"));
    assert!(text.contains("Sprint Lead"));
}

#[tokio::test]
async fn test_get_project_sections() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects/proj123/sections"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "sec1", "name": "To Do"},
                {"gid": "sec2", "name": "In Progress"},
                {"gid": "sec3", "name": "Done"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::ProjectSections, "proj123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("To Do"));
    assert!(text.contains("In Progress"));
    assert!(text.contains("Done"));
}

#[tokio::test]
async fn test_get_section() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/sections/sec123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "sec123", "name": "In Progress", "project": {"gid": "proj1", "name": "My Project"}}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::Section, "sec123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("In Progress"));
    assert!(text.contains("My Project"));
}

#[tokio::test]
async fn test_get_workspace_tags() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/tags"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "tag1", "name": "Bug", "color": "red"},
                {"gid": "tag2", "name": "Feature", "color": "green"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::WorkspaceTags, "ws123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Bug"));
    assert!(text.contains("Feature"));
}

#[tokio::test]
async fn test_get_tag() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/tags/tag123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "tag123", "name": "Priority", "color": "orange", "notes": "High priority items"}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::Tag, "tag123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Priority"));
    assert!(text.contains("High priority items"));
}

#[tokio::test]
async fn test_get_my_tasks() {
    let mock_server = MockServer::start().await;

    // First call gets the user's task list
    Mock::given(method("GET"))
        .and(path("/users/me/user_task_list"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "tasklist123"}
        })))
        .mount(&mock_server)
        .await;

    // Second call gets tasks from that list
    Mock::given(method("GET"))
        .and(path("/user_task_lists/tasklist123/tasks"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "task1", "name": "My first task", "completed": false},
                {"gid": "task2", "name": "My second task", "completed": true}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::MyTasks, "ws123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("My first task"));
    assert!(text.contains("My second task"));
}

#[tokio::test]
async fn test_get_workspace_projects() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/projects"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "proj1", "name": "Project Alpha"},
                {"gid": "proj2", "name": "Project Beta"},
                {"gid": "proj3", "name": "1:1 with Alice"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::WorkspaceProjects, "ws123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Project Alpha"));
    assert!(text.contains("Project Beta"));
    assert!(text.contains("1:1 with Alice"));
}

// ============================================================================
// Additional Create Tests - Complete Coverage
// ============================================================================

#[tokio::test]
async fn test_create_project_from_template() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/project_templates/tmpl123/instantiateProject"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {
                "gid": "job123",
                "resource_type": "job",
                "status": "in_progress",
                "new_project": {"gid": "proj456", "name": "New Sprint"}
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::ProjectFromTemplate,
        template_gid: Some("tmpl123".to_string()),
        name: Some("New Sprint".to_string()),
        team_gid: Some("team1".to_string()),
        workspace_gid: None,
        project_gid: None,
        task_gid: None,
        parent_gid: None,
        requested_dates: None,
        requested_roles: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        text: None,
        custom_fields: None,
        source_gid: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("job123"));
}

#[tokio::test]
async fn test_create_portfolio() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/portfolios"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {"gid": "port123", "name": "Q1 Portfolio", "color": "blue"}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::Portfolio,
        workspace_gid: Some("ws123".to_string()),
        name: Some("Q1 Portfolio".to_string()),
        color: Some("blue".to_string()),
        public: Some(true),
        project_gid: None,
        task_gid: None,
        team_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        notes: None,
        html_notes: None,
        html_text: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        status_type: None,
        title: None,
        text: None,
        custom_fields: None,
        source_gid: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Q1 Portfolio"));
}

#[tokio::test]
async fn test_create_section() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/projects/proj123/sections"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {"gid": "sec123", "name": "New Section"}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::Section,
        project_gid: Some("proj123".to_string()),
        name: Some("New Section".to_string()),
        workspace_gid: None,
        task_gid: None,
        team_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        text: None,
        custom_fields: None,
        source_gid: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("New Section"));
}

#[tokio::test]
async fn test_create_status_update() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/status_updates"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {
                "gid": "status123",
                "title": "Week 1 Update",
                "text": "Everything on track",
                "status_type": "on_track"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::StatusUpdate,
        parent_gid: Some("proj123".to_string()),
        status_type: Some("on_track".to_string()),
        title: Some("Week 1 Update".to_string()),
        text: Some("Everything on track".to_string()),
        workspace_gid: None,
        project_gid: None,
        task_gid: None,
        team_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        name: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        custom_fields: None,
        source_gid: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Week 1 Update"));
    assert!(text.contains("on_track"));
}

#[tokio::test]
async fn test_create_tag() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tags"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {"gid": "tag123", "name": "Urgent", "color": "red"}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::Tag,
        workspace_gid: Some("ws123".to_string()),
        name: Some("Urgent".to_string()),
        color: Some("red".to_string()),
        notes: Some("High priority items".to_string()),
        project_gid: None,
        task_gid: None,
        team_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        html_notes: None,
        html_text: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        text: None,
        custom_fields: None,
        source_gid: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Urgent"));
}

// ============================================================================
// Additional Update Tests - Complete Coverage
// ============================================================================

#[tokio::test]
async fn test_update_project() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/projects/proj123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "proj123", "name": "Updated Project", "archived": true}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(UpdateParams {
        resource_type: UpdateResourceType::Project,
        gid: "proj123".to_string(),
        name: Some("Updated Project".to_string()),
        archived: Some(true),
        notes: None,
        html_notes: None,
        html_text: None,
        completed: None,
        due_on: None,
        start_on: None,
        assignee: None,
        color: None,
        privacy_setting: None,
        public: None,
        text: None,
        title: None,
        status_type: None,
        custom_fields: None,
        opt_fields: None,
    });

    let result = server.asana_update(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Updated Project"));
}

#[tokio::test]
async fn test_update_portfolio() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/portfolios/port123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "port123", "name": "Updated Portfolio", "color": "green"}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(UpdateParams {
        resource_type: UpdateResourceType::Portfolio,
        gid: "port123".to_string(),
        name: Some("Updated Portfolio".to_string()),
        color: Some("green".to_string()),
        public: Some(true),
        notes: None,
        html_notes: None,
        html_text: None,
        completed: None,
        due_on: None,
        start_on: None,
        assignee: None,
        archived: None,
        privacy_setting: None,
        text: None,
        title: None,
        status_type: None,
        custom_fields: None,
        opt_fields: None,
    });

    let result = server.asana_update(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Updated Portfolio"));
}

#[tokio::test]
async fn test_update_tag() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/tags/tag123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "tag123", "name": "Critical", "color": "red"}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(UpdateParams {
        resource_type: UpdateResourceType::Tag,
        gid: "tag123".to_string(),
        name: Some("Critical".to_string()),
        color: Some("red".to_string()),
        notes: None,
        html_notes: None,
        html_text: None,
        completed: None,
        due_on: None,
        start_on: None,
        assignee: None,
        archived: None,
        privacy_setting: None,
        public: None,
        text: None,
        title: None,
        status_type: None,
        custom_fields: None,
        opt_fields: None,
    });

    let result = server.asana_update(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Critical"));
}

#[tokio::test]
async fn test_update_comment() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/stories/story123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "story123", "text": "Updated comment text"}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(UpdateParams {
        resource_type: UpdateResourceType::Comment,
        gid: "story123".to_string(),
        text: Some("Updated comment text".to_string()),
        name: None,
        notes: None,
        html_notes: None,
        html_text: None,
        completed: None,
        due_on: None,
        start_on: None,
        assignee: None,
        color: None,
        archived: None,
        privacy_setting: None,
        public: None,
        title: None,
        status_type: None,
        custom_fields: None,
        opt_fields: None,
    });

    let result = server.asana_update(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Updated comment text"));
}

#[tokio::test]
async fn test_update_status_update() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/status_updates/status123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "status123",
                "title": "Week 2 Update",
                "text": "Still on track",
                "status_type": "on_track"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(UpdateParams {
        resource_type: UpdateResourceType::StatusUpdate,
        gid: "status123".to_string(),
        title: Some("Week 2 Update".to_string()),
        text: Some("Still on track".to_string()),
        status_type: Some("on_track".to_string()),
        name: None,
        notes: None,
        html_notes: None,
        html_text: None,
        completed: None,
        due_on: None,
        start_on: None,
        assignee: None,
        color: None,
        archived: None,
        privacy_setting: None,
        public: None,
        custom_fields: None,
        opt_fields: None,
    });

    let result = server.asana_update(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Week 2 Update"));
    assert!(text.contains("on_track"));
}

// ============================================================================
// Additional Link Tests - Complete Coverage
// ============================================================================

#[tokio::test]
async fn test_link_remove_task_from_project() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tasks/task123/removeProject"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Remove,
        relationship: RelationshipType::TaskProject,
        target_gid: "task123".to_string(),
        item_gid: Some("proj456".to_string()),
        item_gids: None,
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("success"));
}

#[tokio::test]
async fn test_link_add_task_tag() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tasks/task123/addTag"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Add,
        relationship: RelationshipType::TaskTag,
        target_gid: "task123".to_string(),
        item_gid: Some("tag456".to_string()),
        item_gids: None,
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Tag added"));
}

#[tokio::test]
async fn test_link_remove_task_tag() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tasks/task123/removeTag"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Remove,
        relationship: RelationshipType::TaskTag,
        target_gid: "task123".to_string(),
        item_gid: Some("tag456".to_string()),
        item_gids: None,
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Tag removed"));
}

#[tokio::test]
async fn test_link_set_task_parent() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tasks/task123/setParent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {"gid": "task123", "parent": {"gid": "parent456"}}
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Add,
        relationship: RelationshipType::TaskParent,
        target_gid: "task123".to_string(),
        item_gid: Some("parent456".to_string()),
        item_gids: None,
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("parent456"));
}

#[tokio::test]
async fn test_link_add_dependents() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tasks/task123/addDependents"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Add,
        relationship: RelationshipType::TaskDependent,
        target_gid: "task123".to_string(),
        item_gid: Some("dep456".to_string()),
        item_gids: None,
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Dependents added"));
}

#[tokio::test]
async fn test_link_add_task_follower() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tasks/task123/addFollowers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Add,
        relationship: RelationshipType::TaskFollower,
        target_gid: "task123".to_string(),
        item_gid: Some("user456".to_string()),
        item_gids: None,
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Followers added"));
}

#[tokio::test]
async fn test_link_add_portfolio_item() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/portfolios/port123/addItem"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Add,
        relationship: RelationshipType::PortfolioItem,
        target_gid: "port123".to_string(),
        item_gid: Some("proj456".to_string()),
        item_gids: None,
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Item added to portfolio"));
}

#[tokio::test]
async fn test_link_remove_portfolio_item() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/portfolios/port123/removeItem"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Remove,
        relationship: RelationshipType::PortfolioItem,
        target_gid: "port123".to_string(),
        item_gid: Some("proj456".to_string()),
        item_gids: None,
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Item removed from portfolio"));
}

#[tokio::test]
async fn test_link_add_portfolio_member() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/portfolios/port123/addMembers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Add,
        relationship: RelationshipType::PortfolioMember,
        target_gid: "port123".to_string(),
        item_gid: Some("user456".to_string()),
        item_gids: None,
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Members added to portfolio"));
}

#[tokio::test]
async fn test_link_add_project_member() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/projects/proj123/addMembers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Add,
        relationship: RelationshipType::ProjectMember,
        target_gid: "proj123".to_string(),
        item_gid: Some("user456".to_string()),
        item_gids: None,
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Members added to project"));
}

#[tokio::test]
async fn test_link_add_project_follower() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/projects/proj123/addFollowers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data": {}})))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(LinkParams {
        action: LinkAction::Add,
        relationship: RelationshipType::ProjectFollower,
        target_gid: "proj123".to_string(),
        item_gid: Some("user456".to_string()),
        item_gids: None,
        section_gid: None,
        insert_before: None,
        insert_after: None,
    });

    let result = server.asana_link(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Followers added to project"));
}

// ============================================================================
// User Tests
// ============================================================================

#[tokio::test]
async fn test_get_me() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/users/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "user123",
                "name": "Test User",
                "email": "test@example.com",
                "photo": {"image_128x128": "https://example.com/photo.png"}
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    // GID is ignored for Me resource type
    let result = server
        .asana_get(get_params(ResourceType::Me, "ignored"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Test User"));
    assert!(text.contains("test@example.com"));
}

#[tokio::test]
async fn test_get_user() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/users/user456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "user456",
                "name": "Other User",
                "email": "other@example.com"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::User, "user456"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Other User"));
    assert!(text.contains("other@example.com"));
}

#[tokio::test]
async fn test_get_workspace_users() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/users"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "user1", "name": "Alice", "email": "alice@example.com"},
                {"gid": "user2", "name": "Bob", "email": "bob@example.com"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::WorkspaceUsers, "ws123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Alice"));
    assert!(text.contains("Bob"));
}

// ============================================================================
// Team Tests
// ============================================================================

#[tokio::test]
async fn test_get_team() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/teams/team123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "team123",
                "name": "Engineering",
                "description": "The engineering team",
                "permalink_url": "https://app.asana.com/teams/team123"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::Team, "team123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Engineering"));
    assert!(text.contains("The engineering team"));
}

#[tokio::test]
async fn test_get_workspace_teams() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/teams"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "team1", "name": "Engineering"},
                {"gid": "team2", "name": "Design"},
                {"gid": "team3", "name": "Product"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::WorkspaceTeams, "ws123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Engineering"));
    assert!(text.contains("Design"));
    assert!(text.contains("Product"));
}

#[tokio::test]
async fn test_get_team_users() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/teams/team123/users"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "user1", "name": "Alice", "email": "alice@example.com"},
                {"gid": "user2", "name": "Charlie", "email": "charlie@example.com"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::TeamUsers, "team123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Alice"));
    assert!(text.contains("Charlie"));
}

// ============================================================================
// Custom Fields Tests
// ============================================================================

#[tokio::test]
async fn test_get_project_custom_fields() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/projects/proj123/custom_field_settings"))
        .and(NoOffset)
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {
                    "gid": "cfs1",
                    "custom_field": {
                        "gid": "cf1",
                        "name": "Priority",
                        "type": "enum",
                        "enum_options": [
                            {"gid": "opt1", "name": "High", "color": "red"},
                            {"gid": "opt2", "name": "Low", "color": "green"}
                        ]
                    },
                    "is_important": true
                },
                {
                    "gid": "cfs2",
                    "custom_field": {
                        "gid": "cf2",
                        "name": "Points",
                        "type": "number",
                        "precision": 0
                    },
                    "is_important": false
                }
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::ProjectCustomFields, "proj123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Priority"));
    assert!(text.contains("Points"));
    assert!(text.contains("High"));
    assert!(text.contains("Low"));
}

// ============================================================================
// Duplicate Tests
// ============================================================================

#[tokio::test]
async fn test_create_project_duplicate() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/projects/proj123/duplicate"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {
                "gid": "job456",
                "resource_type": "job",
                "status": "in_progress",
                "new_project": {"gid": "newproj789", "name": "Copy of Project"}
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::ProjectDuplicate,
        source_gid: Some("proj123".to_string()),
        name: Some("Copy of Project".to_string()),
        team_gid: Some("team1".to_string()),
        include: Some(vec!["members".to_string(), "task_notes".to_string()]),
        workspace_gid: None,
        project_gid: None,
        task_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        text: None,
        custom_fields: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("job456"));
    assert!(text.contains("newproj789"));
}

#[tokio::test]
async fn test_create_project_duplicate_requires_source_gid() {
    let mock_server = MockServer::start().await;
    let server = test_server(&mock_server.uri());

    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::ProjectDuplicate,
        source_gid: None, // Missing required field
        name: Some("Copy".to_string()),
        workspace_gid: None,
        project_gid: None,
        task_gid: None,
        team_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        text: None,
        custom_fields: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("source_gid is required"));
}

#[tokio::test]
async fn test_create_task_duplicate() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/tasks/task123/duplicate"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {
                "gid": "newtask456",
                "name": "Copy of Task",
                "completed": false
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::TaskDuplicate,
        source_gid: Some("task123".to_string()),
        name: Some("Copy of Task".to_string()),
        include: Some(vec!["subtasks".to_string(), "notes".to_string()]),
        workspace_gid: None,
        project_gid: None,
        task_gid: None,
        team_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        text: None,
        custom_fields: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("newtask456"));
    assert!(text.contains("Copy of Task"));
}

#[tokio::test]
async fn test_create_task_duplicate_requires_source_gid() {
    let mock_server = MockServer::start().await;
    let server = test_server(&mock_server.uri());

    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::TaskDuplicate,
        source_gid: None, // Missing required field
        name: Some("Copy".to_string()),
        workspace_gid: None,
        project_gid: None,
        task_gid: None,
        team_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        text: None,
        custom_fields: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message.contains("source_gid is required"));
}

// ============================================================================
// Search Tests
// ============================================================================

#[tokio::test]
async fn test_search_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/tasks/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "task1", "name": "Fix login bug", "completed": false},
                {"gid": "task2", "name": "Login UI improvements", "completed": true}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(TaskSearchParams {
        workspace_gid: Some("ws123".to_string()),
        text: Some("login".to_string()),
        assignee: None,
        projects: None,
        tags: None,
        sections: None,
        completed: None,
        due_on: None,
        due_on_before: None,
        due_on_after: None,
        start_on: None,
        start_on_before: None,
        start_on_after: None,
        modified_at_after: None,
        modified_at_before: None,
        portfolios: None,
        sort_by: None,
        sort_ascending: None,
        detail_level: DetailLevel::Default,
        extra_fields: None,
        opt_fields: None,
    });

    let result = server.asana_task_search(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Fix login bug"));
    assert!(text.contains("Login UI improvements"));
}

#[tokio::test]
async fn test_search_with_assignee_me() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/tasks/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "task1", "name": "My assigned task", "assignee": {"gid": "me", "name": "Me"}}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(TaskSearchParams {
        workspace_gid: Some("ws123".to_string()),
        assignee: Some("me".to_string()),
        text: None,
        projects: None,
        tags: None,
        sections: None,
        completed: None,
        due_on: None,
        due_on_before: None,
        due_on_after: None,
        start_on: None,
        start_on_before: None,
        start_on_after: None,
        modified_at_after: None,
        modified_at_before: None,
        portfolios: None,
        sort_by: None,
        sort_ascending: None,
        detail_level: DetailLevel::Default,
        extra_fields: None,
        opt_fields: None,
    });

    let result = server.asana_task_search(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("My assigned task"));
}

#[tokio::test]
async fn test_search_with_filters() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/tasks/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "task1", "name": "Due soon task", "due_on": "2024-01-15", "completed": false}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(TaskSearchParams {
        workspace_gid: Some("ws123".to_string()),
        completed: Some(false),
        due_on_before: Some("2024-01-31".to_string()),
        due_on_after: Some("2024-01-01".to_string()),
        projects: Some(vec!["proj1".to_string()]),
        tags: Some(vec!["tag1".to_string(), "tag2".to_string()]),
        sort_by: Some("due_date".to_string()),
        sort_ascending: Some(true),
        text: None,
        assignee: None,
        sections: None,
        due_on: None,
        start_on: None,
        start_on_before: None,
        start_on_after: None,
        modified_at_after: None,
        modified_at_before: None,
        portfolios: None,
        detail_level: DetailLevel::Default,
        extra_fields: None,
        opt_fields: None,
    });

    let result = server.asana_task_search(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Due soon task"));
}

#[tokio::test]
async fn test_search_unassigned() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/tasks/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "task1", "name": "Unassigned task", "assignee": null}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(TaskSearchParams {
        workspace_gid: Some("ws123".to_string()),
        assignee: Some("null".to_string()), // Special value for unassigned
        text: None,
        projects: None,
        tags: None,
        sections: None,
        completed: None,
        due_on: None,
        due_on_before: None,
        due_on_after: None,
        start_on: None,
        start_on_before: None,
        start_on_after: None,
        modified_at_after: None,
        modified_at_before: None,
        portfolios: None,
        sort_by: None,
        sort_ascending: None,
        detail_level: DetailLevel::Default,
        extra_fields: None,
        opt_fields: None,
    });

    let result = server.asana_task_search(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Unassigned task"));
}

// ============================================================================
// Resource Search (Typeahead) Tests
// ============================================================================

#[tokio::test]
async fn test_resource_search_project() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/typeahead"))
        .and(query_param("query", "CloudSmith"))
        .and(query_param("resource_type", "project"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "proj1", "name": "CloudSmith Backend", "resource_type": "project"},
                {"gid": "proj2", "name": "CloudSmith Frontend", "resource_type": "project"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(ResourceSearchParams {
        query: Some("CloudSmith".to_string()),
        resource_type: SearchableResourceType::Project,
        workspace_gid: Some("ws123".to_string()),
        count: None,
    });

    let result = server.asana_resource_search(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("CloudSmith Backend"));
    assert!(text.contains("CloudSmith Frontend"));
}

#[tokio::test]
async fn test_resource_search_template() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/typeahead"))
        .and(query_param("query", "Sprint"))
        .and(query_param("resource_type", "project_template"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "tmpl1", "name": "Sprint Planning Template", "resource_type": "project_template"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(ResourceSearchParams {
        query: Some("Sprint".to_string()),
        resource_type: SearchableResourceType::ProjectTemplate,
        workspace_gid: Some("ws123".to_string()),
        count: None,
    });

    let result = server.asana_resource_search(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Sprint Planning Template"));
}

#[tokio::test]
async fn test_resource_search_user() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/typeahead"))
        .and(query_param("query", "John"))
        .and(query_param("resource_type", "user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "user1", "name": "John Smith", "resource_type": "user"},
                {"gid": "user2", "name": "Johnny Appleseed", "resource_type": "user"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(ResourceSearchParams {
        query: Some("John".to_string()),
        resource_type: SearchableResourceType::User,
        workspace_gid: Some("ws123".to_string()),
        count: Some(10),
    });

    let result = server.asana_resource_search(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("John Smith"));
    assert!(text.contains("Johnny Appleseed"));
}

#[tokio::test]
async fn test_resource_search_requires_query() {
    let mock_server = MockServer::start().await;
    let server = test_server(&mock_server.uri());

    let params = Parameters(ResourceSearchParams {
        query: None, // Missing query
        resource_type: SearchableResourceType::Project,
        workspace_gid: Some("ws123".to_string()),
        count: None,
    });

    let result = server.asana_resource_search(params).await;
    assert!(result.is_err());
}

// ============================================================================
// Project Brief Tests (Key Resources on Overview tab, NOT the Note tab)
// ============================================================================

#[tokio::test]
async fn test_get_project_brief() {
    let mock_server = MockServer::start().await;

    // Uses brief GID directly (matches asana_update behavior)
    Mock::given(method("GET"))
        .and(path("/project_briefs/brief123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "brief123",
                "text": "This is the project overview",
                "html_text": "<p>This is the project overview</p>",
                "permalink_url": "https://app.asana.com/..."
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::ProjectBrief, "brief123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("This is the project overview"));
    assert!(text.contains("brief123"));
}

#[tokio::test]
async fn test_get_project_project_brief() {
    let mock_server = MockServer::start().await;

    // Fetches project with opt_fields=project_brief to discover the brief
    Mock::given(method("GET"))
        .and(path("/projects/proj123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "proj123",
                "name": "Test Project",
                "project_brief": {
                    "gid": "brief123",
                    "text": "Project brief from project",
                    "html_text": "<p>Project brief from project</p>",
                    "permalink_url": "https://app.asana.com/..."
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::ProjectProjectBrief, "proj123"))
        .await
        .unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Project brief from project"));
    assert!(text.contains("brief123"));
}

#[tokio::test]
async fn test_get_project_project_brief_no_brief() {
    let mock_server = MockServer::start().await;

    // Project without a brief returns null for project_brief
    Mock::given(method("GET"))
        .and(path("/projects/proj456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "proj456",
                "name": "Test Project Without Brief",
                "project_brief": null
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let result = server
        .asana_get(get_params(ResourceType::ProjectProjectBrief, "proj456"))
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_project_brief() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/projects/proj123/project_briefs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {
                "gid": "brief456",
                "text": "New project brief content"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::ProjectBrief,
        project_gid: Some("proj123".to_string()),
        text: Some("New project brief content".to_string()),
        workspace_gid: None,
        task_gid: None,
        team_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        name: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        custom_fields: None,
        source_gid: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("brief456"));
    assert!(text.contains("New project brief content"));
}

#[tokio::test]
async fn test_update_project_brief() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/project_briefs/brief123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "gid": "brief123",
                "text": "Updated project brief",
                "html_text": "<p>Updated project brief</p>"
            }
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(UpdateParams {
        resource_type: UpdateResourceType::ProjectBrief,
        gid: "brief123".to_string(),
        text: Some("Updated project brief".to_string()),
        name: None,
        notes: None,
        html_notes: None,
        html_text: None,
        completed: None,
        due_on: None,
        start_on: None,
        assignee: None,
        color: None,
        archived: None,
        privacy_setting: None,
        public: None,
        title: None,
        status_type: None,
        custom_fields: None,
        opt_fields: None,
    });

    let result = server.asana_update(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Updated project brief"));
}

#[tokio::test]
async fn test_create_project_brief_requires_project_gid() {
    let mock_server = MockServer::start().await;
    let server = test_server(&mock_server.uri());

    let params = Parameters(CreateParams {
        resource_type: CreateResourceType::ProjectBrief,
        project_gid: None, // Missing project_gid
        text: Some("Some content".to_string()),
        workspace_gid: None,
        task_gid: None,
        team_gid: None,
        parent_gid: None,
        template_gid: None,
        requested_dates: None,
        requested_roles: None,
        name: None,
        notes: None,
        html_notes: None,
        html_text: None,
        color: None,
        due_on: None,
        start_on: None,
        assignee: None,
        privacy_setting: None,
        public: None,
        status_type: None,
        title: None,
        custom_fields: None,
        source_gid: None,
        include: None,
        opt_fields: None,
    });

    let result = server.asana_create(params).await;
    assert!(result.is_err());
}

// ============================================================================
// Resource Search Additional Coverage Tests
// ============================================================================

#[tokio::test]
async fn test_resource_search_uses_default_workspace() {
    let mock_server = MockServer::start().await;

    // When workspace_gid is None, should use default workspace
    Mock::given(method("GET"))
        .and(path("/workspaces/default-ws/typeahead"))
        .and(query_param("query", "Test"))
        .and(query_param("resource_type", "project"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "proj1", "name": "Test Project", "resource_type": "project"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri()).with_default_workspace("default-ws");
    let params = Parameters(ResourceSearchParams {
        query: Some("Test".to_string()),
        resource_type: SearchableResourceType::Project,
        workspace_gid: None, // Should use default
        count: None,
    });

    let result = server.asana_resource_search(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Test Project"));
}

#[tokio::test]
async fn test_resource_search_count_defaults_to_20() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/typeahead"))
        .and(query_param("count", "20")) // Default count
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(ResourceSearchParams {
        query: Some("Test".to_string()),
        resource_type: SearchableResourceType::Project,
        workspace_gid: Some("ws123".to_string()),
        count: None, // Should default to 20
    });

    let result = server.asana_resource_search(params).await.unwrap();
    assert!(!result.content.is_empty());
}

#[tokio::test]
async fn test_resource_search_count_clamped_to_100() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/typeahead"))
        .and(query_param("count", "100")) // Should be clamped to 100
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(ResourceSearchParams {
        query: Some("Test".to_string()),
        resource_type: SearchableResourceType::Project,
        workspace_gid: Some("ws123".to_string()),
        count: Some(500), // Request 500, should be clamped to 100
    });

    let result = server.asana_resource_search(params).await.unwrap();
    assert!(!result.content.is_empty());
}

#[tokio::test]
async fn test_searchable_resource_type_as_str() {
    // Test all enum variants produce correct API strings
    assert_eq!(SearchableResourceType::Project.as_str(), "project");
    assert_eq!(
        SearchableResourceType::ProjectTemplate.as_str(),
        "project_template"
    );
    assert_eq!(SearchableResourceType::Portfolio.as_str(), "portfolio");
    assert_eq!(SearchableResourceType::User.as_str(), "user");
    assert_eq!(SearchableResourceType::Team.as_str(), "team");
    assert_eq!(SearchableResourceType::Tag.as_str(), "tag");
    assert_eq!(SearchableResourceType::Goal.as_str(), "goal");
}

#[tokio::test]
async fn test_resource_search_portfolio() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/typeahead"))
        .and(query_param("resource_type", "portfolio"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "port1", "name": "Q1 Portfolio", "resource_type": "portfolio"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(ResourceSearchParams {
        query: Some("Q1".to_string()),
        resource_type: SearchableResourceType::Portfolio,
        workspace_gid: Some("ws123".to_string()),
        count: None,
    });

    let result = server.asana_resource_search(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Q1 Portfolio"));
}

#[tokio::test]
async fn test_resource_search_team() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/typeahead"))
        .and(query_param("resource_type", "team"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "team1", "name": "Engineering", "resource_type": "team"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(ResourceSearchParams {
        query: Some("Eng".to_string()),
        resource_type: SearchableResourceType::Team,
        workspace_gid: Some("ws123".to_string()),
        count: None,
    });

    let result = server.asana_resource_search(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Engineering"));
}

#[tokio::test]
async fn test_resource_search_tag() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/typeahead"))
        .and(query_param("resource_type", "tag"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "tag1", "name": "urgent", "resource_type": "tag"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(ResourceSearchParams {
        query: Some("urgent".to_string()),
        resource_type: SearchableResourceType::Tag,
        workspace_gid: Some("ws123".to_string()),
        count: None,
    });

    let result = server.asana_resource_search(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("urgent"));
}

#[tokio::test]
async fn test_resource_search_goal() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/workspaces/ws123/typeahead"))
        .and(query_param("resource_type", "goal"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"gid": "goal1", "name": "Increase Revenue", "resource_type": "goal"}
            ],
            "next_page": null
        })))
        .mount(&mock_server)
        .await;

    let server = test_server(&mock_server.uri());
    let params = Parameters(ResourceSearchParams {
        query: Some("Revenue".to_string()),
        resource_type: SearchableResourceType::Goal,
        workspace_gid: Some("ws123".to_string()),
        count: None,
    });

    let result = server.asana_resource_search(params).await.unwrap();
    let text = get_response_text(&result);

    assert!(text.contains("Increase Revenue"));
}
