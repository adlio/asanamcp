//! Helper functions for the MCP server.

use crate::Error;
use rmcp::model::{CallToolResult, Content, ErrorCode, ErrorData as McpError};
use serde::Serialize;

use super::params::LinkParams;

/// Convert depth parameter to Option<usize>.
///
/// - Negative values (especially -1) mean unlimited depth (None)
/// - Zero or positive values are converted to Some(n)
pub fn depth_to_option(depth: i32) -> Option<usize> {
    if depth < 0 {
        None
    } else {
        Some(depth as usize)
    }
}

/// Convert an Error to an appropriate MCP error with proper error code.
///
/// Maps error types to MCP error codes:
/// - NotFound -> INVALID_PARAMS (resource doesn't exist)
/// - MissingToken, InvalidToken -> INVALID_PARAMS (auth config issue)
/// - Api, Http, Parse -> INTERNAL_ERROR (server/network issue)
pub fn error_to_mcp(context: &str, error: Error) -> McpError {
    let (code, message) = match &error {
        Error::NotFound(resource) => (
            ErrorCode::INVALID_PARAMS,
            format!("{}: resource not found - {}", context, resource),
        ),
        Error::MissingToken => (
            ErrorCode::INVALID_PARAMS,
            format!("{}: ASANA_TOKEN environment variable not set", context),
        ),
        Error::InvalidToken => (
            ErrorCode::INVALID_PARAMS,
            format!("{}: invalid token format", context),
        ),
        Error::Api { message: msg } => (
            ErrorCode::INTERNAL_ERROR,
            format!("{}: API error - {}", context, msg),
        ),
        Error::Http(e) => (
            ErrorCode::INTERNAL_ERROR,
            format!("{}: HTTP error - {}", context, e),
        ),
        Error::Parse(e) => (
            ErrorCode::INTERNAL_ERROR,
            format!("{}: failed to parse response - {}", context, e),
        ),
    };

    McpError::new(code, message, None)
}

/// Convert any Display error to an MCP error (for non-Error types).
pub fn to_mcp_error(context: &str, error: impl std::fmt::Display) -> McpError {
    McpError::new(
        ErrorCode::INTERNAL_ERROR,
        format!("{}: {}", context, error),
        None,
    )
}

/// Serialize a value to a JSON response.
pub fn json_response<T: Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| to_mcp_error("Failed to serialize response", e))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Create a validation error with the given message.
pub fn validation_error(message: &str) -> McpError {
    McpError::new(ErrorCode::INVALID_PARAMS, message.to_string(), None)
}

/// Create a success response with a message.
pub fn success_response(message: &str) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(
        serde_json::json!({"success": true, "message": message}).to_string(),
    )]))
}

