//! MCP server for LLM clients (OpenSpec change add-mcp-editing): newline-delimited JSON-RPC
//! on stdin / stdout, logs on stderr. `--version` prints the version.

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("ripper-mcp {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    ripper_core::mcp::run_stdio();
}
