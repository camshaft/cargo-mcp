use super::*;
use std::sync::Arc;
use tokio::test;

//= docs/design/technical-spec.md#general
//= type=test
//# The server MUST implement the Model Context Protocol (MCP) specification.
#[test]
async fn test_mcp_protocol_implementation() {
    let ctx = Arc::new(TestContext::new().unwrap());
    let test = Test::start(ctx).await.unwrap();

    // Just verify we can connect and disconnect cleanly
    test.cancel().await.unwrap();
}
