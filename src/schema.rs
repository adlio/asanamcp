//! Schema dumping for MCP tool inspection.

use asanamcp::params::*;
use schemars::schema_for;

/// Tool schema info for display.
struct ToolSchema {
    name: &'static str,
    description: &'static str,
    schema: serde_json::Value,
}

/// Dump tool schemas to stdout.
pub fn dump_schemas(filter: Option<&str>) {
    let tools = vec![
        ToolSchema {
            name: "asana_get",
            description: "Get any Asana resource by type and GID",
            schema: serde_json::to_value(schema_for!(GetParams)).unwrap(),
        },
        ToolSchema {
            name: "asana_create",
            description: "Create a new Asana resource",
            schema: serde_json::to_value(schema_for!(CreateParams)).unwrap(),
        },
        ToolSchema {
            name: "asana_update",
            description: "Update an existing Asana resource",
            schema: serde_json::to_value(schema_for!(UpdateParams)).unwrap(),
        },
        ToolSchema {
            name: "asana_link",
            description: "Add or remove relationships between resources",
            schema: serde_json::to_value(schema_for!(LinkParams)).unwrap(),
        },
        ToolSchema {
            name: "asana_task_search",
            description: "Search for tasks with rich filtering",
            schema: serde_json::to_value(schema_for!(TaskSearchParams)).unwrap(),
        },
        ToolSchema {
            name: "asana_resource_search",
            description: "Search for resources by name (projects, templates, users, etc.)",
            schema: serde_json::to_value(schema_for!(ResourceSearchParams)).unwrap(),
        },
        ToolSchema {
            name: "asana_workspaces",
            description: "List all accessible workspaces",
            schema: serde_json::to_value(schema_for!(WorkspacesParams)).unwrap(),
        },
    ];

    let filtered: Vec<_> = match filter {
        Some(f) => {
            let f_lower = f.to_lowercase();
            tools
                .into_iter()
                .filter(|t| {
                    t.name.to_lowercase().contains(&f_lower)
                        || t.name.replace("asana_", "").to_lowercase() == f_lower
                })
                .collect()
        }
        None => tools,
    };

    if filtered.is_empty() {
        eprintln!("No matching tools found for filter: {:?}", filter);
        eprintln!("Available tools: asana_get, asana_create, asana_update, asana_link, asana_task_search, asana_resource_search, asana_workspaces");
        std::process::exit(1);
    }

    for tool in filtered {
        println!("=== {} ===", tool.name);
        println!("Description: {}", tool.description);
        println!();
        println!(
            "{}",
            serde_json::to_string_pretty(&tool.schema).expect("Failed to serialize schema")
        );
        println!();
    }
}
