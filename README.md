# asanamcp

[![Crates.io](https://img.shields.io/crates/v/asanamcp.svg)](https://crates.io/crates/asanamcp)
[![Documentation](https://docs.rs/asanamcp/badge.svg)](https://docs.rs/asanamcp)
[![CI](https://github.com/adlio/asanamcp/actions/workflows/ci.yml/badge.svg)](https://github.com/adlio/asanamcp/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/adlio/asanamcp/branch/main/graph/badge.svg)](https://codecov.io/gh/adlio/asanamcp)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

MCP server for the Asana API.

## Quick Start

1. Get an Asana Personal Access Token at https://app.asana.com/0/my-apps

2. Add to Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "asana": {
      "command": "asanamcp",
      "env": {
        "ASANA_TOKEN": "your-personal-access-token",
        "ASANA_DEFAULT_WORKSPACE": "your-workspace-gid"
      }
    }
  }
}
```

The `ASANA_DEFAULT_WORKSPACE` is optional but recommended if you work primarily in one workspace. When set, workspace-based operations (search, list projects, list users, etc.) will use this default, reducing the need to specify workspace GID in every request.

3. Install:

```bash
# From crates.io (when published)
cargo install asanamcp

# From GitHub
cargo install --git https://github.com/adlio/asanamcp

# From local clone
git clone https://github.com/adlio/asanamcp
cd asanamcp
make install
```

## After Installation

The binary is installed to `~/.cargo/bin/asanamcp`. Ensure `~/.cargo/bin` is in your PATH:

```bash
# Check installation
which asanamcp
# Should output: /Users/<you>/.cargo/bin/asanamcp

# Verify it runs
asanamcp --help
```

### Testing the Server

Before configuring Claude Desktop, verify the server works:

```bash
# Dump tool schemas (useful for debugging)
asanamcp --schema

# Dump a specific tool's schema
asanamcp --schema get

# Launch the MCP Inspector (interactive web UI)
make inspect
```

The MCP Inspector (`make inspect`) opens a browser-based UI where you can see all available tools, their schemas, and test API calls interactively. This is useful for troubleshooting.

## Tools

| Tool | Description |
|------|-------------|
| `asana_workspaces` | List all workspaces |
| `asana_get` | Fetch any resource (projects, tasks, portfolios, etc.) |
| `asana_create` | Create resources (tasks, comments, projects, etc.) |
| `asana_update` | Update existing resources |
| `asana_link` | Manage relationships (task↔project, dependencies, etc.) |
| `asana_task_search` | Search for tasks with rich filters (assignee, due date, etc.) |
| `asana_resource_search` | Search for resources by name (projects, templates, users, teams, etc.) |

### asana_get

Fetch any Asana resource with recursive traversal support.

```json
{"resource_type": "portfolio", "gid": "123", "depth": -1}
```

| resource_type | gid | Options |
|---------------|-----|---------|
| `project` | project GID | |
| `portfolio` | portfolio GID | `depth`: traversal depth |
| `task` | task GID | `include_subtasks`, `include_dependencies`, `include_comments` |
| `my_tasks` | workspace GID* | Tasks assigned to current user |
| `workspace_favorites` | workspace GID* | `depth` for portfolio traversal |
| `workspace_projects` | workspace GID* | All projects in workspace |
| `workspace_templates` | team GID (optional) | Empty = all accessible templates |
| `workspace_tags` | workspace GID* | |
| `workspace_users` | workspace GID* | |
| `workspace_teams` | workspace GID* | |
| `project_tasks` | project/portfolio GID | `subtask_depth` |
| `task_subtasks` | task GID | |
| `task_comments` | task GID | |
| `project_status_updates` | project/portfolio GID | |
| `workspace` | workspace GID | |
| `project_template` | template GID | |
| `project_sections` | project GID | |
| `section` | section GID | |
| `tag` | tag GID | |
| `me` | (ignored) | Current authenticated user |
| `user` | user GID | |
| `team` | team GID | |
| `team_users` | team GID | |
| `project_custom_fields` | project GID | |
| `project_brief` | brief GID | Project brief (Key Resources on Overview tab, NOT the Note tab) |
| `project_project_brief` | project GID | Get project's brief via project GID |

*Uses `ASANA_DEFAULT_WORKSPACE` if gid is empty.

Depth: `-1` = unlimited, `0` = none, `N` = N levels.

### asana_create

```json
{"resource_type": "task", "project_gid": "123", "name": "New task", "assignee": "me"}
```

| resource_type | Required fields |
|---------------|-----------------|
| `task` | `project_gid` or `workspace_gid`*, `name` |
| `subtask` | `task_gid`, `name` |
| `project` | `workspace_gid` or `team_gid`, `name` |
| `project_from_template` | `template_gid`, `name` |
| `portfolio` | `workspace_gid`*, `name` |
| `section` | `project_gid`, `name` |
| `comment` | `task_gid`, `text` |
| `status_update` | `parent_gid`, `status_type`, `text` |
| `tag` | `workspace_gid`*, `name` |
| `project_duplicate` | `source_gid`, `name` |
| `task_duplicate` | `source_gid`, `name` |
| `project_brief` | `project_gid`, `html_text` (with `<body>` tags) | Key Resources on Overview tab (NOT the Note tab) |

*Uses `ASANA_DEFAULT_WORKSPACE` if not provided.

### asana_update

```json
{"resource_type": "task", "gid": "123", "completed": true}
```

Supports: `task`, `project`, `portfolio`, `section`, `tag`, `comment`, `status_update`, `project_brief` (Key Resources on Overview tab, NOT the Note tab).

### asana_link

```json
{"action": "add", "relationship": "task_project", "target_gid": "task123", "item_gid": "proj456"}
```

| relationship | target | item |
|--------------|--------|------|
| `task_project` | task GID | project GID |
| `task_tag` | task GID | tag GID |
| `task_parent` | task GID | parent task GID |
| `task_dependency` | task GID | blocking task GID(s) |
| `task_dependent` | task GID | dependent task GID(s) |
| `task_follower` | task GID | user GID(s) |
| `portfolio_item` | portfolio GID | project GID |
| `portfolio_member` | portfolio GID | user GID(s) |
| `project_member` | project GID | user GID(s) |
| `project_follower` | project GID | user GID(s) |

Use `item_gid` for single items or `item_gids` for bulk operations.

### asana_task_search

Search for tasks with rich filtering options.

```json
{"workspace_gid": "123", "text": "bug", "completed": false, "assignee": "me"}
```

| Filter | Description |
|--------|-------------|
| `workspace_gid` | Workspace to search (uses default if not provided) |
| `text` | Search in task name and notes |
| `assignee` | User GID, `me`, or `null` for unassigned |
| `projects` | Filter by project GID(s) |
| `tags` | Filter by tag GID(s) |
| `sections` | Filter by section GID(s) |
| `completed` | `true` or `false` |
| `due_on`, `due_on_before`, `due_on_after` | Date filters (YYYY-MM-DD) |
| `sort_by` | `due_date`, `created_at`, `completed_at`, `likes`, `modified_at` |
| `sort_ascending` | `true` or `false` |

### asana_resource_search

Search for any Asana resource by name using typeahead. Use this to find projects, templates, users, teams, and more.

```json
{"query": "CloudSmith", "resource_type": "project_template"}
```

| Parameter | Description |
|-----------|-------------|
| `query` | Search text (required) |
| `resource_type` | `project`, `project_template`, `portfolio`, `user`, `team`, `tag`, or `goal` |
| `workspace_gid` | Workspace to search (uses default if not provided) |
| `count` | Max results (default 20, max 100) |

## Library Usage

```rust
use asanamcp::{AsanaClient, Resource};

#[tokio::main]
async fn main() -> Result<(), asanamcp::Error> {
    let client = AsanaClient::from_env()?;

    let workspaces: Vec<Resource> = client
        .get_all("/workspaces", &[("opt_fields", "gid,name")])
        .await?;

    for ws in workspaces {
        println!("{}: {:?}", ws.gid, ws.fields.get("name"));
    }
    Ok(())
}
```

## Development

```bash
make ci         # Run all checks (fmt, lint, build, docs, test)
make test       # Run tests
make coverage   # Coverage report
make fmt        # Format code
make lint       # Run clippy
make inspect    # Open MCP Inspector web UI
make schema     # Dump tool schemas to stdout
```

## License

MIT
