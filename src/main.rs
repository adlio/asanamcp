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

use asanamcp::AsanaServer;
use rmcp::{transport::stdio, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the Asana MCP server
    let server = AsanaServer::new()?;

    // Create STDIO transport and serve
    let service = server.serve(stdio()).await?;

    // Wait for the service to complete
    service.waiting().await?;

    Ok(())
}
