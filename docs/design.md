# Asana MCP Tools Design

This document outlines the complete design for the Asana MCP server tools - covering reads, writes, and relationship management.

## Current State

The SDK (`asanaclient` crate) is **read-only**. The MCP server has 2 tools:
- `asana_workspaces` - list workspaces
- `asana_get` - unified read tool with `resource_type` discriminator

## Target Use Cases

These are the primary use cases driving this design:

| # | Use Case | Tool | Call |
|---|----------|------|------|
| 1 | Create Task in Project | `asana_create` | `resource_type=Task, project_gid=X, name="..."` |
| 2 | Update Task notes | `asana_update` | `resource_type=Task, gid=X, notes="..."` |
| 3 | Change Project color | `asana_update` | `resource_type=Project, gid=X, color="light-green"` |
| 4 | Change Portfolio color | `asana_update` | `resource_type=Portfolio, gid=X, color="light-blue"` |
| 5 | Create Project from Template | `asana_create` | `resource_type=Project, template_gid=X, name="...", requested_dates=[...]` |
| 6 | Create Portfolio | `asana_create` | `resource_type=Portfolio, workspace_gid=X, name="..."` |
| 7 | Add Portfolio to Portfolio | `asana_link` | `action=Add, relationship=PortfolioItem, target_gid=parent, item_gid=child` |
| 8 | Move Task to new Project | `asana_link` | `action=Add` then `action=Remove` with `relationship=TaskProject` |
| 9 | Copy Task (Phase 3) | - | Requires `duplicateTask` - not in initial scope |
| 10 | Add User to Project | `asana_link` | `action=Add, relationship=ProjectMember, target_gid=project, item_gids=[user]` |
| 11 | Add User to Portfolio | `asana_link` | `action=Add, relationship=PortfolioMember, target_gid=portfolio, item_gids=[user]` |
| 12 | Read Project Template | `asana_get` | `resource_type=ProjectTemplate, gid=X` |
| 13 | List Project Templates | `asana_get` | `resource_type=ProjectTemplates, gid=workspace_gid` |

## Target Design: 4 Tools Total

The goal is to consolidate all Asana operations into 4 unified tools that cover the full spectrum of read/write/relationship operations.

| Tool | Purpose |
|------|---------|
| `asana_get` | Read any resource (absorbs `asana_workspaces`) |
| `asana_create` | Create any resource (including from template) |
| `asana_update` | Update any resource |
| `asana_link` | Add/remove relationships between resources |

### Design Principles

1. **Resource-type discriminator pattern** - One tool handles many operations via `resource_type` parameter
2. **Clear error messages** - Agents can iterate on failures, so we favor consolidation over strict parameter validation
3. **Gradual implementation** - Start with core use cases, extend as needed

---

## Tool Specifications

### 1. `asana_get` (Extended)

Absorbs `asana_workspaces` and adds support for templates.

**Parameters:**
```rust
struct GetParams {
    resource_type: ResourceType,  // Extended enum
    gid: Option<String>,          // Optional for list operations
    // ... existing depth, include_* params
}

enum ResourceType {
    // Existing
    Project, Portfolio, Task, Favorites, Tasks, Subtasks, Comments, StatusUpdates,
    // New
    Workspaces,           // list all (no gid needed)
    Workspace,            // get one by gid
    ProjectTemplates,     // list all for workspace (gid = workspace_gid)
    ProjectTemplate,      // get one by gid (includes requested_dates, requested_roles)
    Sections,             // list for project (gid = project_gid)
    Section,              // get one by gid
    Tags,                 // list for workspace
    Tag,                  // get one by gid
}
```

**Mappings to Asana API:**

