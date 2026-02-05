# Agent Instructions for asanamcp

This document provides guidance for AI coding assistants working in this repository.

## Project Overview

This is a standalone MCP server for the Asana API. It provides:

- **asanamcp** - Both a library crate and binary for MCP server functionality

## Code Quality Requirements

Before committing any changes, ensure all checks pass:

```bash
make ci
```

This runs:
1. `cargo fmt -- --check` - Code formatting
2. `cargo clippy --all-targets --all-features -- -D warnings` - Linting
3. `cargo build --all-targets --all-features` - Compilation
4. `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` - Documentation
5. `cargo nextest run --all-features` - Tests

### Formatting

- Use `cargo fmt` before committing
- The project follows standard Rust formatting conventions

### Linting

- All Clippy warnings are treated as errors (`-D warnings`)
- Fix all warnings before committing
- Use `#[allow(...)]` sparingly and with justification

### Testing

- Tests use `cargo-nextest` for parallel execution
- Write tests for new functionality
- Run `make test` to verify tests pass
- Tests mock HTTP using `wiremock`

## Architecture Guidelines

### Hybrid Response Pattern

The crate uses a hybrid typed/raw response pattern:

```rust
/// Minimal typed fields for recursion + raw JSON for AI consumption
pub struct Resource {
    pub gid: String,
    pub resource_type: Option<String>,
    #[serde(flatten)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}
```

**Benefits:**
- Type-safe recursion via `gid` field
- Type-safe dispatch via `resource_type` field
- Full data preserved in `fields` for AI consumption
- Future-proof: new Asana fields don't break deserialization

### HTTP Client

- Generic `T: DeserializeOwned` signatures work with hybrid types AND raw Value
- Pagination handled automatically by `get_all()`
- `with_base_url()` enables testing with mock servers

### MCP Server

- Uses `rmcp` crate for MCP protocol implementation
- 4 main tools: `asana_get`, `asana_create`, `asana_update`, `asana_link`
- Tool parameters use `JsonSchema` derive for schema generation
- All operations use raw HTTP calls (no SDK dependency)

### Error Handling

- Define specific error types with `thiserror`
- Never use `anyhow` in library code
- Provide context in error messages
- Use `Result<T, Error>` consistently

## Asana API Reference

When implementing Asana API operations:

- API docs: https://developers.asana.com/reference/rest-api-reference
- Base URL: `https://app.asana.com/api/1.0`
- Authentication: Bearer token via `Authorization` header
- GIDs are string identifiers (not integers)
- Pagination uses `offset` tokens, not page numbers

### Key Concepts

- **Workspace** - Top-level container, users belong to workspaces
- **Project** - Container for tasks, has sections
- **Task** - Work item, can have subtasks, custom fields, multiple parents
- **Portfolio** - Collection of projects (can be nested)
- **Section** - Grouping within a project

## Directory Structure

```
asanamcp/
├── Cargo.toml              # Single crate config
├── Makefile                # Build automation
├── README.md               # User documentation
├── AGENTS.md               # This file
├── src/
│   ├── lib.rs              # Library exports
│   ├── main.rs             # Binary entry point
│   ├── client.rs           # HTTP client
│   ├── error.rs            # Error types
│   ├── types.rs            # Hybrid response types
│   └── server.rs           # MCP tools implementation
└── docs/
    └── design.md           # High-level design document
```

## Common Tasks

### Adding a new resource type to asana_get

1. Add variant to `ResourceType` enum in `server.rs`
2. Add match arm in `asana_get` method
3. Add appropriate opt_fields constant
4. Add tests

### Adding a new create operation

1. Add variant to `CreateResourceType` enum
2. Add match arm in `asana_create` method
3. Validate required parameters
4. Build request body with appropriate fields
5. Add tests

### Running tests with coverage

```bash
make coverage-html
```

This generates an HTML report and opens it in your browser.
