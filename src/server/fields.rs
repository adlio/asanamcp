//! Opt fields constants for Asana API requests.
//!
//! These constants define which fields to request from the Asana API
//! for each resource type. Including specific fields reduces response
//! size and improves performance.

/// Fields to request for project resources.
pub const PROJECT_FIELDS: &str = "gid,name,color,archived,public,owner,owner.name,\
    team,team.name,workspace,workspace.name,current_status_update,\
    current_status_update.gid,current_status_update.status_type,\
    current_status_update.title,current_status_update.text,\
    notes,created_at,modified_at,due_date,due_on,start_on,permalink_url,icon";

/// Fields to request for portfolio resources.
pub const PORTFOLIO_FIELDS: &str = "gid,name,color,owner,owner.name,workspace,\
    current_status_update,current_status_update.gid,current_status_update.status_type,\
    current_status_update.title,current_status_update.text,\
    created_at,created_by,permalink_url,public";

/// Fields to request for portfolio items (minimal for type dispatch).
pub const PORTFOLIO_ITEMS_FIELDS: &str = "gid,resource_type,name";

/// Full fields to request for a single task.
pub const TASK_FULL_FIELDS: &str = "gid,name,resource_type,completed,completed_at,\
    completed_by,completed_by.name,assignee,assignee.name,assignee.email,\
    due_on,due_at,start_on,start_at,notes,html_notes,created_at,created_by,\
    created_by.name,modified_at,permalink_url,parent,parent.name,num_likes,\
    num_subtasks,liked,projects,projects.name,workspace,workspace.name,\
    tags,tags.name,memberships,memberships.project,memberships.project.name,\
    memberships.section,memberships.section.name,assignee_section,assignee_section.name";

/// Fields to request for recursive task fetching.
pub const RECURSIVE_TASK_FIELDS: &str = "gid,name,resource_type,completed,completed_at,\
    assignee,assignee.name,due_on,due_at,start_on,notes,created_at,modified_at,\
    permalink_url,parent,parent.name,num_likes,num_subtasks,liked,\
    projects,projects.name,workspace,tags,memberships,memberships.project,\
    memberships.project.name,memberships.section,memberships.section.name";

/// Fields to request for subtasks.
pub const SUBTASK_FIELDS: &str = "gid,name,completed,assignee,assignee.name,due_on,num_subtasks";

/// Fields to request for stories/comments.
pub const STORY_FIELDS: &str = "gid,created_at,created_by,created_by.name,\
    resource_subtype,text,html_text,is_pinned,is_edited,num_likes,liked";

/// Fields to request for status updates.
pub const STATUS_UPDATE_FIELDS: &str = "gid,title,text,html_text,status_type,\
    created_at,created_by,created_by.name,modified_at,parent,parent.name";

/// Fields to request for workspaces.
pub const WORKSPACE_FIELDS: &str = "gid,name,is_organization";

/// Fields to request for project templates.
pub const TEMPLATE_FIELDS: &str = "gid,name,description,html_description,owner,owner.name,\
    team,team.name,public,requested_dates,requested_dates.gid,requested_dates.name,\
    requested_dates.description,requested_roles,requested_roles.gid,requested_roles.name,color";

/// Fields to request for sections.
pub const SECTION_FIELDS: &str = "gid,name,project,project.name,created_at";

/// Fields to request for tags.
pub const TAG_FIELDS: &str =
    "gid,name,color,notes,workspace,workspace.name,created_at,permalink_url";

/// Fields to request for users.
pub const USER_FIELDS: &str = "gid,name,email,photo,workspaces,workspaces.name";

/// Fields to request for teams.
pub const TEAM_FIELDS: &str = "gid,name,description,html_description,organization,permalink_url";

/// Fields to request for custom field settings.
pub const CUSTOM_FIELD_SETTINGS_FIELDS: &str = "gid,custom_field,custom_field.gid,\
    custom_field.name,custom_field.type,custom_field.enum_options,\
    custom_field.enum_options.gid,custom_field.enum_options.name,\
    custom_field.enum_options.color,custom_field.precision,\
    custom_field.currency_code,is_important,project";

/// Fields to request for search results.
pub const SEARCH_FIELDS: &str = "gid,name,completed,assignee,assignee.name,\
    due_on,start_on,projects,projects.name,tags,tags.name,permalink_url";

/// Fields to request for project briefs (the "Key Resources" section on Overview tab, NOT the Note tab).
pub const PROJECT_BRIEF_FIELDS: &str =
    "gid,title,text,html_text,permalink_url,project,project.name";