| resource_type | gid meaning | Asana operation |
|---------------|-------------|-----------------|
| `Workspaces` | not used | `getWorkspaces` |
| `Workspace` | workspace_gid | `getWorkspace` |
| `ProjectTemplates` | workspace_gid | `getProjectTemplates` |
| `ProjectTemplate` | template_gid | `getProjectTemplate` |
| `Project` | project_gid | `getProject` |
| `Portfolio` | portfolio_gid | `getPortfolio` |
| `Task` | task_gid | `getTask` |
| `Tasks` | project_gid | `getTasksForProject` |
| `Subtasks` | task_gid | `getSubtasksForTask` |
| `Sections` | project_gid | `getSectionsForProject` |
| `Section` | section_gid | `getSection` |
| `Comments` | task_gid | `getStoriesForTask` (filtered) |
| `StatusUpdates` | project/portfolio_gid | `getStatusesForObject` |
| `Favorites` | user_gid or "me" | `getFavoritesForUser` |
| `Tags` | workspace_gid | `getTagsForWorkspace` |
| `Tag` | tag_gid | `getTag` |

---

### 2. `asana_create`

Creates new resources. Handles template instantiation as a special case.

**Parameters:**
```rust
struct CreateParams {
    resource_type: CreateResourceType,

    // Context (which one depends on resource_type)
    workspace_gid: Option<String>,
    project_gid: Option<String>,
    portfolio_gid: Option<String>,
    task_gid: Option<String>,       // parent for subtasks, or for comments

    // Template instantiation
    template_gid: Option<String>,
    requested_dates: Option<Vec<DateVariable>>,
    requested_roles: Option<Vec<RoleAssignment>>,

    // Common fields
    name: Option<String>,
    notes: Option<String>,
    html_notes: Option<String>,

    // Resource-specific fields
    color: Option<String>,          // portfolios, projects
    due_on: Option<String>,         // tasks, projects
    start_on: Option<String>,       // tasks, projects
    assignee: Option<String>,       // tasks
    privacy_setting: Option<String>, // projects, portfolios
}

enum CreateResourceType {
    Task,
    Subtask,
    Project,
    Portfolio,
    Section,
    Comment,      // creates a story/comment on a task
    StatusUpdate, // creates status on project/portfolio
    Tag,
}

struct DateVariable {
    gid: String,
    value: String,  // YYYY-MM-DD
}

struct RoleAssignment {
    gid: String,
    value: String,  // user_gid
}
```

**Mappings to Asana API:**

| resource_type | Required context | Asana operation |
|---------------|------------------|-----------------|
| `Task` | project_gid | `createTask` with projects field |
| `Subtask` | task_gid | `createSubtaskForTask` |
| `Project` | workspace_gid | `createProject` |
| `Project` | workspace_gid + template_gid | `instantiateProject` |
| `Portfolio` | workspace_gid | `createPortfolio` |
| `Section` | project_gid | `createSectionForProject` |
| `Comment` | task_gid | `createStoryForTask` |
| `StatusUpdate` | project_gid or portfolio_gid | `createStatusForObject` |
| `Tag` | workspace_gid | `createTagForWorkspace` |

**Template Instantiation Flow:**

1. User calls `asana_create(resource_type=Project, template_gid=X, name="My Project")`
2. If template has date variables and `requested_dates` is missing:
   - Return error: "Template X requires date variables. Call asana_get(resource_type=ProjectTemplate, gid=X) to see required dates."
3. User fetches template, sees `requested_dates` array
4. User retries with `requested_dates` populated
5. Returns Job response (async) - user may need to poll or just proceed

**Error Examples:**
```
"resource_type=Task requires project_gid"
"resource_type=Project requires workspace_gid"
"resource_type=Subtask requires task_gid (parent task)"
"Template 'xyz' requires requested_dates for: Start Date, Due Date"
```

---

### 3. `asana_update`

Updates existing resources.

**Parameters:**
```rust
struct UpdateParams {
    resource_type: UpdateResourceType,
    gid: String,

    // Common updatable fields
    name: Option<String>,
    notes: Option<String>,
    html_notes: Option<String>,

    // Task-specific
    completed: Option<bool>,
    due_on: Option<String>,
    start_on: Option<String>,
    assignee: Option<String>,

    // Project/Portfolio-specific
    color: Option<String>,
    archived: Option<bool>,
    privacy_setting: Option<String>,
    owner: Option<String>,

    // Custom fields (generic map)
    custom_fields: Option<HashMap<String, String>>,
}

enum UpdateResourceType {
    Task,
    Project,
    Portfolio,
    Section,
    Tag,
    Comment,  // update story text
}
```

