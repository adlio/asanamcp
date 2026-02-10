# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `--version` / `-V` flag with build metadata (git SHA, dirty state, build timestamp)

### Fixed

- Obsolete sections in README.md

## [0.3.1] - 2026-02-09

### Added

- Homebrew tap support (`brew install adlio/tap/asanamcp`)
- Prebuilt binaries for 7 platforms via cargo-dist

## [0.3.0] - 2026-02-09

### Added

- `asana_delete` tool for permanently deleting Asana resources (task, project, portfolio, section, tag, comment, status_update, project_brief)

## [0.2.1] - 2026-02-09

### Added

- `html_text` support for comment creation and updates (use instead of `text` for rich formatting)
- `status_update` resource type for fetching a single status update by GID
- `status_updates` resource type for listing status updates on any parent (project, portfolio, or goal)

### Fixed

- Use correct Asana API endpoint for status updates (`GET /status_updates?parent=` instead of non-existent nested endpoints)
- Respect `detail_level` parameter on `task_subtasks` and `task_comments` endpoints (was previously ignored)
- Add `resource_subtype` to default status update fields

## [0.2.0] - 2026-02-09

### Added

- `detail_level` and `extra_fields` parameters for controlling response size

### Fixed

- Extract actual Asana API error messages from 404 responses instead of generic "resource not found"
- Remove redundant "resource not found -" and "API error -" prefixes from MCP error messages

## [0.1.0] - 2026-02-08

### Added

- Initial release of the asanamcp MCP server
- 7 MCP tools: workspaces, get, create, update, link, task_search, resource_search
- Recursive traversal for portfolios and projects
- Asana Personal Access Token authentication
- Library re-export for programmatic use

[Unreleased]: https://github.com/adlio/asanamcp/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/adlio/asanamcp/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/adlio/asanamcp/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/adlio/asanamcp/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/adlio/asanamcp/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/adlio/asanamcp/releases/tag/v0.1.0
