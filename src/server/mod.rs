//! MCP server implementation for Asana.

mod fields;
mod helpers;
pub mod params;

use crate::client::AsanaClient;
use crate::types::{
    FavoriteError, FavoriteItem, FavoritesResponse, Job, PortfolioItem, PortfolioItemExpanded,
    PortfolioWithItems, Resource, Story, TaskDependency, TaskWithContext,
};
use crate::Error;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ErrorData as McpError, Implementation, ProtocolVersion, ServerCapabilities,
    ServerInfo,
};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};

use fields::*;
use helpers::*;
pub use params::*;

/// MCP server for Asana operations.
#[derive(Debug, Clone)]
pub struct AsanaServer {
    client: AsanaClient,
    tool_router: ToolRouter<AsanaServer>,
}

#[tool_router]
impl AsanaServer {
    /// Create a new Asana MCP server.
    pub fn new() -> Result<Self, Error> {
        let client = AsanaClient::from_env()?;
        Ok(Self {
            client,
            tool_router: Self::tool_router(),
        })
    }

    /// Create a server with a custom client (for testing).
    #[cfg(test)]
    pub(crate) fn with_client(client: AsanaClient) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }

    /// List all workspaces accessible to the authenticated user.
    #[tool(description = "List all Asana workspaces accessible to the authenticated user")]
    async fn asana_workspaces(
        &self,
        _params: Parameters<WorkspacesParams>,
    ) -> Result<CallToolResult, McpError> {
        let workspaces: Vec<Resource> = self
            .client
            .get_all("/workspaces", &[("opt_fields", WORKSPACE_FIELDS)])
            .await
            .map_err(|e| error_to_mcp("Failed to list workspaces", e))?;

        json_response(&workspaces)
    }

    /// Universal get tool for fetching Asana resources.
    #[tool(description = "Get any Asana resource by type and GID. Supports:\n\
            - project: Get a project (gid = project GID)\n\
            - portfolio: Get a portfolio with nested items (gid = portfolio GID, use depth to control recursion)\n\
            - task: Get a task with context (gid = task GID, use include_* flags)\n\
            - my_tasks: Get tasks assigned to current user (gid = workspace GID)\n\
            - workspace_favorites: Get user's favorites (gid = workspace GID)\n\
            - workspace_projects: List all projects in workspace (gid = workspace GID)\n\
            - project_tasks: Get all tasks from a project/portfolio (gid = project/portfolio GID, use subtask_depth)\n\
            - task_subtasks: Get subtasks of a task (gid = task GID)\n\
            - task_comments: Get comments on a task (gid = task GID)\n\
            - project_status_updates: Get status history (gid = project/portfolio GID)\n\
            - all_workspaces: List all workspaces (gid is ignored)\n\
            - workspace: Get a single workspace (gid = workspace GID)\n\
            - workspace_templates: List templates in a workspace (gid = workspace GID)\n\
            - project_template: Get a single template (gid = template GID)\n\
            - project_sections: List sections in a project (gid = project GID)\n\
            - section: Get a single section (gid = section GID)\n\
            - workspace_tags: List tags in a workspace (gid = workspace GID)\n\
            - tag: Get a single tag (gid = tag GID)\n\
            - me: Get current authenticated user (gid ignored)\n\
            - user: Get a user (gid = user GID)\n\
            - workspace_users: List users in workspace (gid = workspace GID)\n\
            - team: Get a team (gid = team GID)\n\
            - workspace_teams: List teams in workspace (gid = workspace GID)\n\
            - team_users: List users in a team (gid = team GID)\n\
            - project_custom_fields: Get custom fields for a project (gid = project GID)\n\n\
            Depth parameters: -1 = unlimited, 0 = none, N = N levels")]
    async fn asana_get(&self, params: Parameters<GetParams>) -> Result<CallToolResult, McpError> {
        let p = params.0;

        match p.resource_type {
            ResourceType::Project => {
                let project: Resource = self
                    .client
                    .get(
                        &format!("/projects/{}", p.gid),
                        &[("opt_fields", PROJECT_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get project", e))?;
                json_response(&project)
            }

            ResourceType::Portfolio => {
                let depth = depth_to_option(p.depth.unwrap_or(0));
                let portfolio = self
                    .get_portfolio_recursive(&p.gid, depth)
                    .await
                    .map_err(|e| error_to_mcp("Failed to get portfolio", e))?;
                json_response(&portfolio)
            }

            ResourceType::Task => {
                let task = self
                    .get_task_with_context(
                        &p.gid,
                        p.include_subtasks.unwrap_or(true),
                        p.include_dependencies.unwrap_or(true),
                        p.include_comments.unwrap_or(true),
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get task", e))?;
                json_response(&task)
            }

            ResourceType::WorkspaceFavorites => {
                let depth = depth_to_option(p.depth.unwrap_or(0));
                let include_projects = p.include_projects.unwrap_or(true);
                let include_portfolios = p.include_portfolios.unwrap_or(true);

                let favorites: Vec<FavoriteItem> = self
                    .client
                    .get_all(
                        "/users/me/favorites",
                        &[
                            ("workspace", p.gid.as_str()),
                            ("opt_fields", "gid,resource_type,name"),
                        ],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get favorites", e))?;

                let mut projects = Vec::new();
                let mut portfolios = Vec::new();
                let mut errors = Vec::new();

                for item in favorites {
                    match item.resource_type.as_str() {
                        "project" if include_projects => {
                            match self
                                .client
                                .get::<Resource>(
                                    &format!("/projects/{}", item.gid),
                                    &[("opt_fields", PROJECT_FIELDS)],
                                )
                                .await
                            {
                                Ok(project) => projects.push(project),
                                Err(e) => errors.push(FavoriteError {
                                    item,
                                    error: e.to_string(),
                                }),
                            }
                        }
                        "portfolio" if include_portfolios => {
                            match self.get_portfolio_recursive(&item.gid, depth).await {
                                Ok(portfolio) => portfolios.push(portfolio),
                                Err(e) => errors.push(FavoriteError {
                                    item,
                                    error: e.to_string(),
                                }),
                            }
                        }
                        _ => {}
                    }
                }

                json_response(&FavoritesResponse {
                    projects,
                    portfolios,
                    errors,
                })
            }

            ResourceType::ProjectTasks => {
                let subtask_depth = p
                    .subtask_depth
                    .map(|d| if d < 0 { None } else { Some(d) })
                    .unwrap_or(Some(0));
                let portfolio_depth = Some(p.depth.unwrap_or(0));

                let tasks = self
                    .get_tasks_recursive(&p.gid, subtask_depth, portfolio_depth)
                    .await
                    .map_err(|e| error_to_mcp("Failed to get tasks", e))?;
                json_response(&tasks)
            }

            ResourceType::TaskSubtasks => {
                let subtasks: Vec<Resource> = self
                    .client
                    .get_all(
                        &format!("/tasks/{}/subtasks", p.gid),
                        &[("opt_fields", SUBTASK_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get subtasks", e))?;
                json_response(&subtasks)
            }

            ResourceType::TaskComments => {
                let stories: Vec<Story> = self
                    .client
                    .get_all(
                        &format!("/tasks/{}/stories", p.gid),
                        &[("opt_fields", STORY_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get comments", e))?;
                let comments: Vec<_> = stories.into_iter().filter(|s| s.is_comment()).collect();
                json_response(&comments)
            }

            ResourceType::ProjectStatusUpdates => {
                // Try as project first, then as portfolio
                let project_result: Result<Vec<Resource>, _> = self
                    .client
                    .get_all(
                        &format!("/projects/{}/status_updates", p.gid),
                        &[("opt_fields", STATUS_UPDATE_FIELDS)],
                    )
                    .await;

                let updates = match project_result {
                    Ok(updates) => updates,
                    Err(Error::NotFound(_)) => self
                        .client
                        .get_all(
                            &format!("/portfolios/{}/status_updates", p.gid),
                            &[("opt_fields", STATUS_UPDATE_FIELDS)],
                        )
                        .await
                        .map_err(|e| error_to_mcp("Failed to get status updates", e))?,
                    Err(e) => return Err(error_to_mcp("Failed to get status updates", e)),
                };
                json_response(&updates)
            }

            ResourceType::AllWorkspaces => {
                let workspaces: Vec<Resource> = self
                    .client
                    .get_all("/workspaces", &[("opt_fields", WORKSPACE_FIELDS)])
                    .await
                    .map_err(|e| error_to_mcp("Failed to list workspaces", e))?;
                json_response(&workspaces)
            }

            ResourceType::Workspace => {
                let workspace: Resource = self
                    .client
                    .get(
                        &format!("/workspaces/{}", p.gid),
                        &[("opt_fields", WORKSPACE_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get workspace", e))?;
                json_response(&workspace)
            }

            ResourceType::WorkspaceTemplates => {
                let templates: Vec<Resource> = self
                    .client
                    .get_all(
                        &format!("/workspaces/{}/project_templates", p.gid),
                        &[("opt_fields", TEMPLATE_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to list project templates", e))?;
                json_response(&templates)
            }

            ResourceType::ProjectTemplate => {
                let template: Resource = self
                    .client
                    .get(
                        &format!("/project_templates/{}", p.gid),
                        &[("opt_fields", TEMPLATE_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get project template", e))?;
                json_response(&template)
            }

            ResourceType::ProjectSections => {
                let sections: Vec<Resource> = self
                    .client
                    .get_all(
                        &format!("/projects/{}/sections", p.gid),
                        &[("opt_fields", SECTION_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to list sections", e))?;
                json_response(&sections)
            }

            ResourceType::Section => {
                let section: Resource = self
                    .client
                    .get(
                        &format!("/sections/{}", p.gid),
                        &[("opt_fields", SECTION_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get section", e))?;
                json_response(&section)
            }

            ResourceType::WorkspaceTags => {
                let tags: Vec<Resource> = self
                    .client
                    .get_all(
                        &format!("/workspaces/{}/tags", p.gid),
                        &[("opt_fields", TAG_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to list tags", e))?;
                json_response(&tags)
            }

            ResourceType::Tag => {
                let tag: Resource = self
                    .client
                    .get(&format!("/tags/{}", p.gid), &[("opt_fields", TAG_FIELDS)])
                    .await
                    .map_err(|e| error_to_mcp("Failed to get tag", e))?;
                json_response(&tag)
            }

            ResourceType::MyTasks => {
                // First get the user's task list for this workspace
                let task_list: Resource = self
                    .client
                    .get(
                        "/users/me/user_task_list",
                        &[("workspace", p.gid.as_str()), ("opt_fields", "gid")],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get user task list", e))?;

                // Then get tasks from that list
                let tasks: Vec<Resource> = self
                    .client
                    .get_all(
                        &format!("/user_task_lists/{}/tasks", task_list.gid),
                        &[("opt_fields", RECURSIVE_TASK_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get tasks", e))?;
                json_response(&tasks)
            }

            ResourceType::WorkspaceProjects => {
                let projects: Vec<Resource> = self
                    .client
                    .get_all(
                        &format!("/workspaces/{}/projects", p.gid),
                        &[("opt_fields", PROJECT_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get projects", e))?;
                json_response(&projects)
            }

            ResourceType::Me => {
                let user: Resource = self
                    .client
                    .get("/users/me", &[("opt_fields", USER_FIELDS)])
                    .await
                    .map_err(|e| error_to_mcp("Failed to get current user", e))?;
                json_response(&user)
            }

            ResourceType::User => {
                let user: Resource = self
                    .client
                    .get(&format!("/users/{}", p.gid), &[("opt_fields", USER_FIELDS)])
                    .await
                    .map_err(|e| error_to_mcp("Failed to get user", e))?;
                json_response(&user)
            }

            ResourceType::WorkspaceUsers => {
                let users: Vec<Resource> = self
                    .client
                    .get_all(
                        &format!("/workspaces/{}/users", p.gid),
                        &[("opt_fields", USER_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get users", e))?;
                json_response(&users)
            }

            ResourceType::Team => {
                let team: Resource = self
                    .client
                    .get(&format!("/teams/{}", p.gid), &[("opt_fields", TEAM_FIELDS)])
                    .await
                    .map_err(|e| error_to_mcp("Failed to get team", e))?;
                json_response(&team)
            }

            ResourceType::WorkspaceTeams => {
                let teams: Vec<Resource> = self
                    .client
                    .get_all(
                        &format!("/workspaces/{}/teams", p.gid),
                        &[("opt_fields", TEAM_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get teams", e))?;
                json_response(&teams)
            }

            ResourceType::TeamUsers => {
                let users: Vec<Resource> = self
                    .client
                    .get_all(
                        &format!("/teams/{}/users", p.gid),
                        &[("opt_fields", USER_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get team users", e))?;
                json_response(&users)
            }

            ResourceType::ProjectCustomFields => {
                let settings: Vec<Resource> = self
                    .client
                    .get_all(
                        &format!("/projects/{}/custom_field_settings", p.gid),
                        &[("opt_fields", CUSTOM_FIELD_SETTINGS_FIELDS)],
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to get custom field settings", e))?;
                json_response(&settings)
            }
        }
    }

    /// Create Asana resources.
    #[tool(description = "Create a new Asana resource. Supports:\n\
            - task: Create a task (workspace_gid or project_gid required)\n\
            - subtask: Create a subtask (task_gid = parent task)\n\
            - project: Create a project (workspace_gid or team_gid required)\n\
            - project_from_template: Instantiate from template (template_gid required)\n\
            - portfolio: Create a portfolio (workspace_gid required)\n\
            - section: Create a section in a project (project_gid required)\n\
            - comment: Add a comment to a task (task_gid required)\n\
            - status_update: Create a status update (parent_gid = project/portfolio)\n\
            - tag: Create a tag (workspace_gid required)\n\
            - project_duplicate: Duplicate a project (source_gid, name required; include[] for options)\n\
            - task_duplicate: Duplicate a task (source_gid, name required; include[] for options)")]
    async fn asana_create(
        &self,
        params: Parameters<CreateParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;

        match p.resource_type {
            CreateResourceType::Task => {
                let mut data = serde_json::Map::new();
                if let Some(name) = p.name {
                    data.insert("name".to_string(), serde_json::json!(name));
                }
                if let Some(ws) = p.workspace_gid {
                    data.insert("workspace".to_string(), serde_json::json!(ws));
                }
                if let Some(proj) = p.project_gid {
                    data.insert("projects".to_string(), serde_json::json!([proj]));
                }
                if let Some(assignee) = p.assignee {
                    data.insert("assignee".to_string(), serde_json::json!(assignee));
                }
                if let Some(due_on) = p.due_on {
                    data.insert("due_on".to_string(), serde_json::json!(due_on));
                }
                if let Some(start_on) = p.start_on {
                    data.insert("start_on".to_string(), serde_json::json!(start_on));
                }
                if let Some(notes) = p.notes {
                    data.insert("notes".to_string(), serde_json::json!(notes));
                }
                if let Some(html_notes) = p.html_notes {
                    data.insert("html_notes".to_string(), serde_json::json!(html_notes));
                }
                if let Some(cf) = p.custom_fields {
                    data.insert("custom_fields".to_string(), serde_json::json!(cf));
                }

                let body = serde_json::json!({"data": data});
                let task: Resource = self
                    .client
                    .post("/tasks", &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to create task", e))?;
                json_response(&task)
            }

            CreateResourceType::Subtask => {
                let task_gid = p
                    .task_gid
                    .ok_or_else(|| validation_error("task_gid is required for subtask"))?;
                let mut data = serde_json::Map::new();
                if let Some(name) = p.name {
                    data.insert("name".to_string(), serde_json::json!(name));
                }
                if let Some(assignee) = p.assignee {
                    data.insert("assignee".to_string(), serde_json::json!(assignee));
                }
                if let Some(due_on) = p.due_on {
                    data.insert("due_on".to_string(), serde_json::json!(due_on));
                }
                if let Some(start_on) = p.start_on {
                    data.insert("start_on".to_string(), serde_json::json!(start_on));
                }
                if let Some(notes) = p.notes {
                    data.insert("notes".to_string(), serde_json::json!(notes));
                }
                if let Some(html_notes) = p.html_notes {
                    data.insert("html_notes".to_string(), serde_json::json!(html_notes));
                }
                if let Some(cf) = p.custom_fields {
                    data.insert("custom_fields".to_string(), serde_json::json!(cf));
                }

                let body = serde_json::json!({"data": data});
                let task: Resource = self
                    .client
                    .post(&format!("/tasks/{}/subtasks", task_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to create subtask", e))?;
                json_response(&task)
            }

            CreateResourceType::Project => {
                let name = p
                    .name
                    .ok_or_else(|| validation_error("name is required for project"))?;
                let mut data = serde_json::Map::new();
                data.insert("name".to_string(), serde_json::json!(name));
                if let Some(ws) = p.workspace_gid {
                    data.insert("workspace".to_string(), serde_json::json!(ws));
                }
                if let Some(team) = p.team_gid {
                    data.insert("team".to_string(), serde_json::json!(team));
                }
                if let Some(color) = p.color {
                    data.insert("color".to_string(), serde_json::json!(color));
                }
                if let Some(notes) = p.notes {
                    data.insert("notes".to_string(), serde_json::json!(notes));
                }
                if let Some(html_notes) = p.html_notes {
                    data.insert("html_notes".to_string(), serde_json::json!(html_notes));
                }
                if let Some(due_on) = p.due_on {
                    data.insert("due_on".to_string(), serde_json::json!(due_on));
                }
                if let Some(start_on) = p.start_on {
                    data.insert("start_on".to_string(), serde_json::json!(start_on));
                }
                if let Some(privacy) = p.privacy_setting {
                    data.insert("privacy_setting".to_string(), serde_json::json!(privacy));
                }

                let body = serde_json::json!({"data": data});
                let project: Resource = self
                    .client
                    .post("/projects", &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to create project", e))?;
                json_response(&project)
            }

            CreateResourceType::ProjectFromTemplate => {
                let template_gid = p
                    .template_gid
                    .ok_or_else(|| validation_error("template_gid is required"))?;
                let name = p.name.ok_or_else(|| validation_error("name is required"))?;

                let mut data = serde_json::Map::new();
                data.insert("name".to_string(), serde_json::json!(name));
                if let Some(team) = p.team_gid {
                    data.insert("team".to_string(), serde_json::json!(team));
                }
                if let Some(public) = p.public {
                    data.insert("public".to_string(), serde_json::json!(public));
                }
                if let Some(dates) = p.requested_dates {
                    data.insert("requested_dates".to_string(), serde_json::json!(dates));
                }
                if let Some(roles) = p.requested_roles {
                    data.insert("requested_roles".to_string(), serde_json::json!(roles));
                }

                let body = serde_json::json!({"data": data});
                let job: Job = self
                    .client
                    .post(
                        &format!("/project_templates/{}/instantiateProject", template_gid),
                        &body,
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to instantiate project from template", e))?;
                json_response(&job)
            }

            CreateResourceType::Portfolio => {
                let workspace_gid = p
                    .workspace_gid
                    .ok_or_else(|| validation_error("workspace_gid is required for portfolio"))?;
                let name = p
                    .name
                    .ok_or_else(|| validation_error("name is required for portfolio"))?;

                let mut data = serde_json::Map::new();
                data.insert("name".to_string(), serde_json::json!(name));
                data.insert("workspace".to_string(), serde_json::json!(workspace_gid));
                if let Some(color) = p.color {
                    data.insert("color".to_string(), serde_json::json!(color));
                }
                if let Some(public) = p.public {
                    data.insert("public".to_string(), serde_json::json!(public));
                }

                let body = serde_json::json!({"data": data});
                let portfolio: Resource = self
                    .client
                    .post("/portfolios", &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to create portfolio", e))?;
                json_response(&portfolio)
            }

            CreateResourceType::Section => {
                let project_gid = p
                    .project_gid
                    .ok_or_else(|| validation_error("project_gid is required for section"))?;
                let name = p
                    .name
                    .ok_or_else(|| validation_error("name is required for section"))?;

                let body = serde_json::json!({"data": {"name": name}});
                let section: Resource = self
                    .client
                    .post(&format!("/projects/{}/sections", project_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to create section", e))?;
                json_response(&section)
            }

            CreateResourceType::Comment => {
                let task_gid = p
                    .task_gid
                    .ok_or_else(|| validation_error("task_gid is required for comment"))?;
                let text = p
                    .text
                    .or(p.notes)
                    .ok_or_else(|| validation_error("text or notes is required for comment"))?;

                let body = serde_json::json!({"data": {"text": text}});
                let story: Resource = self
                    .client
                    .post(&format!("/tasks/{}/stories", task_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to create comment", e))?;
                json_response(&story)
            }

            CreateResourceType::StatusUpdate => {
                let parent_gid = p
                    .parent_gid
                    .ok_or_else(|| validation_error("parent_gid is required for status update"))?;
                let status_type = p
                    .status_type
                    .ok_or_else(|| validation_error("status_type is required for status update"))?;

                let mut data = serde_json::Map::new();
                data.insert("parent".to_string(), serde_json::json!(parent_gid));
                data.insert("status_type".to_string(), serde_json::json!(status_type));
                if let Some(title) = p.title {
                    data.insert("title".to_string(), serde_json::json!(title));
                }
                if let Some(text) = p.text {
                    data.insert("text".to_string(), serde_json::json!(text));
                }

                let body = serde_json::json!({"data": data});
                let status: Resource = self
                    .client
                    .post("/status_updates", &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to create status update", e))?;
                json_response(&status)
            }

            CreateResourceType::Tag => {
                let workspace_gid = p
                    .workspace_gid
                    .ok_or_else(|| validation_error("workspace_gid is required for tag"))?;
                let name = p
                    .name
                    .ok_or_else(|| validation_error("name is required for tag"))?;

                let mut data = serde_json::Map::new();
                data.insert("name".to_string(), serde_json::json!(name));
                data.insert("workspace".to_string(), serde_json::json!(workspace_gid));
                if let Some(color) = p.color {
                    data.insert("color".to_string(), serde_json::json!(color));
                }
                if let Some(notes) = p.notes {
                    data.insert("notes".to_string(), serde_json::json!(notes));
                }

                let body = serde_json::json!({"data": data});
                let tag: Resource = self
                    .client
                    .post("/tags", &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to create tag", e))?;
                json_response(&tag)
            }

            CreateResourceType::ProjectDuplicate => {
                let source_gid = p.source_gid.ok_or_else(|| {
                    validation_error("source_gid is required for project_duplicate")
                })?;
                let name = p
                    .name
                    .ok_or_else(|| validation_error("name is required for project_duplicate"))?;

                let mut data = serde_json::Map::new();
                data.insert("name".to_string(), serde_json::json!(name));
                if let Some(team) = p.team_gid {
                    data.insert("team".to_string(), serde_json::json!(team));
                }
                if let Some(include) = p.include {
                    data.insert("include".to_string(), serde_json::json!(include));
                }

                let body = serde_json::json!({"data": data});
                let job: Resource = self
                    .client
                    .post(&format!("/projects/{}/duplicate", source_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to duplicate project", e))?;
                json_response(&job)
            }

            CreateResourceType::TaskDuplicate => {
                let source_gid = p
                    .source_gid
                    .ok_or_else(|| validation_error("source_gid is required for task_duplicate"))?;
                let name = p
                    .name
                    .ok_or_else(|| validation_error("name is required for task_duplicate"))?;

                let mut data = serde_json::Map::new();
                data.insert("name".to_string(), serde_json::json!(name));
                if let Some(include) = p.include {
                    data.insert("include".to_string(), serde_json::json!(include));
                }

                let body = serde_json::json!({"data": data});
                let task: Resource = self
                    .client
                    .post(&format!("/tasks/{}/duplicate", source_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to duplicate task", e))?;
                json_response(&task)
            }
        }
    }

    /// Update Asana resources.
    #[tool(
        description = "Update an existing Asana resource. Provide gid and only the fields to change.\n\
            \n\
            Resource types and their fields:\n\
            - task: name, assignee, due_on, start_on, completed, notes, html_notes, custom_fields\n\
            - project: name, notes, html_notes, color, archived, public, privacy_setting\n\
            - portfolio: name, color, public\n\
            - section: name (required)\n\
            - tag: name, color, notes\n\
            - comment: text (required)\n\
            - status_update: title, text, html_notes, status_type (on_track/at_risk/off_track)"
    )]
    async fn asana_update(
        &self,
        params: Parameters<UpdateParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;

        match p.resource_type {
            UpdateResourceType::Task => {
                let mut data = serde_json::Map::new();
                if let Some(name) = p.name {
                    data.insert("name".to_string(), serde_json::json!(name));
                }
                if let Some(assignee) = p.assignee {
                    data.insert("assignee".to_string(), serde_json::json!(assignee));
                }
                if let Some(due_on) = p.due_on {
                    data.insert("due_on".to_string(), serde_json::json!(due_on));
                }
                if let Some(start_on) = p.start_on {
                    data.insert("start_on".to_string(), serde_json::json!(start_on));
                }
                if let Some(notes) = p.notes {
                    data.insert("notes".to_string(), serde_json::json!(notes));
                }
                if let Some(html_notes) = p.html_notes {
                    data.insert("html_notes".to_string(), serde_json::json!(html_notes));
                }
                if let Some(completed) = p.completed {
                    data.insert("completed".to_string(), serde_json::json!(completed));
                }
                if let Some(cf) = p.custom_fields {
                    data.insert("custom_fields".to_string(), serde_json::json!(cf));
                }

                let body = serde_json::json!({"data": data});
                let task: Resource = self
                    .client
                    .put(&format!("/tasks/{}", p.gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to update task", e))?;
                json_response(&task)
            }

            UpdateResourceType::Project => {
                let mut data = serde_json::Map::new();
                if let Some(name) = p.name {
                    data.insert("name".to_string(), serde_json::json!(name));
                }
                if let Some(color) = p.color {
                    data.insert("color".to_string(), serde_json::json!(color));
                }
                if let Some(notes) = p.notes {
                    data.insert("notes".to_string(), serde_json::json!(notes));
                }
                if let Some(html_notes) = p.html_notes {
                    data.insert("html_notes".to_string(), serde_json::json!(html_notes));
                }
                if let Some(due_on) = p.due_on {
                    data.insert("due_on".to_string(), serde_json::json!(due_on));
                }
                if let Some(start_on) = p.start_on {
                    data.insert("start_on".to_string(), serde_json::json!(start_on));
                }
                if let Some(archived) = p.archived {
                    data.insert("archived".to_string(), serde_json::json!(archived));
                }
                if let Some(privacy) = p.privacy_setting {
                    data.insert("privacy_setting".to_string(), serde_json::json!(privacy));
                }
                if let Some(cf) = p.custom_fields {
                    data.insert("custom_fields".to_string(), serde_json::json!(cf));
                }

                let body = serde_json::json!({"data": data});
                let project: Resource = self
                    .client
                    .put(&format!("/projects/{}", p.gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to update project", e))?;
                json_response(&project)
            }

            UpdateResourceType::Portfolio => {
                let mut data = serde_json::Map::new();
                if let Some(name) = p.name {
                    data.insert("name".to_string(), serde_json::json!(name));
                }
                if let Some(color) = p.color {
                    data.insert("color".to_string(), serde_json::json!(color));
                }
                if let Some(public) = p.public {
                    data.insert("public".to_string(), serde_json::json!(public));
                }

                let body = serde_json::json!({"data": data});
                let portfolio: Resource = self
                    .client
                    .put(&format!("/portfolios/{}", p.gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to update portfolio", e))?;
                json_response(&portfolio)
            }

            UpdateResourceType::Section => {
                let name = p
                    .name
                    .as_ref()
                    .ok_or_else(|| validation_error("name is required for section update"))?;
                let body = serde_json::json!({"data": {"name": name}});
                let section: Resource = self
                    .client
                    .put(&format!("/sections/{}", p.gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to update section", e))?;
                json_response(&section)
            }

            UpdateResourceType::Tag => {
                let mut data = serde_json::Map::new();
                if let Some(name) = p.name {
                    data.insert("name".to_string(), serde_json::json!(name));
                }
                if let Some(color) = p.color {
                    data.insert("color".to_string(), serde_json::json!(color));
                }
                if let Some(notes) = p.notes {
                    data.insert("notes".to_string(), serde_json::json!(notes));
                }

                let body = serde_json::json!({"data": data});
                let tag: Resource = self
                    .client
                    .put(&format!("/tags/{}", p.gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to update tag", e))?;
                json_response(&tag)
            }

            UpdateResourceType::Comment => {
                let text = p
                    .text
                    .ok_or_else(|| validation_error("text is required for comment update"))?;
                let body = serde_json::json!({"data": {"text": text}});
                let story: Resource = self
                    .client
                    .put(&format!("/stories/{}", p.gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to update comment", e))?;
                json_response(&story)
            }

            UpdateResourceType::StatusUpdate => {
                let mut data = serde_json::Map::new();
                if let Some(title) = p.title {
                    data.insert("title".to_string(), serde_json::json!(title));
                }
                if let Some(text) = p.text {
                    data.insert("text".to_string(), serde_json::json!(text));
                }
                if let Some(html_text) = p.html_notes {
                    data.insert("html_text".to_string(), serde_json::json!(html_text));
                }
                if let Some(status_type) = p.status_type {
                    data.insert("status_type".to_string(), serde_json::json!(status_type));
                }

                if data.is_empty() {
                    return Err(validation_error(
                        "at least one of title, text, html_notes, or status_type is required",
                    ));
                }

                let body = serde_json::json!({"data": data});
                let status: Resource = self
                    .client
                    .put(&format!("/status_updates/{}", p.gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to update status update", e))?;
                json_response(&status)
            }
        }
    }

    /// Manage relationships between Asana resources.
    #[tool(description = "Add or remove relationships between Asana resources.\n\
            Use action='add' or action='remove', specify relationship type, target_gid, and item_gid(s).\n\
            \n\
            Relationships (target_gid -> item_gid):\n\
            - task_project: task -> project (add/remove task from project)\n\
            - task_tag: task -> tag\n\
            - task_parent: task -> parent_task (set parent to make subtask)\n\
            - task_dependency: task -> blocking_task(s)\n\
            - task_dependent: task -> dependent_task(s)\n\
            - task_follower: task -> user(s)\n\
            - portfolio_item: portfolio -> project\n\
            - portfolio_member: portfolio -> user(s)\n\
            - project_member: project -> user(s)\n\
            - project_follower: project -> user(s)\n\
            \n\
            Use item_gid for single item, item_gids for bulk operations.")]
    async fn asana_link(&self, params: Parameters<LinkParams>) -> Result<CallToolResult, McpError> {
        let p = params.0;

        match (p.action, p.relationship) {
            // Task-Project
            (LinkAction::Add, RelationshipType::TaskProject) => {
                let project_gid = p
                    .item_gid
                    .ok_or_else(|| validation_error("item_gid (project) is required"))?;
                let mut data = serde_json::Map::new();
                data.insert("project".to_string(), serde_json::json!(project_gid));
                if let Some(section) = p.section_gid {
                    data.insert("section".to_string(), serde_json::json!(section));
                }
                let body = serde_json::json!({"data": data});
                self.client
                    .post_empty(&format!("/tasks/{}/addProject", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to add task to project", e))?;
                success_response("Task added to project")
            }
            (LinkAction::Remove, RelationshipType::TaskProject) => {
                let project_gid = p
                    .item_gid
                    .ok_or_else(|| validation_error("item_gid (project) is required"))?;
                let body = serde_json::json!({"data": {"project": project_gid}});
                self.client
                    .post_empty(&format!("/tasks/{}/removeProject", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to remove task from project", e))?;
                success_response("Task removed from project")
            }

            // Task-Tag
            (LinkAction::Add, RelationshipType::TaskTag) => {
                let tag_gid = p
                    .item_gid
                    .ok_or_else(|| validation_error("item_gid (tag) is required"))?;
                let body = serde_json::json!({"data": {"tag": tag_gid}});
                self.client
                    .post_empty(&format!("/tasks/{}/addTag", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to add tag to task", e))?;
                success_response("Tag added to task")
            }
            (LinkAction::Remove, RelationshipType::TaskTag) => {
                let tag_gid = p
                    .item_gid
                    .ok_or_else(|| validation_error("item_gid (tag) is required"))?;
                let body = serde_json::json!({"data": {"tag": tag_gid}});
                self.client
                    .post_empty(&format!("/tasks/{}/removeTag", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to remove tag from task", e))?;
                success_response("Tag removed from task")
            }

            // Task-Parent
            (LinkAction::Add, RelationshipType::TaskParent) => {
                let parent_gid = p
                    .item_gid
                    .ok_or_else(|| validation_error("item_gid (parent task) is required"))?;
                let body = serde_json::json!({"data": {"parent": parent_gid}});
                let task: Resource = self
                    .client
                    .post(&format!("/tasks/{}/setParent", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to set task parent", e))?;
                json_response(&task)
            }
            (LinkAction::Remove, RelationshipType::TaskParent) => {
                let body = serde_json::json!({"data": {"parent": null}});
                let task: Resource = self
                    .client
                    .post(&format!("/tasks/{}/setParent", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to remove task parent", e))?;
                json_response(&task)
            }

            // Task-Dependency
            (LinkAction::Add, RelationshipType::TaskDependency) => {
                let gids = get_item_gids(&p)?;
                let body = serde_json::json!({"data": {"dependencies": gids}});
                self.client
                    .post_empty(&format!("/tasks/{}/addDependencies", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to add dependencies", e))?;
                success_response("Dependencies added")
            }
            (LinkAction::Remove, RelationshipType::TaskDependency) => {
                let gids = get_item_gids(&p)?;
                let body = serde_json::json!({"data": {"dependencies": gids}});
                self.client
                    .post_empty(
                        &format!("/tasks/{}/removeDependencies", p.target_gid),
                        &body,
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to remove dependencies", e))?;
                success_response("Dependencies removed")
            }

            // Task-Dependent
            (LinkAction::Add, RelationshipType::TaskDependent) => {
                let gids = get_item_gids(&p)?;
                let body = serde_json::json!({"data": {"dependents": gids}});
                self.client
                    .post_empty(&format!("/tasks/{}/addDependents", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to add dependents", e))?;
                success_response("Dependents added")
            }
            (LinkAction::Remove, RelationshipType::TaskDependent) => {
                let gids = get_item_gids(&p)?;
                let body = serde_json::json!({"data": {"dependents": gids}});
                self.client
                    .post_empty(&format!("/tasks/{}/removeDependents", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to remove dependents", e))?;
                success_response("Dependents removed")
            }

            // Task-Follower
            (LinkAction::Add, RelationshipType::TaskFollower) => {
                let gids = get_item_gids(&p)?;
                let body = serde_json::json!({"data": {"followers": gids}});
                self.client
                    .post_empty(&format!("/tasks/{}/addFollowers", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to add followers", e))?;
                success_response("Followers added")
            }
            (LinkAction::Remove, RelationshipType::TaskFollower) => {
                let gid = p
                    .item_gid
                    .ok_or_else(|| validation_error("item_gid (follower) is required"))?;
                let body = serde_json::json!({"data": {"followers": [gid]}});
                self.client
                    .post_empty(&format!("/tasks/{}/removeFollowers", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to remove follower", e))?;
                success_response("Follower removed")
            }

            // Portfolio-Item
            (LinkAction::Add, RelationshipType::PortfolioItem) => {
                let item_gid = p
                    .item_gid
                    .ok_or_else(|| validation_error("item_gid (project) is required"))?;
                let mut data = serde_json::Map::new();
                data.insert("item".to_string(), serde_json::json!(item_gid));
                if let Some(before) = p.insert_before {
                    data.insert("insert_before".to_string(), serde_json::json!(before));
                }
                if let Some(after) = p.insert_after {
                    data.insert("insert_after".to_string(), serde_json::json!(after));
                }
                let body = serde_json::json!({"data": data});
                self.client
                    .post_empty(&format!("/portfolios/{}/addItem", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to add item to portfolio", e))?;
                success_response("Item added to portfolio")
            }
            (LinkAction::Remove, RelationshipType::PortfolioItem) => {
                let item_gid = p
                    .item_gid
                    .ok_or_else(|| validation_error("item_gid (project) is required"))?;
                let body = serde_json::json!({"data": {"item": item_gid}});
                self.client
                    .post_empty(&format!("/portfolios/{}/removeItem", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to remove item from portfolio", e))?;
                success_response("Item removed from portfolio")
            }

            // Portfolio-Member
            (LinkAction::Add, RelationshipType::PortfolioMember) => {
                let gids = get_item_gids(&p)?;
                let body = serde_json::json!({"data": {"members": gids}});
                self.client
                    .post_empty(&format!("/portfolios/{}/addMembers", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to add portfolio members", e))?;
                success_response("Members added to portfolio")
            }
            (LinkAction::Remove, RelationshipType::PortfolioMember) => {
                let gids = get_item_gids(&p)?;
                let body = serde_json::json!({"data": {"members": gids}});
                self.client
                    .post_empty(
                        &format!("/portfolios/{}/removeMembers", p.target_gid),
                        &body,
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to remove portfolio members", e))?;
                success_response("Members removed from portfolio")
            }

            // Project-Member
            (LinkAction::Add, RelationshipType::ProjectMember) => {
                let gids = get_item_gids(&p)?;
                let body = serde_json::json!({"data": {"members": gids.join(",")}});
                self.client
                    .post_empty(&format!("/projects/{}/addMembers", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to add project members", e))?;
                success_response("Members added to project")
            }
            (LinkAction::Remove, RelationshipType::ProjectMember) => {
                let gids = get_item_gids(&p)?;
                let body = serde_json::json!({"data": {"members": gids.join(",")}});
                self.client
                    .post_empty(&format!("/projects/{}/removeMembers", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to remove project members", e))?;
                success_response("Members removed from project")
            }

            // Project-Follower
            (LinkAction::Add, RelationshipType::ProjectFollower) => {
                let gids = get_item_gids(&p)?;
                let body = serde_json::json!({"data": {"followers": gids.join(",")}});
                self.client
                    .post_empty(&format!("/projects/{}/addFollowers", p.target_gid), &body)
                    .await
                    .map_err(|e| error_to_mcp("Failed to add project followers", e))?;
                success_response("Followers added to project")
            }
            (LinkAction::Remove, RelationshipType::ProjectFollower) => {
                let gids = get_item_gids(&p)?;
                let body = serde_json::json!({"data": {"followers": gids.join(",")}});
                self.client
                    .post_empty(
                        &format!("/projects/{}/removeFollowers", p.target_gid),
                        &body,
                    )
                    .await
                    .map_err(|e| error_to_mcp("Failed to remove project followers", e))?;
                success_response("Followers removed from project")
            }
        }
    }

    /// Search for tasks in a workspace.
    #[tool(description = "Search for tasks in a workspace with filters.\n\
            \n\
            Required: workspace_gid\n\
            \n\
            Filters (all optional):\n\
            - text: Search in task name and notes\n\
            - assignee: User GID, 'me' for current user, or 'null' for unassigned\n\
            - projects: Filter by project GID(s)\n\
            - tags: Filter by tag GID(s)\n\
            - sections: Filter by section GID(s)\n\
            - completed: true/false\n\
            - due_on, due_on_before, due_on_after: Date filters (YYYY-MM-DD)\n\
            - start_on, start_on_before, start_on_after: Start date filters\n\
            - modified_at_after, modified_at_before: Datetime filters (ISO 8601)\n\
            - portfolios: Filter by portfolio GID(s)\n\
            - sort_by: due_date, created_at, completed_at, likes, modified_at\n\
            - sort_ascending: true/false")]
    async fn asana_search(
        &self,
        params: Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;

        // Build query parameters
        let mut query_params: Vec<(String, String)> =
            vec![("opt_fields".to_string(), SEARCH_FIELDS.to_string())];

        if let Some(text) = p.text {
            query_params.push(("text".to_string(), text));
        }
        if let Some(assignee) = p.assignee {
            if assignee == "null" {
                query_params.push(("assignee.any".to_string(), "null".to_string()));
            } else if assignee == "me" {
                query_params.push(("assignee.any".to_string(), "me".to_string()));
            } else {
                query_params.push(("assignee.any".to_string(), assignee));
            }
        }
        if let Some(projects) = p.projects {
            query_params.push(("projects.any".to_string(), projects.join(",")));
        }
        if let Some(tags) = p.tags {
            query_params.push(("tags.any".to_string(), tags.join(",")));
        }
        if let Some(sections) = p.sections {
            query_params.push(("sections.any".to_string(), sections.join(",")));
        }
        if let Some(completed) = p.completed {
            query_params.push(("completed".to_string(), completed.to_string()));
        }
        if let Some(due_on) = p.due_on {
            query_params.push(("due_on".to_string(), due_on));
        }
        if let Some(due_on_before) = p.due_on_before {
            query_params.push(("due_on.before".to_string(), due_on_before));
        }
        if let Some(due_on_after) = p.due_on_after {
            query_params.push(("due_on.after".to_string(), due_on_after));
        }
        if let Some(start_on) = p.start_on {
            query_params.push(("start_on".to_string(), start_on));
        }
        if let Some(start_on_before) = p.start_on_before {
            query_params.push(("start_on.before".to_string(), start_on_before));
        }
        if let Some(start_on_after) = p.start_on_after {
            query_params.push(("start_on.after".to_string(), start_on_after));
        }
        if let Some(modified_at_after) = p.modified_at_after {
            query_params.push(("modified_at.after".to_string(), modified_at_after));
        }
        if let Some(modified_at_before) = p.modified_at_before {
            query_params.push(("modified_at.before".to_string(), modified_at_before));
        }
        if let Some(portfolios) = p.portfolios {
            query_params.push(("portfolios.any".to_string(), portfolios.join(",")));
        }
        if let Some(sort_by) = p.sort_by {
            query_params.push(("sort_by".to_string(), sort_by));
        }
        if let Some(sort_ascending) = p.sort_ascending {
            query_params.push(("sort_ascending".to_string(), sort_ascending.to_string()));
        }

        // Convert to slice of tuples for the API call
        let query_refs: Vec<(&str, &str)> = query_params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let tasks: Vec<Resource> = self
            .client
            .get_all(
                &format!("/workspaces/{}/tasks/search", p.workspace_gid),
                &query_refs,
            )
            .await
            .map_err(|e| error_to_mcp("Failed to search tasks", e))?;

        json_response(&tasks)
    }
}

// ============================================================================
// Recursive Helper Methods
// ============================================================================

impl AsanaServer {
    /// Get a portfolio with its items recursively expanded.
    pub(crate) async fn get_portfolio_recursive(
        &self,
        gid: &str,
        max_depth: Option<usize>,
    ) -> Result<PortfolioWithItems, Error> {
        self.fetch_portfolio_with_depth(gid, max_depth, 0).await
    }

    fn fetch_portfolio_with_depth<'a>(
        &'a self,
        gid: &'a str,
        max_depth: Option<usize>,
        current_depth: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<PortfolioWithItems, Error>> + Send + 'a>,
    > {
        Box::pin(async move {
            let portfolio: Resource = self
                .client
                .get(
                    &format!("/portfolios/{}", gid),
                    &[("opt_fields", PORTFOLIO_FIELDS)],
                )
                .await?;

            let should_fetch_items = match max_depth {
                None => true,
                Some(max) => current_depth < max,
            };

            if !should_fetch_items {
                return Ok(PortfolioWithItems {
                    portfolio,
                    items: Vec::new(),
                });
            }

            let item_refs: Vec<PortfolioItem> = self
                .client
                .get_all(
                    &format!("/portfolios/{}/items", gid),
                    &[("opt_fields", PORTFOLIO_ITEMS_FIELDS)],
                )
                .await?;

            let mut items = Vec::new();

            for item_ref in item_refs {
                let expanded = match item_ref.resource_type.as_str() {
                    "project" => {
                        let project: Resource = self
                            .client
                            .get(
                                &format!("/projects/{}", item_ref.gid),
                                &[("opt_fields", PROJECT_FIELDS)],
                            )
                            .await?;
                        PortfolioItemExpanded::Project(Box::new(project))
                    }
                    "portfolio" => {
                        let nested = self
                            .fetch_portfolio_with_depth(&item_ref.gid, max_depth, current_depth + 1)
                            .await?;
                        PortfolioItemExpanded::Portfolio(Box::new(nested))
                    }
                    _ => continue,
                };
                items.push(expanded);
            }

            Ok(PortfolioWithItems { portfolio, items })
        })
    }

    /// Get a task with full context.
    pub(crate) async fn get_task_with_context(
        &self,
        gid: &str,
        include_subtasks: bool,
        include_dependencies: bool,
        include_comments: bool,
    ) -> Result<TaskWithContext, Error> {
        let task: Resource = self
            .client
            .get(
                &format!("/tasks/{}", gid),
                &[("opt_fields", TASK_FULL_FIELDS)],
            )
            .await?;

        let subtasks = if include_subtasks {
            self.client
                .get_all(
                    &format!("/tasks/{}/subtasks", gid),
                    &[("opt_fields", SUBTASK_FIELDS)],
                )
                .await?
        } else {
            Vec::new()
        };

        let (dependencies, dependents) = if include_dependencies {
            let deps: Vec<TaskDependency> = self
                .client
                .get_all(
                    &format!("/tasks/{}/dependencies", gid),
                    &[("opt_fields", "gid,name,resource_type")],
                )
                .await?;
            let depts: Vec<TaskDependency> = self
                .client
                .get_all(
                    &format!("/tasks/{}/dependents", gid),
                    &[("opt_fields", "gid,name,resource_type")],
                )
                .await?;
            (deps, depts)
        } else {
            (Vec::new(), Vec::new())
        };

        let comments = if include_comments {
            let stories: Vec<Story> = self
                .client
                .get_all(
                    &format!("/tasks/{}/stories", gid),
                    &[("opt_fields", STORY_FIELDS)],
                )
                .await?;
            stories.into_iter().filter(|s| s.is_comment()).collect()
        } else {
            Vec::new()
        };

        Ok(TaskWithContext {
            task,
            subtasks,
            dependencies,
            dependents,
            comments,
        })
    }

    /// Get all tasks recursively from a project or portfolio.
    pub(crate) async fn get_tasks_recursive(
        &self,
        gid: &str,
        subtask_depth: Option<i32>,
        portfolio_depth: Option<i32>,
    ) -> Result<Vec<Resource>, Error> {
        let portfolio_depth = portfolio_depth.unwrap_or(0);

        // Try to detect resource type by attempting to fetch as project first
        match self
            .client
            .get::<Resource>(&format!("/projects/{}", gid), &[("opt_fields", "gid")])
            .await
        {
            Ok(_) => self.get_tasks_from_project(gid, subtask_depth).await,
            Err(Error::NotFound(_)) => {
                self.get_tasks_from_portfolio(gid, subtask_depth, portfolio_depth)
                    .await
            }
            Err(e) => Err(e),
        }
    }

    async fn get_tasks_from_project(
        &self,
        project_gid: &str,
        subtask_depth: Option<i32>,
    ) -> Result<Vec<Resource>, Error> {
        let tasks: Vec<Resource> = self
            .client
            .get_all(
                &format!("/projects/{}/tasks", project_gid),
                &[("opt_fields", RECURSIVE_TASK_FIELDS)],
            )
            .await?;
        self.expand_subtasks_flat(tasks, subtask_depth, 0).await
    }

    async fn get_tasks_from_portfolio(
        &self,
        portfolio_gid: &str,
        subtask_depth: Option<i32>,
        portfolio_depth: i32,
    ) -> Result<Vec<Resource>, Error> {
        let depth = if portfolio_depth < 0 {
            None
        } else {
            Some(portfolio_depth as usize)
        };
        let portfolio = self.get_portfolio_recursive(portfolio_gid, depth).await?;
        let project_gids = Self::collect_project_gids_from_portfolio(&portfolio);

        let mut all_tasks = Vec::new();
        for project_gid in project_gids {
            match self
                .get_tasks_from_project(&project_gid, subtask_depth)
                .await
            {
                Ok(tasks) => all_tasks.extend(tasks),
                Err(Error::NotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(all_tasks)
    }

    fn collect_project_gids_from_portfolio(portfolio: &PortfolioWithItems) -> Vec<String> {
        let mut gids = Vec::new();
        for item in &portfolio.items {
            match item {
                PortfolioItemExpanded::Project(p) => gids.push(p.gid.clone()),
                PortfolioItemExpanded::Portfolio(nested) => {
                    gids.extend(Self::collect_project_gids_from_portfolio(nested));
                }
            }
        }
        gids
    }

    fn expand_subtasks_flat<'a>(
        &'a self,
        tasks: Vec<Resource>,
        subtask_depth: Option<i32>,
        current_depth: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<Resource>, Error>> + Send + 'a>,
    > {
        Box::pin(async move {
            let max_depth = match subtask_depth {
                Some(d) if d < 0 => None,
                Some(d) => Some(d as usize),
                None => None,
            };

            let should_fetch_subtasks = match max_depth {
                None => true,
                Some(max) => current_depth < max,
            };

            let mut all_tasks = Vec::new();

            for task in tasks {
                let num_subtasks = task
                    .fields
                    .get("num_subtasks")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                all_tasks.push(task.clone());

                if should_fetch_subtasks && num_subtasks > 0 {
                    let subtasks: Vec<Resource> = self
                        .client
                        .get_all(
                            &format!("/tasks/{}/subtasks", task.gid),
                            &[("opt_fields", RECURSIVE_TASK_FIELDS)],
                        )
                        .await?;
                    let expanded = self
                        .expand_subtasks_flat(subtasks, subtask_depth, current_depth + 1)
                        .await?;
                    all_tasks.extend(expanded);
                }
            }

            Ok(all_tasks)
        })
    }
}

// ============================================================================
// Server Handler
// ============================================================================

#[tool_handler]
impl ServerHandler for AsanaServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "asanamcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Asana MCP server providing tools for interacting with Asana tasks, \
                 projects, and portfolios. Authenticate with ASANA_TOKEN environment variable."
                    .to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests;
