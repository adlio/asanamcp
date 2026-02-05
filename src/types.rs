//! Type definitions for hybrid response handling.
//!
//! These types use a hybrid approach: minimal typed fields for recursion and dispatch,
//! with remaining fields captured as raw JSON for AI consumption.

use serde::{Deserialize, Serialize};
use serde_json::Map;

/// A globally unique identifier for an Asana resource.
pub type Gid = String;

/// Generic wrapper for Asana API single-object responses.
#[derive(Debug, Clone, Deserialize)]
pub struct DataWrapper<T> {
    /// The wrapped data.
    pub data: T,
}

/// Generic wrapper for paginated list API responses.
#[derive(Debug, Clone, Deserialize)]
pub struct ListWrapper<T> {
    /// The list of items.
    pub data: Vec<T>,
    /// Pagination information for fetching more results.
    pub next_page: Option<NextPage>,
}

/// Pagination cursor for fetching additional results.
#[derive(Debug, Clone, Deserialize)]
pub struct NextPage {
    /// The offset token for the next page.
    pub offset: String,
}

/// A minimal wrapper for any Asana resource.
///
/// Provides typed access to `gid` and `resource_type` for recursion and dispatch,
/// while preserving all other fields as raw JSON for AI consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// The unique identifier for the resource.
    pub gid: Gid,

    /// The resource type (e.g., "project", "portfolio", "task").
    #[serde(default)]
    pub resource_type: Option<String>,

    /// All other fields from the API response.
    #[serde(flatten)]
    pub fields: Map<String, serde_json::Value>,
}

/// A portfolio item reference for type dispatch during recursion.
#[derive(Debug, Clone, Deserialize)]
pub struct PortfolioItem {
    /// The unique identifier.
    pub gid: Gid,

    /// The resource type: "project" or "portfolio".
    pub resource_type: String,

    /// The name of the item.
    #[serde(default)]
    pub name: Option<String>,
}

/// A task reference with minimal fields for subtask expansion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRef {
    /// The unique identifier.
    pub gid: Gid,

    /// The task name.
    #[serde(default)]
    pub name: Option<String>,

    /// Whether the task is completed.
    #[serde(default)]
    pub completed: bool,

    /// Number of subtasks (for determining if expansion is needed).
    #[serde(default)]
    pub num_subtasks: u32,
}

/// A user reference with minimal fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRef {
    /// The unique identifier.
    pub gid: Gid,

    /// The user's display name.
    #[serde(default)]
    pub name: Option<String>,
}

/// A favorite item reference for filtering by type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteItem {
    /// The unique identifier.
    pub gid: Gid,

    /// The resource type: "project" or "portfolio".
    pub resource_type: String,

    /// The name of the item.
    #[serde(default)]
    pub name: Option<String>,
}

/// A story/comment for task context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    /// The unique identifier.
    pub gid: Gid,

    /// The story subtype (e.g., "comment_added").
    #[serde(default)]
    pub resource_subtype: Option<String>,

    /// The plain text content.
    #[serde(default)]
    pub text: Option<String>,

    /// The HTML content.
    #[serde(default)]
    pub html_text: Option<String>,

    /// All other fields.
    #[serde(flatten)]
    pub fields: Map<String, serde_json::Value>,
}

impl Story {
    /// Returns true if this story is a user comment.
    pub fn is_comment(&self) -> bool {
        self.resource_subtype.as_deref() == Some("comment_added")
    }
}

/// A task dependency reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    /// The unique identifier.
    pub gid: Gid,

    /// The task name.
    #[serde(default)]
    pub name: Option<String>,

    /// The resource type.
    #[serde(default)]
    pub resource_type: Option<String>,
}

/// A portfolio with its nested items expanded.
#[derive(Debug, Clone, Serialize)]
pub struct PortfolioWithItems {
    /// The portfolio details.
    #[serde(flatten)]
    pub portfolio: Resource,
    /// The items in the portfolio.
    pub items: Vec<PortfolioItemExpanded>,
}

/// An expanded portfolio item with full details.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "resource_type", rename_all = "snake_case")]
pub enum PortfolioItemExpanded {
    /// A project in the portfolio.
    Project(Box<Resource>),
    /// A nested portfolio with its items.
    Portfolio(Box<PortfolioWithItems>),
}

/// A task with its related data expanded.
#[derive(Debug, Clone, Serialize)]
pub struct TaskWithContext {
    /// The task details.
    #[serde(flatten)]
    pub task: Resource,
    /// Subtasks of this task.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subtasks: Vec<TaskRef>,
    /// Tasks this task depends on (blockers).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<TaskDependency>,
    /// Tasks that depend on this task.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dependents: Vec<TaskDependency>,
    /// Comments on this task.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<Story>,
}

/// Response containing user favorites with full details.
#[derive(Debug, Serialize)]
pub struct FavoritesResponse {
    /// The favorited projects.
    pub projects: Vec<Resource>,
    /// The favorited portfolios with their items.
    pub portfolios: Vec<PortfolioWithItems>,
    /// Items that couldn't be fetched.
    pub errors: Vec<FavoriteError>,
}

/// An error fetching a favorite item.
#[derive(Debug, Serialize)]
pub struct FavoriteError {
    /// The item that failed.
    pub item: FavoriteItem,
    /// The error message.
    pub error: String,
}

/// An async job reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// The unique identifier.
    pub gid: Gid,

    /// The job status.
    #[serde(default)]
    pub status: Option<String>,

    /// The new project (if applicable).
    #[serde(default)]
    pub new_project: Option<Resource>,

    /// All other fields.
    #[serde(flatten)]
    pub fields: Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_deserialization() {
        let json = r#"{"gid": "123", "name": "Test", "custom_field": "value"}"#;
        let resource: Resource = serde_json::from_str(json).unwrap();

        assert_eq!(resource.gid, "123");
        assert_eq!(resource.fields.get("name").unwrap(), "Test");
        assert_eq!(resource.fields.get("custom_field").unwrap(), "value");
    }

    #[test]
    fn test_portfolio_item_deserialization() {
        let json = r#"{"gid": "456", "resource_type": "project", "name": "My Project"}"#;
        let item: PortfolioItem = serde_json::from_str(json).unwrap();

        assert_eq!(item.gid, "456");
        assert_eq!(item.resource_type, "project");
        assert_eq!(item.name, Some("My Project".to_string()));
    }

    #[test]
    fn test_story_is_comment() {
        let comment = Story {
            gid: "1".to_string(),
            resource_subtype: Some("comment_added".to_string()),
            text: Some("Hello".to_string()),
            html_text: None,
            fields: Map::new(),
        };
        assert!(comment.is_comment());

        let system = Story {
            gid: "2".to_string(),
            resource_subtype: Some("added_to_project".to_string()),
            text: None,
            html_text: None,
            fields: Map::new(),
        };
        assert!(!system.is_comment());
    }

    #[test]
    fn test_data_wrapper() {
        let json = r#"{"data": {"gid": "789", "name": "Wrapped"}}"#;
        let wrapper: DataWrapper<Resource> = serde_json::from_str(json).unwrap();

        assert_eq!(wrapper.data.gid, "789");
    }

    #[test]
    fn test_list_wrapper_with_pagination() {
        let json = r#"{
            "data": [{"gid": "1"}, {"gid": "2"}],
            "next_page": {"offset": "abc123"}
        }"#;
        let wrapper: ListWrapper<Resource> = serde_json::from_str(json).unwrap();

        assert_eq!(wrapper.data.len(), 2);
        assert_eq!(wrapper.next_page.unwrap().offset, "abc123");
    }
}