**Mappings to Asana API:**

| resource_type | Asana operation |
|---------------|-----------------|
| `Task` | `updateTask` |
| `Project` | `updateProject` |
| `Portfolio` | `updatePortfolio` |
| `Section` | `updateSection` |
| `Tag` | `updateTag` |
| `Comment` | `updateStory` |

**What CAN be updated (per Asana API):**

- **Task**: name, notes, html_notes, completed, due_on, start_on, assignee, custom_fields
- **Project**: name, notes, html_notes, color, archived, privacy_setting, due_on, start_on, owner, custom_fields
- **Portfolio**: name, color, archived, privacy_setting, due_on, start_on, custom_fields
- **Section**: name
- **Tag**: name, color, notes
- **Comment/Story**: text, html_text

**What CANNOT be updated (read-only or separate endpoints):**
- Task's `projects` field (use `asana_link`)
- Task's `parent` field (use `asana_link` with `setParent` action)
- Project/Portfolio `members` (use `asana_link`)
- Project/Portfolio `items` (use `asana_link`)

---

### 4. `asana_link`

Manages relationships between resources. Handles add/remove operations.

**Parameters:**
```rust
struct LinkParams {
    action: LinkAction,
    relationship: RelationshipType,

    // The container/target
    target_gid: String,

    // The item(s) being linked
    item_gid: Option<String>,        // single item
    item_gids: Option<Vec<String>>,  // multiple items (for members)

    // Positioning (for tasks in projects, items in portfolios)
    section_gid: Option<String>,
    insert_before: Option<String>,
    insert_after: Option<String>,
}

enum LinkAction {
    Add,
    Remove,
}

enum RelationshipType {
    // Task relationships
    TaskProject,      // task <-> project
    TaskTag,          // task <-> tag
    TaskParent,       // task -> parent (subtask relationship)
    TaskDependency,   // task -> depends on task
    TaskDependent,    // task -> blocked by task
    TaskFollower,     // task <-> user follower

    // Portfolio relationships
    PortfolioItem,    // portfolio <-> project or nested portfolio
    PortfolioMember,  // portfolio <-> user member

    // Project relationships
    ProjectMember,    // project <-> user member
    ProjectFollower,  // project <-> user follower
}
```

**Mappings to Asana API:**

| action | relationship | Asana operation |
|--------|--------------|-----------------|
| Add | TaskProject | `addProjectForTask` |
| Remove | TaskProject | `removeProjectForTask` |
| Add | TaskTag | `addTagForTask` |
| Remove | TaskTag | `removeTagForTask` |
| Add | TaskParent | `setParentForTask` |
| Remove | TaskParent | `setParentForTask` with parent=null |
| Add | TaskDependency | `addDependenciesForTask` |
| Remove | TaskDependency | `removeDependenciesForTask` |
| Add | TaskDependent | `addDependentsForTask` |
| Remove | TaskDependent | `removeDependentsForTask` |
| Add | TaskFollower | `addFollowersForTask` |
| Remove | TaskFollower | `removeFollowerForTask` |
| Add | PortfolioItem | `addItemForPortfolio` |
| Remove | PortfolioItem | `removeItemForPortfolio` |
| Add | PortfolioMember | `addMembersForPortfolio` |
| Remove | PortfolioMember | `removeMembersForPortfolio` |
| Add | ProjectMember | `addMembersForProject` |
| Remove | ProjectMember | `removeMembersForProject` |
| Add | ProjectFollower | `addFollowersForProject` |
| Remove | ProjectFollower | `removeFollowersForProject` |

**Positioning Metadata:**

For `TaskProject` with `action=Add`:
- `section_gid` - which section to add the task to
- `insert_before` / `insert_after` - position relative to another task

For `PortfolioItem` with `action=Add`:
- `insert_before` / `insert_after` - position relative to another item

