//! MCP server for Asana API integration.
//!
//! This binary provides a Model Context Protocol (MCP) server that exposes
//! Asana operations as tools for AI assistants.
//!
//! # Usage
//!
//! Set the `ASANA_TOKEN` environment variable and run:
//!
//! ```bash
//! export ASANA_TOKEN="your-personal-access-token"
//! asanamcp
//! ```
//!
//! The server communicates via STDIO using the MCP protocol.
//!
//! # Schema Inspection
//!
//! To dump tool schemas for debugging:
//!
//! ```bash
//! asanamcp --schema
//! asanamcp --schema get    # Show only asana_get schema
//! asanamcp --schema create # Show only asana_create schema
//! ```

use asanamcp::AsanaServer;
use rmcp::{transport::stdio, ServiceExt};
use std::env;

mod schema;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Handle --schema flag
    if args.iter().any(|a| a == "--schema") {
        let filter = args.iter().skip_while(|a| *a != "--schema").nth(1);
        schema::dump_schemas(filter.map(|s| s.as_str()));
        return Ok(());
    }

    // Handle --version flag
    if args.iter().any(|a| a == "--version" || a == "-V") {
        print_version();
        return Ok(());
    }

    // Handle --help flag
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    // Create the Asana MCP server
    let server = AsanaServer::new()?;

    // Create STDIO transport and serve
    let service = server.serve(stdio()).await?;

    // Wait for the service to complete
    service.waiting().await?;

    Ok(())
}

fn print_version() {
    println!(
        "{} {} ({}{} {})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("BUILD_GIT_SHA"),
        env!("BUILD_GIT_DIRTY"),
        env!("BUILD_TIMESTAMP"),
    );
}

fn print_help() {
    println!(
        r#"asanamcp - MCP server for Asana API

USAGE:
    asanamcp [OPTIONS]

OPTIONS:
    --schema [TOOL]  Dump tool schemas (optionally filter by tool name)
    -V, --version    Print version information
    -h, --help       Show this help message

ENVIRONMENT:
    ASANA_TOKEN              Asana personal access token (required)
    ASANA_DEFAULT_WORKSPACE  Default workspace GID (optional)

EXAMPLES:
    asanamcp                 Start MCP server on stdio
    asanamcp --schema        Dump all tool schemas
    asanamcp --schema get    Dump only asana_get schema
"#
    );
}
