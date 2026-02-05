# asanamcp

[![CI](https://github.com/adlio/asanamcp/actions/workflows/ci.yml/badge.svg)](https://github.com/adlio/asanamcp/actions/workflows/ci.yml)
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
        "ASANA_TOKEN": "your-personal-access-token"
      }
    }
  }
}
```

3. Install:

```bash
cargo install --git https://github.com/adlio/asanamcp
```

## Tools

| Tool | Description |
|------|-------------|
| `asana_workspaces` | List all workspaces |
| `asana_get` | Fetch any resource (projects, tasks, portfolios, etc.) |
| `asana_create` | Create resources (tasks, comments, projects, etc.) |
| `asana_update` | Update existing resources |
| `asana_link` | Manage relationships (task↔project, dependencies, etc.) |

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
| `workspace_favorites` | workspace GID | `include_projects`, `include_portfolios` |
| `project_tasks` | project/portfolio GID | `subtask_depth` |
| `task_subtasks` | task GID | |
| `task_comments` | task GID | |
| `project_status_updates` | project/portfolio GID | |
| `workspace` | workspace GID | |
| `workspace_templates` | workspace GID | |
| `project_template` | template GID | |
| `project_sections` | project GID | |
| `section` | section GID | |
| `workspace_tags` | workspace GID | |
| `tag` | tag GID | |

Depth: `-1` = unlimited, `0` = none, `N` = N levels.

### asana_create

```json
{"resource_type": "task", "project_gid": "123", "name": "New task", "assignee": "me"}
```

| resource_type | Required fields |
|---------------|-----------------|
| `task` | `workspace_gid` or `project_gid`, `name` |
| `subtask` | `task_gid`, `name` |
| `project` | `workspace_gid` or `team_gid`, `name` |
| `project_from_template` | `template_gid`, `name` |
| `portfolio` | `workspace_gid`, `name` |
| `section` | `project_gid`, `name` |
| `comment` | `task_gid`, `text` |
| `status_update` | `parent_gid`, `status_type`, `text` |
| `tag` | `workspace_gid`, `name` |

### asana_update

```json
{"resource_type": "task", "gid": "123", "completed": true}
```

Supports: `task`, `project`, `portfolio`, `section`, `tag`, `comment`.

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
```

## License

MIT