**Common Operations:**

```
# Move task from project A to project B
asana_link(action=Add, relationship=TaskProject, target_gid=B, item_gid=task)
asana_link(action=Remove, relationship=TaskProject, target_gid=A, item_gid=task)

# Add task to a specific section
asana_link(action=Add, relationship=TaskProject, target_gid=project, item_gid=task, section_gid=section)

# Make task a subtask of another task
asana_link(action=Add, relationship=TaskParent, target_gid=parent_task, item_gid=child_task)

# Add project to portfolio
asana_link(action=Add, relationship=PortfolioItem, target_gid=portfolio, item_gid=project)

# Add multiple members to project
asana_link(action=Add, relationship=ProjectMember, target_gid=project, item_gids=[user1, user2])
```

---

## Implementation Plan

### Phase 1: Core Use Cases (Initial Implementation)

These cover the original 5 use cases plus template instantiation:

**asana_get extensions:**
- `Workspaces` (absorb `asana_workspaces`)
- `ProjectTemplates`
- `ProjectTemplate`

**asana_create:**
- `Task` (in a project)
- `Project` (simple create)
- `Project` (from template via `instantiateProject`)
- `Portfolio`

**asana_update:**
- `Task` (notes, html_notes, name, completed, dates)
- `Project` (color, name, archived)
- `Portfolio` (color, name, archived)

**asana_link:**
- `TaskProject` (add/remove) - for moving tasks
- `PortfolioItem` (add) - for adding projects/portfolios to portfolios
- `ProjectMember` (add) - for assigning users

### Phase 2: Extended Operations

- `Subtask` creation
- `Section` create/update
- `Comment` create/update
- `StatusUpdate` create
- `Tag` operations
- `TaskDependency` / `TaskDependent`
- `TaskFollower` / `ProjectFollower`
- `PortfolioMember`

### Phase 3: Advanced (If Needed)

- `duplicateTask` / `duplicateProject` - would need new tool or action
- `projectSaveAsTemplate` - would need new tool or action
- Goals, Time Tracking, Webhooks, etc.

---

## SDK Changes Required

The `asanaclient` crate is currently read-only. We need to add:

### 1. HTTP Methods

Add to `Client`:
```rust
pub async fn post<T, B>(&self, path: &str, body: &B) -> Result<T>
where
    T: DeserializeOwned,
    B: Serialize;

pub async fn put<T, B>(&self, path: &str, body: &B) -> Result<T>
where
    T: DeserializeOwned,
    B: Serialize;

pub async fn delete(&self, path: &str) -> Result<()>;
```

### 2. Request Types

New types for request bodies:
```rust
// tasks
pub struct CreateTaskRequest { ... }
pub struct UpdateTaskRequest { ... }
pub struct AddProjectRequest { project: String, section: Option<String>, ... }

// projects
pub struct CreateProjectRequest { ... }
pub struct UpdateProjectRequest { ... }
pub struct InstantiateProjectRequest { name: String, requested_dates: Vec<...>, ... }

// portfolios
pub struct CreatePortfolioRequest { ... }
pub struct UpdatePortfolioRequest { ... }
pub struct AddItemRequest { item: String, insert_before: Option<String>, ... }

// members
pub struct AddMembersRequest { members: String }  // comma-separated
```

### 3. API Module Functions

Add to `api/tasks.rs`:
```rust
pub async fn create(client: &Client, request: CreateTaskRequest) -> Result<Task>;
pub async fn update(client: &Client, gid: &str, request: UpdateTaskRequest) -> Result<Task>;
pub async fn add_project(client: &Client, task_gid: &str, request: AddProjectRequest) -> Result<()>;
pub async fn remove_project(client: &Client, task_gid: &str, project_gid: &str) -> Result<()>;
// ... etc
```

Similar patterns for `api/projects.rs`, `api/portfolios.rs`, etc.

---

## What This Design CANNOT Support

These operations don't fit the 4-tool model and would need additional tools:

### Requires New Tools

