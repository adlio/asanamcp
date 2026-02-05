//! Parameter types for MCP tool inputs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parameters for listing workspaces (no parameters needed).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkspacesParams {}

/// The type of resource to fetch.
///
/// Note: The `gid` parameter meaning varies by resource type:
/// - `project`, `portfolio`, `task`, `workspace`, `project_template`, `section`, `tag`:
///   GID of that specific resource
/// - `workspace_favorites`, `workspace_projects`, `workspace_templates`, `workspace_tags`:
///   GID of the workspace
/// - `my_tasks`: GID of the workspace to get user's assigned tasks from
/// - `project_tasks`: GID of the project or portfolio to get tasks from
/// - `task_subtasks`, `task_comments`: GID of the parent task
/// - `project_status_updates`: GID of the project or portfolio
/// - `project_sections`: GID of the project
/// - `all_workspaces`: GID is ignored
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// Get a single project by GID
    Project,
    /// Get a portfolio with nested items (use depth parameter)
    Portfolio,
    /// Get a task with context (use include_* flags)
    Task,
    /// Get user's favorites from a workspace (gid = workspace GID or empty for default)
    #[serde(rename = "workspace_favorites", alias = "favorites")]
    WorkspaceFavorites,
    /// Get all tasks from a project or portfolio (gid = project/portfolio GID)
    #[serde(rename = "project_tasks", alias = "tasks")]
    ProjectTasks,
    /// Get subtasks of a task (gid = parent task GID)
    #[serde(rename = "task_subtasks", alias = "subtasks")]
    TaskSubtasks,
    /// Get comments on a task (gid = task GID)
    #[serde(rename = "task_comments", alias = "comments")]
    TaskComments,
    /// Get status update history (gid = project/portfolio GID)
    #[serde(rename = "project_status_updates", alias = "status_updates")]
    ProjectStatusUpdates,
    /// List all workspaces (gid is ignored)
    #[serde(rename = "all_workspaces", alias = "workspaces")]
    AllWorkspaces,
    /// Get a single workspace by GID
    Workspace,
    /// List templates in a workspace (gid = workspace GID)
    #[serde(rename = "workspace_templates", alias = "project_templates")]
    WorkspaceTemplates,
    /// Get a single project template by GID
    ProjectTemplate,
    /// List sections in a project (gid = project GID)
    #[serde(rename = "project_sections", alias = "sections")]
    ProjectSections,
    /// Get a single section by GID
    Section,
    /// List tags in a workspace (gid = workspace GID)
    #[serde(rename = "workspace_tags", alias = "tags")]
    WorkspaceTags,
    /// Get a single tag by GID
    Tag,
    /// Get tasks assigned to the current user in a workspace (gid = workspace GID)
    #[serde(rename = "my_tasks", alias = "my_assigned_tasks")]
    MyTasks,
    /// List all projects in a workspace (gid = workspace GID)
    #[serde(rename = "workspace_projects", alias = "projects")]
    WorkspaceProjects,
    /// Get the current authenticated user (gid is ignored)
    #[serde(alias = "current_user")]
    Me,
    /// Get a user by GID
    User,
    /// List all users in a workspace (gid = workspace GID)
    #[serde(rename = "workspace_users", alias = "users")]
    WorkspaceUsers,
    /// Get a team by GID
    Team,
    /// List all teams in an organization/workspace (gid = workspace GID)
    #[serde(rename = "workspace_teams", alias = "teams")]
    WorkspaceTeams,
    /// List users in a team (gid = team GID)
    TeamUsers,
    /// Get custom field settings for a project (gid = project GID)
    #[serde(rename = "project_custom_fields", alias = "custom_fields")]
    ProjectCustomFields,
}

