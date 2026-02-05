# asanamcp

MCP (Model Context Protocol) server for the Asana API.

## Features

- **4 unified tools** covering read, create, update, and relationship operations
- **Recursive traversal** of portfolios, projects, and tasks with configurable depth
- **Hybrid typed responses** - minimal typed fields for dispatch with raw JSON for AI consumption
- **Full CRUD support** for tasks, projects, portfolios, sections, tags, and more

## Installation

### From source

```bash
git clone https://github.com/adlio/asanamcp
cd asanamcp
make install
```

### Using cargo

```bash
cargo install --git https://github.com/adlio/asanamcp
```

## Usage

### Environment Setup

Set your Asana Personal Access Token:

```bash
export ASANA_TOKEN="your-personal-access-token"
```

Get a token at https://app.asana.com/0/my-apps

### Running the Server

```bash
asanamcp
```

The server communicates via STDIO using the MCP protocol.

### Claude Desktop Configuration

Add to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

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

## MCP Tools

### asana_workspaces

List all workspaces accessible to the authenticated user.

### asana_get

Universal read tool for fetching any Asana resource:

| resource_type | gid meaning | Description |
|---------------|-------------|-------------|
| `project` | project GID | Get a project |
| `portfolio` | portfolio GID | Get portfolio with nested items (use `depth`) |
| `task` | task GID | Get task with context (use `include_*` flags) |
| `favorites` | workspace GID | Get user's favorites |
| `tasks` | project/portfolio GID | Get all tasks (use `subtask_depth`) |
| `subtasks` | task GID | Get subtasks |
| `comments` | task GID | Get comments |
| `status_updates` | project/portfolio GID | Get status history |
| `workspaces` | (ignored) | List all workspaces |
| `workspace` | workspace GID | Get a workspace |
| `project_templates` | workspace GID | List templates |
| `project_template` | template GID | Get a template |
| `sections` | project GID | List sections |
| `section` | section GID | Get a section |
| `tags` | workspace GID | List tags |
| `tag` | tag GID | Get a tag |

**Depth parameters:** `-1` = unlimited, `0` = none, `N` = N levels

### asana_create

Create new resources:

- `task` - Create a task (requires `workspace_gid` or `project_gid`)
- `subtask` - Create a subtask (requires `task_gid`)
- `project` - Create a project (requires `workspace_gid` or `team_gid`)
- `project_from_template` - Instantiate from template (requires `template_gid`)
- `portfolio` - Create a portfolio (requires `workspace_gid`)
- `section` - Create a section (requires `project_gid`)
- `comment` - Add a comment (requires `task_gid`)
- `status_update` - Create status update (requires `parent_gid`)
- `tag` - Create a tag (requires `workspace_gid`)

### asana_update

Update existing resources:

- `task` - Update task fields
- `project` - Update project fields
- `portfolio` - Update portfolio fields
- `section` - Update section name
- `tag` - Update tag fields
- `comment` - Update comment text

### asana_link

Manage relationships between resources:

| relationship | Description |
|--------------|-------------|
| `task_project` | Add/remove task from project |
| `task_tag` | Add/remove tag from task |
| `task_parent` | Set/remove task parent |
| `task_dependency` | Add/remove task dependencies |
| `task_dependent` | Add/remove task dependents |
| `task_follower` | Add/remove task followers |
| `portfolio_item` | Add/remove project from portfolio |
| `portfolio_member` | Add/remove portfolio members |
| `project_member` | Add/remove project members |
| `project_follower` | Add/remove project followers |

## Library Usage

The crate can also be used as a library:

```rust
use asanamcp::{AsanaClient, Resource};

#[tokio::main]
async fn main() -> Result<(), asanamcp::Error> {
    let client = AsanaClient::from_env()?;

    // List workspaces
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
# Run all checks (format, lint, build, docs, test)
make ci

# Run tests
make test

# Generate coverage report
make coverage-html

# Format code
make fmt

# Run clippy
make lint
```

## License

MIT