| Operation | Why it doesn't fit |
|-----------|-------------------|
| `duplicateTask` | Not a create (copies existing), has unique params |
| `duplicateProject` | Not a create (copies existing), has unique params |
| `projectSaveAsTemplate` | Converts project to template, unique operation |
| `searchTasksForWorkspace` | Search is different from get, complex query params |
| `triggerRule` | Unique operation |
| `createBatchRequest` | Batch operations are a different pattern |

### Out of Scope (Enterprise/Admin)

| Operation | Reason |
|-----------|--------|
| `getAuditLogEvents` | Admin/enterprise feature |
| `createOrganizationExport` | Admin operation |
| `approveAccessRequest` / `rejectAccessRequest` | Admin operation |
| Team management (`createTeam`, `addUserForTeam`) | Usually admin |
| Workspace management (`addUserForWorkspace`) | Usually admin |
| `createAllocation`, `createBudget`, `createRate` | Advanced/enterprise |

### Covered by asana_update but with limitations

| Operation | Limitation |
|-----------|------------|
| `updateGoal`, `updateGoalMetric` | Goals not in initial scope |
| `updateMembership` | Access level changes may not work via simple update |
| `updateWebhook` | Webhooks not in initial scope |

---

## Error Handling Strategy

All errors should be actionable. Examples:

```
// Missing required context
"asana_create: resource_type=Task requires project_gid. Provide the project GID where the task should be created."

// Template needs dates
"asana_create: Template 'Launch Checklist' (gid: 12345) requires date variables.
Required dates: Start Date (gid: 1), Launch Date (gid: 2).
Call asana_get(resource_type=ProjectTemplate, gid=12345) for full details, then retry with requested_dates."

// Invalid relationship
"asana_link: relationship=TaskProject requires item_gid (the task) and target_gid (the project)."

// Invalid action for relationship
"asana_link: relationship=TaskParent does not support action=Remove. To unset parent, use action=Add with target_gid=null."

// Positioning only valid for add
"asana_link: section_gid is only valid for action=Add"
```

---

## Asana API Coverage Summary

| Category | Total Ops | Supported | Not Supported | Notes |
|----------|-----------|-----------|---------------|-------|
| Read (GET) | 97 | ~25 | ~72 | Core resources covered |
| Create | 30 | ~10 | ~20 | Core resources covered |
| Update | 19 | ~6 | ~13 | Core resources covered |
| Delete | 18 | 0 | 18 | Intentionally excluded (risky) |
| Relationship (add/remove) | 41 | ~16 | ~25 | Core relationships covered |
| Special | 13 | 1 | 12 | Only instantiateProject |
| **Total** | **218** | **~58** | **~160** | ~27% coverage |

The ~58 supported operations cover the most common use cases. The ~160 unsupported operations are mostly:
- Delete operations (intentionally excluded)
- Admin/enterprise features
- Advanced features (goals, time tracking, webhooks, budgets)
- Duplicate/special operations

---

## Complete API Operation Mapping

This section maps every Asana API operation to our tool design.

### Read Operations (97 total)

#### Supported via `asana_get`

| Asana Operation | resource_type | gid meaning |
|-----------------|---------------|-------------|
| `getWorkspaces` | `Workspaces` | not used |
| `getWorkspace` | `Workspace` | workspace_gid |
| `getProject` | `Project` | project_gid |
| `getProjects` | `Projects` | workspace_gid |
| `getProjectsForWorkspace` | `Projects` | workspace_gid |
| `getProjectsForTeam` | `Projects` | team_gid |
| `getPortfolio` | `Portfolio` | portfolio_gid |
| `getPortfolios` | `Portfolios` | workspace_gid |
| `getItemsForPortfolio` | `PortfolioItems` | portfolio_gid |
| `getTask` | `Task` | task_gid |
| `getTasks` | `Tasks` | project_gid |
| `getTasksForProject` | `Tasks` | project_gid |
| `getTasksForSection` | `Tasks` | section_gid |
| `getSubtasksForTask` | `Subtasks` | task_gid |
| `getSection` | `Section` | section_gid |
| `getSectionsForProject` | `Sections` | project_gid |
| `getProjectTemplate` | `ProjectTemplate` | template_gid |
| `getProjectTemplates` | `ProjectTemplates` | workspace_gid |
| `getStoriesForTask` | `Comments` | task_gid (filtered to comments) |
| `getStatusesForObject` | `StatusUpdates` | project/portfolio_gid |
| `getFavoritesForUser` | `Favorites` | user_gid or "me" |
| `getTag` | `Tag` | tag_gid |
| `getTags` | `Tags` | workspace_gid |
| `getTagsForWorkspace` | `Tags` | workspace_gid |
| `getTagsForTask` | `TaskTags` | task_gid |