/// Parameters for the universal get tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetParams {
    /// The type of resource to fetch
    pub resource_type: ResourceType,
    /// The GID of the resource. Optional for: all_workspaces, me, and workspace-based operations
    /// (which fall back to ASANA_DEFAULT_WORKSPACE). Required for resource-specific operations.
    #[serde(default)]
    pub gid: Option<String>,
    /// Portfolio/task traversal depth: -1 = unlimited, 0 = none, N = N levels
    #[serde(default)]
    pub depth: Option<i32>,
    /// Subtask expansion depth: -1 = unlimited, 0 = none (default), N = N levels
    #[serde(default)]
    pub subtask_depth: Option<i32>,
    /// Include subtasks when fetching a task (default: true)
    #[serde(default)]
    pub include_subtasks: Option<bool>,
    /// Include dependencies/dependents when fetching a task (default: true)
    #[serde(default)]
    pub include_dependencies: Option<bool>,
    /// Include comments when fetching a task (default: true)
    #[serde(default)]
    pub include_comments: Option<bool>,
    /// Override default fields returned. If not provided, returns curated fields per resource type.
    /// Example: ["gid", "name", "completed", "assignee.name"]
    #[serde(default)]
    pub opt_fields: Option<Vec<String>>,
}

/// The type of resource to create.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CreateResourceType {
    /// Create a new task
    Task,
    /// Create a subtask under a parent task
    Subtask,
    /// Create a new project
    Project,
    /// Create a project from a template
    #[serde(rename = "project_from_template")]
    ProjectFromTemplate,
    /// Create a new portfolio
    Portfolio,
    /// Create a section in a project
    Section,
    /// Create a comment on a task
    Comment,
    /// Create a status update on a project/portfolio
    #[serde(rename = "status_update")]
    StatusUpdate,
    /// Create a new tag
    Tag,
    /// Duplicate an existing project
    #[serde(rename = "project_duplicate")]
    ProjectDuplicate,
    /// Duplicate an existing task
    #[serde(rename = "task_duplicate")]
    TaskDuplicate,
}

/// Date variable for template instantiation.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct DateVariableParam {
    /// The GID of the date variable from the template
    pub gid: String,
    /// The date value in YYYY-MM-DD format
    pub value: String,
}

/// Role assignment for template instantiation.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct RoleAssignmentParam {
    /// The GID of the role from the template
    pub gid: String,
    /// The user GID to assign to this role
    pub value: String,
}

/// Parameters for the create tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateParams {
    /// The type of resource to create
    pub resource_type: CreateResourceType,
    /// Workspace GID (required for task without project, portfolio, tag)
    #[serde(default)]
    pub workspace_gid: Option<String>,
    /// Project GID (for task creation, section creation)
    #[serde(default)]
    pub project_gid: Option<String>,
    /// Task GID (for subtask or comment creation)
    #[serde(default)]
    pub task_gid: Option<String>,
    /// Team GID (for project creation)
    #[serde(default)]
    pub team_gid: Option<String>,
    /// Parent GID (for status update - project or portfolio)
    #[serde(default)]
    pub parent_gid: Option<String>,
    /// Template GID (for project_from_template)
    #[serde(default)]
    pub template_gid: Option<String>,
    /// Date variables for template instantiation
    #[serde(default)]
    pub requested_dates: Option<Vec<DateVariableParam>>,
    /// Role assignments for template instantiation
    #[serde(default)]
    pub requested_roles: Option<Vec<RoleAssignmentParam>>,
    /// Name of the resource
    #[serde(default)]
    pub name: Option<String>,
    /// Plain text notes/description
    #[serde(default)]
    pub notes: Option<String>,
    /// HTML notes/description
    #[serde(default)]
    pub html_notes: Option<String>,
    /// Color (for project, portfolio, tag)
    #[serde(default)]
    pub color: Option<String>,
    /// Due date in YYYY-MM-DD format
    #[serde(default)]
    pub due_on: Option<String>,
    /// Start date in YYYY-MM-DD format
    #[serde(default)]
    pub start_on: Option<String>,
    /// Assignee user GID (for task)
    #[serde(default)]
    pub assignee: Option<String>,
    /// Privacy setting (for project): "public_to_workspace" or "private_to_team"
    #[serde(default)]
    pub privacy_setting: Option<String>,
    /// Whether the resource is public
    #[serde(default)]
    pub public: Option<bool>,
    /// Status type for status_update: "on_track", "at_risk", "off_track", etc.
    #[serde(default)]
    pub status_type: Option<String>,
    /// Title (for status_update)
    #[serde(default)]
    pub title: Option<String>,
    /// Text content (for comment, status_update)
    #[serde(default)]
    pub text: Option<String>,
    /// Custom field values as {field_gid: value}
    #[serde(default)]
    pub custom_fields: Option<HashMap<String, serde_json::Value>>,
    /// Source GID (for project_duplicate, task_duplicate - the resource to copy)
    #[serde(default)]
    pub source_gid: Option<String>,
    /// What to include when duplicating. For project: members, notes, task_notes, task_assignee,
    /// task_subtasks, task_attachments, task_dates, task_dependencies, task_followers, task_tags.
    /// For task: notes, assignee, subtasks, attachments, tags, followers, projects, dates, dependencies, parent.
    #[serde(default)]
    pub include: Option<Vec<String>>,
    /// Override default fields returned in response. If not provided, returns minimal confirmation.
    /// Example: ["gid", "name", "permalink_url"]
    #[serde(default)]
    pub opt_fields: Option<Vec<String>>,
}

