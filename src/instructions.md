# Cargo MCP Server

A Model Context Protocol (MCP) server that provides programmatic access to Cargo workspace information. This server enables AI agents to understand and interact with Rust project structures through a standardized interface.

## When to Use

- Analyzing workspace structure and crate relationships
- Understanding project dependencies and organization
- Mapping crate locations and directory structures
- Gathering metadata about Rust projects
- Automating workspace-related tasks

## Key Features

- **Workspace Information**: Access comprehensive details about all crates in a Cargo workspace
- **Directory Mapping**: Get file system locations for workspace crates
- **Crate Details**: Retrieve specific information about individual crates
- **Structured Data**: All information is provided in machine-readable JSON format
- **Resource-based API**: Access data through well-defined resource URIs

## Usage Notes

- All resources are accessed through the `cargo://` URI scheme
- Responses are provided in JSON format for easy parsing
- Resource-specific documentation is available in individual resource descriptions
- The server automatically detects and provides information about the current workspace

For detailed information about specific resources and their response formats, refer to the resource documentation provided in the resource descriptions.