#### Not Supported (72 operations)

| Asana Operation | Reason |
|-----------------|--------|
| `getUser`, `getUsers`, `getUsersForTeam`, `getUsersForWorkspace` | Users are referenced but not fetched directly |
| `getTeam`, `getTeamsForWorkspace`, `getTeamsForUser` | Teams not in scope |
| `getTeamMembership`, `getTeamMemberships`, `getTeamMembershipsForTeam`, `getTeamMembershipsForUser` | Team memberships not in scope |
| `getProjectMembership`, `getProjectMembershipsForProject` | Can extend later |
| `getPortfolioMembership`, `getPortfolioMemberships`, `getPortfolioMembershipsForPortfolio` | Can extend later |
| `getWorkspaceMembership`, `getWorkspaceMembershipsForUser`, `getWorkspaceMembershipsForWorkspace` | Workspace memberships not in scope |
| `getGoal`, `getGoals`, `getGoalRelationship`, `getGoalRelationships`, `getParentGoalsForGoal` | Goals not in scope |
| `getCustomField`, `getCustomFieldsForWorkspace`, `getCustomFieldSettingsForProject/Portfolio/Goal/Team` | Custom fields read via parent resource |
| `getStory` | Individual stories not needed |
| `getAttachment`, `getAttachmentsForObject` | Attachments not in scope |
| `getWebhook`, `getWebhooks` | Webhooks not in scope |
| `getJob` | Jobs are transient |
| `getEvents`, `getWorkspaceEvents` | Events/webhooks not in scope |
| `getTimePeriod`, `getTimePeriods` | Time periods not in scope |
| `getTimeTrackingEntry`, `getTimeTrackingEntries`, `getTimeTrackingEntriesForTask` | Time tracking not in scope |
| `getAllocation`, `getAllocations` | Resource management not in scope |
| `getBudget`, `getBudgets` | Budgets not in scope |
| `getRate`, `getRates` | Rates not in scope |
| `getTaskTemplate`, `getTaskTemplates` | Task templates not in scope (only project templates) |
| `getProjectBrief` | Project briefs not in scope |
| `getProjectStatus`, `getProjectStatusesForProject` | Use StatusUpdates instead |
| `getProjectsForTask` | Task already includes projects |
| `getDependenciesForTask`, `getDependentsForTask` | Can extend later, available in Task response |
| `getUserTaskList`, `getUserTaskListForUser` | User task lists not in scope |
| `getTasksForTag` | Can extend later |
| `getTasksForUserTaskList` | User task lists not in scope |
| `getTaskForCustomID` | Custom IDs not in scope |
| `getTaskCountsForProject` | Counts not in scope |
| `getAuditLogEvents` | Admin feature |
| `getOrganizationExport` | Admin feature |
| `getAccessRequests` | Admin feature |
| `getCustomType`, `getCustomTypes` | Custom types not in scope |
| `getReactionsOnObject` | Reactions not in scope |

### Create Operations (30 total)

#### Supported via `asana_create`

