# Agent Instructions for asanamcp

Guidance for AI coding assistants working in this repository.

## Quick Reference

```bash
make ci              # Run before committing (fmt, clippy, build, docs, test)
make test            # Run tests only
make coverage        # Coverage report
cargo nextest run    # Tests with parallel execution
```

## Project Structure

```
src/
├── lib.rs           # Library exports
├── main.rs          # Binary entry point
├── client.rs        # HTTP client (generic, paginated)
├── error.rs         # Error types (thiserror)
├── types.rs         # Hybrid response types
└── server/
    ├── mod.rs       # MCP server + tool implementations
    ├── params.rs    # Tool parameter types (JsonSchema)
    ├── helpers.rs   # Helper functions
    ├── fields.rs    # Asana opt_fields constants
    └── tests.rs     # All server tests
```

## Known Pitfalls

### rmcp Crate

**Version matters.** The crate changed significantly between versions. We use `rmcp = "0.11"`.

**schemars version conflict.** rmcp re-exports schemars 1.x. If you add schemars to Cargo.toml, it must be version 1.x to match:

```toml
# CORRECT
schemars = "1.0"

# WRONG - will cause derive macro conflicts
schemars = "0.8"
```

**Feature names.** The transport feature is `transport-io`, not `transport-stdio`:

```toml
# CORRECT
rmcp = { version = "0.11", features = ["server", "transport-io"] }

# WRONG
rmcp = { version = "0.11", features = ["server", "transport-stdio"] }
```

**Tool handler imports.** The rmcp macros require specific imports:

```rust
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ErrorData as McpError, ...};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
```

### Rust Patterns

**Async recursion.** Rust doesn't support async recursion directly. Use `Box::pin()` or restructure as iterative with a stack/queue. The `get_portfolio_recursive` method uses an iterative approach with a `VecDeque`.

**serde flatten for hybrid types.** The `#[serde(flatten)]` attribute captures unknown fields into a Map:

```rust
pub struct Resource {
    pub gid: String,
    #[serde(flatten)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}
```

**skip_serializing_if.** Empty vectors with this attribute won't appear in JSON output:

```rust
#[serde(skip_serializing_if = "Vec::is_empty")]
pub subtasks: Vec<TaskRef>,  // Omitted when empty, not serialized as []
```

### Testing with wiremock

**NoOffset matcher.** Pagination tests need a custom matcher to distinguish first-page requests:

```rust
struct NoOffset;
impl Match for NoOffset {
    fn matches(&self, request: &Request) -> bool {
        !request.url.query().map_or(false, |q| q.contains("offset="))
    }
}
```

**Mount order matters.** More specific mocks should be mounted after general ones, but wiremock matches in mount order. Use `.expect(1)` for explicit call counts.

**API endpoint paths.** Always verify the actual Asana API path. Common mistakes:
- Tags: `/tags` not `/workspaces/{gid}/tags`
- Status updates: `/status_updates` not `/projects/{gid}/status_updates` for creation

### Asana API

**GIDs are strings.** Never parse as integers.

**Pagination uses offset tokens.** Not page numbers. The `next_page.offset` field contains an opaque string.

**opt_fields reduce response size.** Always specify them. See `fields.rs` for standard field sets.

**Status types are semantic.** Values like `on_track`, `at_risk`, `off_track` - not colors or numbers.

## Adding Features

### New resource type in asana_get

1. Add variant to `ResourceType` in `params.rs`
2. Add `#[serde(alias = "...")]` for backward compatibility if renaming
3. Add match arm in `asana_get` in `mod.rs`
4. Add opt_fields constant in `fields.rs` if needed
5. Add test in `tests.rs`

### New create/update operation

1. Add variant to `CreateResourceType` or `UpdateResourceType`
2. Add match arm with parameter validation
3. Build request body, POST/PUT to correct endpoint
4. Add test

### New relationship type

1. Add variant to `RelationshipType` in `params.rs`
2. Add match arms for both `Add` and `Remove` actions in `asana_link`
3. Add tests for both actions

## Code Style

- `cargo fmt` before committing
- All clippy warnings are errors
- Use `thiserror` for error types, never `anyhow` in library code
- Prefer `Option<T>` over sentinel values
- Document public APIs with `///` comments
