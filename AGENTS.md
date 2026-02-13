# Agent Instructions for asanamcp

## Overview

asanamcp is a Model Context Protocol (MCP) server that enables AI assistants to interact with Asana's project management
API. It runs as a STDIO-based binary integrating with Claude Desktop and other MCP clients.

```
MCP Client (Claude) → AsanaServer (tool routing) → AsanaClient (HTTP) → Asana API
```

## MCP Tools

| Tool               | Purpose                                                            |
|--------------------|--------------------------------------------------------------------|
| `asana_workspaces` | List available workspaces                                          |
| `asana_get`        | Fetch any resource type (25+ types) with optional depth/context    |
| `asana_create`     | Create tasks, projects, portfolios, comments, etc.                 |
| `asana_update`     | Modify existing resources                                          |
| `asana_link`       | Manage relationships (task↔project, dependencies, followers, etc.) |
| `asana_search`     | Advanced task search with filters                                  |

## Environment Variables

- `ASANA_TOKEN` (required): Personal access token for Asana API
- `ASANA_DEFAULT_WORKSPACE` (optional): Default workspace GID for operations that require one

## Project Structure

```
src/
├── lib.rs           # Library exports
├── main.rs          # Binary entry point, CLI args
├── schema.rs        # Tool schema generation for --schema flag
├── client.rs        # HTTP client (auth, pagination, error handling)
├── error.rs         # Error types (thiserror)
├── types.rs         # Hybrid response types (typed + raw JSON)
└── server/
    ├── mod.rs       # MCP server + tool implementations
    ├── params.rs    # Tool parameter types (JsonSchema)
    ├── helpers.rs   # Validation, error mapping, field resolution
    ├── fields.rs    # Asana opt_fields constants per resource type
    └── tests.rs     # Server tests
```

**Hybrid types pattern**: `types.rs` uses minimal typed fields (`gid`, `resource_type`) for code dispatch while
preserving all other Asana fields in a flattened `serde_json::Map`. This gives the AI full access to Asana data without
maintaining exhaustive Rust structs.

## Commands

```bash
make ci              # Run before committing (fmt, clippy, build, docs, test)
make test            # Run tests only
make coverage        # Coverage report
make inspect         # Open MCP Inspector web UI
asanamcp --schema              # Dump all tool JSON schemas
asanamcp --schema asana_get    # Dump specific tool schema
```

## Adding Features

### New resource type in asana_get

1. Add variant to `ResourceType` in `params.rs`
2. Add match arm in `asana_get` in `mod.rs`
3. Add opt_fields constant in `fields.rs` if needed
4. Add test in `tests.rs`

### New create/update operation

1. Add variant to `CreateResourceType` or `UpdateResourceType` in `params.rs`
2. Add match arm with parameter validation in `mod.rs`
3. Build request body, POST/PUT to correct endpoint
4. Add test

### New relationship type

1. Add variant to `RelationshipType` in `params.rs`
2. Add match arms for both `Add` and `Remove` actions in `asana_link`
3. Add tests for both actions

## Asana API Reference

The file `docs/asana_oas.yaml` is a local copy of Asana's OpenAPI 3.0 specification (~2.7MB). Use
it to verify which fields the Asana API actually supports for a given endpoint before adding or
removing parameters. Because the file is large, search it with targeted Grep queries rather than
reading it whole. For example:

```bash
# Find the schema definition for a resource
Grep pattern="PortfolioBase:" path="docs/asana_oas.yaml"

# Check if a specific field exists on a resource
Grep pattern="owner" path="docs/asana_oas.yaml" context=5

# Find the request body for an endpoint
Grep pattern="updatePortfolio" path="docs/asana_oas.yaml" context=10
```

## Code Style

- Run `make ci` before committing
- All clippy warnings are errors
- Use `thiserror` for errors, not `anyhow`
- Document public APIs with `///` comments
- Follow existing patterns in the code
