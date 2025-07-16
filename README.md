# cargo-mcp

MCP server for cargo/crates.io/rustdoc

## Installation

```console
$ cargo install --git https://github.com/camshaft/cargo-mcp
```

### VSCode config

In `.vscode/mcp.json`:

```json
{
  "servers": {
    "camshaft/cargo-mcp": {
      "type": "stdio",
      "command": "cargo-mcp"
    }
  }
}
```

### Cline config

```json
{
  "mcpServers": {
    "camshaft/cargo-mcp": {
      "command": "cargo-mcp",
      "args": [],
      "disabled": false,
      "autoApprove": []
    }
  }
}
```
