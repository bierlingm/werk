#![forbid(unsafe_code)]

//! werk-mcp: MCP server exposing werk's operative gestures as protocol tools.
//!
//! The third interface surface (alongside TUI and CLI). Same gestures, same
//! mutations, same facts — served through a protocol that agents already speak.

mod tools;

use rmcp::{ServiceExt, transport::stdio};

use tools::WerkServer;

/// Run the MCP server on stdio transport.
pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    // Logging must go to stderr — stdout is the MCP transport.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let server = WerkServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
