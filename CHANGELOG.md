# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/adlio/asanamcp/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/adlio/asanamcp/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/adlio/asanamcp/releases/tag/v0.1.0