| Asana Operation | resource_type | Required context |
|-----------------|---------------|------------------|
| `createTask` | `Task` | project_gid |
| `createSubtaskForTask` | `Subtask` | task_gid |
| `createProject` | `Project` | workspace_gid |
| `createProjectForWorkspace` | `Project` | workspace_gid |
| `createProjectForTeam` | `Project` | team_gid |
| `instantiateProject` | `Project` | workspace_gid + template_gid |
| `createPortfolio` | `Portfolio` | workspace_gid |
| `createSectionForProject` | `Section` | project_gid |
| `createStoryForTask` | `Comment` | task_gid |
| `createStatusForObject` | `StatusUpdate` | project_gid or portfolio_gid |
| `createTag` | `Tag` | workspace_gid |
| `createTagForWorkspace` | `Tag` | workspace_gid |

#### Not Supported (18 operations)

| Asana Operation | Reason |
|-----------------|--------|
| `createGoal`, `createGoalMetric` | Goals not in scope |
| `createTeam` | Teams not in scope |
| `createCustomField`, `createEnumOptionForCustomField` | Custom field creation not in scope |
| `createWebhook` | Webhooks not in scope |
| `createAttachmentForObject` | Attachments not in scope |
| `createTimeTrackingEntry` | Time tracking not in scope |
| `createProjectBrief` | Project briefs not in scope |
| `createProjectStatusForProject` | Use StatusUpdate instead |
| `createMembership` | Use asana_link instead |
| `createAllocation` | Resource management not in scope |
| `createBudget` | Budgets not in scope |
| `createRate` | Rates not in scope |
| `createAccessRequest` | Admin feature |
| `createOrganizationExport`, `createGraphExport`, `createResourceExport` | Admin features |
| `createBatchRequest` | Batch operations not in scope |

### Update Operations (19 total)

#### Supported via `asana_update`

| Asana Operation | resource_type |
|-----------------|---------------|
| `updateTask` | `Task` |
| `updateProject` | `Project` |
| `updatePortfolio` | `Portfolio` |
| `updateSection` | `Section` |
| `updateTag` | `Tag` |
| `updateStory` | `Comment` |

#### Not Supported (13 operations)

| Asana Operation | Reason |
|-----------------|--------|
| `updateGoal`, `updateGoalMetric`, `updateGoalRelationship` | Goals not in scope |
| `updateTeam` | Teams not in scope |
| `updateCustomField`, `updateEnumOption` | Custom field updates not in scope |
| `updateWebhook` | Webhooks not in scope |
| `updateTimeTrackingEntry` | Time tracking not in scope |
| `updateProjectBrief` | Project briefs not in scope |
| `updateMembership` | Memberships managed via asana_link |
| `updateAllocation` | Resource management not in scope |
| `updateBudget` | Budgets not in scope |
| `updateRate` | Rates not in scope |
| `updateUser`, `updateUserForWorkspace` | User updates not in scope |
| `updateWorkspace` | Workspace updates not in scope |

### Delete Operations (18 total)

#### Supported: NONE

All delete operations are intentionally excluded as too risky for automated agents.

| Asana Operation | Reason |
|-----------------|--------|
| `deleteTask` | Risky - data loss |
| `deleteProject` | Risky - data loss |
| `deletePortfolio` | Risky - data loss |
| `deleteSection` | Risky - data loss |
| `deleteTag` | Risky - data loss |
| `deleteStory` | Risky - data loss |
| `deleteStatus` | Risky - data loss |
| `deleteProjectStatus` | Risky - data loss |
| `deleteProjectBrief` | Risky - data loss |
| `deleteProjectTemplate` | Risky - data loss |
| `deleteTaskTemplate` | Risky - data loss |
| `deleteCustomField` | Risky - data loss |
| `deleteMembership` | Use asana_link remove instead |
| `deleteAttachment` | Risky - data loss |
| `deleteTimeTrackingEntry` | Risky - data loss |
| `deleteWebhook` | Webhooks not in scope |
| `deleteGoal` | Goals not in scope |
| `deleteAllocation`, `deleteBudget`, `deleteRate` | Resource management not in scope |

### Relationship Operations (41 total)

#### Supported via `asana_link`

