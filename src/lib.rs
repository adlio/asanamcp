//! Asana MCP Server Library
//!
//! This crate provides an MCP (Model Context Protocol) server for interacting
//! with the Asana API. It can be used as a library or run as a standalone binary.
//!
//! # Features
//!
//! - **Hybrid typed responses**: Minimal typed fields for recursion with raw JSON for AI consumption
//! - **Recursive operations**: Fetch portfolios, tasks, and subtasks with configurable depth
//! - **Full CRUD support**: Create, read, update, and manage relationships between resources
//!
//! # Example
//!
//! ```rust,no_run
//! use asanamcp::{AsanaServer, AsanaClient};
//!
//! # async fn example() -> Result<(), asanamcp::Error> {
//! // Create client directly for low-level API access
//! let client = AsanaClient::from_env()?;
//!
//! // Or create the MCP server for tool-based access
//! let server = AsanaServer::new()?;
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod error;
pub mod server;
pub mod types;

// Re-export main types at crate root
pub use client::AsanaClient;
pub use error::Error;
pub use server::AsanaServer;

// Re-export commonly used types
pub use types::{
    FavoriteItem, FavoritesResponse, Job, PortfolioItem, PortfolioItemExpanded, PortfolioWithItems,
    Resource, Story, TaskDependency, TaskRef, TaskWithContext,
};