/// Parameters for task search.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchParams {
    /// Workspace GID to search in (uses ASANA_DEFAULT_WORKSPACE if not provided)
    #[serde(default)]
    pub workspace_gid: Option<String>,
    /// Search for tasks containing this text in name or notes
    #[serde(default)]
    pub text: Option<String>,
    /// Filter by assignee user GID (use "me" for current user, "null" for unassigned)
    #[serde(default)]
    pub assignee: Option<String>,
    /// Filter by project GID(s)
    #[serde(default)]
    pub projects: Option<Vec<String>>,
    /// Filter by tag GID(s)
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Filter by section GID(s)
    #[serde(default)]
    pub sections: Option<Vec<String>>,
    /// Filter by completion status
    #[serde(default)]
    pub completed: Option<bool>,
    /// Filter by tasks due on this date (YYYY-MM-DD)
    #[serde(default)]
    pub due_on: Option<String>,
    /// Filter by tasks due on or before this date
    #[serde(default)]
    pub due_on_before: Option<String>,
    /// Filter by tasks due on or after this date
    #[serde(default)]
    pub due_on_after: Option<String>,
    /// Filter by tasks starting on this date
    #[serde(default)]
    pub start_on: Option<String>,
    /// Filter by tasks starting on or before this date
    #[serde(default)]
    pub start_on_before: Option<String>,
    /// Filter by tasks starting on or after this date
    #[serde(default)]
    pub start_on_after: Option<String>,
    /// Filter by tasks modified on or after this datetime (ISO 8601)
    #[serde(default)]
    pub modified_at_after: Option<String>,
    /// Filter by tasks modified on or before this datetime (ISO 8601)
    #[serde(default)]
    pub modified_at_before: Option<String>,
    /// Filter by tasks in portfolios (GID)
    #[serde(default)]
    pub portfolios: Option<Vec<String>>,
    /// Sort by: due_date, created_at, completed_at, likes, modified_at
    #[serde(default)]
    pub sort_by: Option<String>,
    /// Sort order: asc or desc
    #[serde(default)]
    pub sort_ascending: Option<bool>,
    /// Override default fields returned. If not provided, returns curated search result fields.
    /// Example: ["gid", "name", "completed", "assignee.name", "due_on"]
    #[serde(default)]
    pub opt_fields: Option<Vec<String>>,
}

/// The type of resource to update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UpdateResourceType {
    /// Update a task
    Task,
    /// Update a project
    Project,
    /// Update a portfolio
    Portfolio,
    /// Update a section
    Section,
    /// Update a tag
    Tag,
    /// Update a comment/story
    Comment,
    /// Update a status update
    #[serde(rename = "status_update")]
    StatusUpdate,
}