| Asana Operation | relationship | action |
|-----------------|--------------|--------|
| `addProjectForTask` | `TaskProject` | `Add` |
| `removeProjectForTask` | `TaskProject` | `Remove` |
| `addTagForTask` | `TaskTag` | `Add` |
| `removeTagForTask` | `TaskTag` | `Remove` |
| `setParentForTask` | `TaskParent` | `Add` (null to remove) |
| `addDependenciesForTask` | `TaskDependency` | `Add` |
| `removeDependenciesForTask` | `TaskDependency` | `Remove` |
| `addDependentsForTask` | `TaskDependent` | `Add` |
| `removeDependentsForTask` | `TaskDependent` | `Remove` |
| `addFollowersForTask` | `TaskFollower` | `Add` |
| `removeFollowerForTask` | `TaskFollower` | `Remove` |
| `addItemForPortfolio` | `PortfolioItem` | `Add` |
| `removeItemForPortfolio` | `PortfolioItem` | `Remove` |
| `addMembersForPortfolio` | `PortfolioMember` | `Add` |
| `removeMembersForPortfolio` | `PortfolioMember` | `Remove` |
| `addMembersForProject` | `ProjectMember` | `Add` |
| `removeMembersForProject` | `ProjectMember` | `Remove` |
| `addFollowersForProject` | `ProjectFollower` | `Add` |
| `removeFollowersForProject` | `ProjectFollower` | `Remove` |
| `addTaskForSection` | `TaskSection` | `Add` |

#### Not Supported (21 operations)

| Asana Operation | Reason |
|-----------------|--------|
| `addCustomFieldSettingForProject/Portfolio/Goal` | Custom field settings not in scope |
| `removeCustomFieldSettingForProject/Portfolio/Goal` | Custom field settings not in scope |
| `addSupportingRelationship`, `removeSupportingRelationship` | Goals not in scope |
| `addFollowers`, `removeFollowers` (for goals) | Goals not in scope |
| `addUserForTeam`, `removeUserForTeam` | Teams not in scope |
| `addUserForWorkspace`, `removeUserForWorkspace` | Workspace management not in scope |
| `setMetric`, `setMetricCurrentValue` | Goals not in scope |

### Special Operations (13 total)

#### Supported

| Asana Operation | How supported |
|-----------------|---------------|
| `instantiateProject` | `asana_create` with template_gid |

#### Not Supported (12 operations)

| Asana Operation | Reason |
|-----------------|--------|
| `duplicateTask` | Would need new tool - unique params |
| `duplicateProject` | Would need new tool - unique params |
| `projectSaveAsTemplate` | Would need new tool - unique operation |
| `instantiateTask` | Task templates not in scope |
| `insertSectionForProject` | Section reordering not in scope |
| `insertEnumOptionForCustomField` | Custom fields not in scope |
| `searchTasksForWorkspace` | Search is a different pattern |
| `typeaheadForWorkspace` | Typeahead not in scope |
| `approveAccessRequest`, `rejectAccessRequest` | Admin features |
| `triggerRule` | Rules not in scope |

---

## File Structure

```
asanaclient/
├── src/
│   ├── client.rs        # Add post(), put(), delete() methods
│   ├── api/
│   │   ├── tasks.rs     # Add create, update, add_project, etc.
│   │   ├── projects.rs  # Add create, update, instantiate, etc.
│   │   ├── portfolios.rs # Add create, update, add_item, etc.
│   │   └── ...
│   └── types/
│       ├── requests.rs  # New file for request body types
│       └── ...

asanaclient-mcp/
├── src/
│   ├── server.rs        # Update asana_get, add asana_create, asana_update, asana_link
│   └── main.rs
```

---

## Migration Notes

### Deprecating `asana_workspaces`

1. Keep `asana_workspaces` working for backward compatibility
2. Add `Workspaces` to `asana_get` resource types
3. Document that `asana_workspaces` is deprecated
4. Remove in future version

### MCP Tool Registration

The server will register 4 tools:
- `asana_get` (updated)
- `asana_create` (new)
- `asana_update` (new)
- `asana_link` (new)

Plus `asana_workspaces` for backward compatibility (deprecated).