/// Extract item GIDs from link parameters.
///
/// Returns item_gids if present and non-empty, otherwise item_gid as a single-element vec.
/// Returns a validation error if neither is provided.
pub fn get_item_gids(p: &LinkParams) -> Result<Vec<String>, McpError> {
    if let Some(ref gids) = p.item_gids {
        if gids.is_empty() {
            return Err(validation_error("item_gids cannot be empty"));
        }
        Ok(gids.clone())
    } else if let Some(ref gid) = p.item_gid {
        Ok(vec![gid.clone()])
    } else {
        Err(validation_error("item_gid or item_gids is required"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_to_option_negative_is_unlimited() {
        assert_eq!(depth_to_option(-1), None);
        assert_eq!(depth_to_option(-100), None);
    }

    #[test]
    fn test_depth_to_option_zero_is_some_zero() {
        assert_eq!(depth_to_option(0), Some(0));
    }

    #[test]
    fn test_depth_to_option_positive_values() {
        assert_eq!(depth_to_option(1), Some(1));
        assert_eq!(depth_to_option(5), Some(5));
        assert_eq!(depth_to_option(100), Some(100));
    }

    #[test]
    fn test_error_to_mcp_not_found() {
        let error = Error::NotFound("project".to_string());
        let mcp_error = error_to_mcp("Test", error);

        assert_eq!(mcp_error.code, ErrorCode::INVALID_PARAMS);
        assert!(mcp_error.message.contains("not found"));
    }

    #[test]
    fn test_error_to_mcp_missing_token() {
        let error = Error::MissingToken;
        let mcp_error = error_to_mcp("Test", error);

        assert_eq!(mcp_error.code, ErrorCode::INVALID_PARAMS);
        assert!(mcp_error.message.contains("ASANA_TOKEN"));
    }

    #[test]
    fn test_error_to_mcp_api_error() {
        let error = Error::Api {
            message: "Rate limited".to_string(),
        };
        let mcp_error = error_to_mcp("Test", error);

        assert_eq!(mcp_error.code, ErrorCode::INTERNAL_ERROR);
        assert!(mcp_error.message.contains("Rate limited"));
    }

    #[test]
    fn test_error_to_mcp_invalid_token() {
        let error = Error::InvalidToken;
        let mcp_error = error_to_mcp("Test", error);

        assert_eq!(mcp_error.code, ErrorCode::INVALID_PARAMS);
        assert!(mcp_error.message.contains("invalid token"));
    }

    #[test]
    fn test_error_to_mcp_parse_error() {
        // Create a real serde_json::Error by parsing invalid JSON
        let parse_err = serde_json::from_str::<serde_json::Value>("not valid json").unwrap_err();
        let error = Error::Parse(parse_err);
        let mcp_error = error_to_mcp("Test", error);

        assert_eq!(mcp_error.code, ErrorCode::INTERNAL_ERROR);
        assert!(mcp_error.message.contains("parse"));
    }

    #[test]
    fn test_to_mcp_error() {
        let mcp_error = to_mcp_error("Serialization", "unexpected EOF");

        assert_eq!(mcp_error.code, ErrorCode::INTERNAL_ERROR);
        assert!(mcp_error.message.contains("Serialization"));
        assert!(mcp_error.message.contains("unexpected EOF"));
    }

    #[test]
    fn test_validation_error() {
        let error = validation_error("name is required");

        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        assert_eq!(error.message, "name is required");
    }

    #[test]
    fn test_get_item_gids_from_item_gids() {
        let params = LinkParams {
            action: super::super::params::LinkAction::Add,
            relationship: super::super::params::RelationshipType::TaskProject,
            target_gid: "task123".to_string(),
            item_gid: None,
            item_gids: Some(vec!["a".to_string(), "b".to_string()]),
            section_gid: None,
            insert_before: None,
            insert_after: None,
        };

        let result = get_item_gids(&params).unwrap();
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn test_get_item_gids_from_item_gid() {
        let params = LinkParams {
            action: super::super::params::LinkAction::Add,
            relationship: super::super::params::RelationshipType::TaskProject,
            target_gid: "task123".to_string(),
            item_gid: Some("single".to_string()),
            item_gids: None,
            section_gid: None,
            insert_before: None,
            insert_after: None,
        };

        let result = get_item_gids(&params).unwrap();
        assert_eq!(result, vec!["single"]);
    }

    #[test]
    fn test_get_item_gids_empty_array_error() {
        let params = LinkParams {
            action: super::super::params::LinkAction::Add,
            relationship: super::super::params::RelationshipType::TaskProject,
            target_gid: "task123".to_string(),
            item_gid: None,
            item_gids: Some(vec![]),
            section_gid: None,
            insert_before: None,
            insert_after: None,
        };

        let result = get_item_gids(&params);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("cannot be empty"));
    }

    #[test]
    fn test_get_item_gids_neither_provided_error() {
        let params = LinkParams {
            action: super::super::params::LinkAction::Add,
            relationship: super::super::params::RelationshipType::TaskProject,
            target_gid: "task123".to_string(),
            item_gid: None,
            item_gids: None,
            section_gid: None,
            insert_before: None,
            insert_after: None,
        };

        let result = get_item_gids(&params);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("required"));
    }
}