/// Parameters for the update tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateParams {
    /// The type of resource to update
    pub resource_type: UpdateResourceType,
    /// The GID of the resource to update
    pub gid: String,
    /// New name
    #[serde(default)]
    pub name: Option<String>,
    /// New plain text notes/description
    #[serde(default)]
    pub notes: Option<String>,
    /// New HTML notes/description
    #[serde(default)]
    pub html_notes: Option<String>,
    /// Mark task as completed/incomplete
    #[serde(default)]
    pub completed: Option<bool>,
    /// New due date in YYYY-MM-DD format
    #[serde(default)]
    pub due_on: Option<String>,
    /// New start date in YYYY-MM-DD format
    #[serde(default)]
    pub start_on: Option<String>,
    /// New assignee user GID
    #[serde(default)]
    pub assignee: Option<String>,
    /// New color
    #[serde(default)]
    pub color: Option<String>,
    /// Archive/unarchive project
    #[serde(default)]
    pub archived: Option<bool>,
    /// New privacy setting
    #[serde(default)]
    pub privacy_setting: Option<String>,
    /// Make public/private
    #[serde(default)]
    pub public: Option<bool>,
    /// New text content (for comment, status_update)
    #[serde(default)]
    pub text: Option<String>,
    /// New title (for status_update)
    #[serde(default)]
    pub title: Option<String>,
    /// New status type (for status_update): "on_track", "at_risk", "off_track", etc.
    #[serde(default)]
    pub status_type: Option<String>,
    /// Updated custom field values
    #[serde(default)]
    pub custom_fields: Option<HashMap<String, serde_json::Value>>,
    /// Override default fields returned in response. If not provided, returns curated fields.
    /// Example: ["gid", "name", "modified_at"]
    #[serde(default)]
    pub opt_fields: Option<Vec<String>>,
}

/// The action to perform on a relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LinkAction {
    /// Add a relationship
    Add,
    /// Remove a relationship
    Remove,
}

/// The type of relationship to manage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// Task <-> Project membership
    #[serde(rename = "task_project")]
    TaskProject,
    /// Task <-> Tag association
    #[serde(rename = "task_tag")]
    TaskTag,
    /// Task parent-child relationship
    #[serde(rename = "task_parent")]
    TaskParent,
    /// Task dependency (blocking) relationship
    #[serde(rename = "task_dependency")]
    TaskDependency,
    /// Task dependent (blocked by) relationship
    #[serde(rename = "task_dependent")]
    TaskDependent,
    /// Task follower
    #[serde(rename = "task_follower")]
    TaskFollower,
    /// Portfolio <-> Project/Portfolio item
    #[serde(rename = "portfolio_item")]
    PortfolioItem,
    /// Portfolio member
    #[serde(rename = "portfolio_member")]
    PortfolioMember,
    /// Project member
    #[serde(rename = "project_member")]
    ProjectMember,
    /// Project follower
    #[serde(rename = "project_follower")]
    ProjectFollower,
}

/// Parameters for the link tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LinkParams {
    /// Whether to add or remove the relationship
    pub action: LinkAction,
    /// The type of relationship to manage
    pub relationship: RelationshipType,
    /// The GID of the target resource (task, project, or portfolio)
    pub target_gid: String,
    /// Single item GID for the relationship
    #[serde(default)]
    pub item_gid: Option<String>,
    /// Multiple item GIDs for bulk operations
    #[serde(default)]
    pub item_gids: Option<Vec<String>>,
    /// Section GID for task-project relationships
    #[serde(default)]
    pub section_gid: Option<String>,
    /// Insert before this GID (for ordering)
    #[serde(default)]
    pub insert_before: Option<String>,
    /// Insert after this GID (for ordering)
    #[serde(default)]
    pub insert_after: Option<String>,
}
