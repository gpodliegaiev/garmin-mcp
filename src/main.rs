//! Entry point of the `garmin-mcp` binary.
//!
//! This program is an MCP (Model Context Protocol) server. An MCP client
//! (for example Claude Code or Claude Desktop) launches it as a child process
//! and talks to it by writing JSON messages to its stdin and reading replies
//! from its stdout. This file only wires things up; the actual tools live in
//! `server.rs`.

// `mod server;` tells the compiler that a module named `server` exists and
// that its code is in `src/server.rs`. Without this line that file would be
// ignored entirely.
mod server;

// `use` brings names into scope so we can write `stdio()` instead of
// `rmcp::transport::stdio()`.
// - `ServiceExt` is a *trait*: a set of methods that can be added to other
//   types. Importing it is what makes the `.serve(...)` method available on
//   our `GarminServer` below. In Rust, trait methods can only be called when
//   the trait is in scope.
// - `stdio` builds a "transport": the channel the MCP messages travel over
//   (here, the process's standard input and output).
use rmcp::{ServiceExt, transport::stdio};

// `crate::` means "starting from the root of this project". This imports the
// `GarminServer` struct defined in `src/server.rs`.
use crate::server::GarminServer;

/// Starts the MCP server and keeps it running until the client disconnects.
///
/// `#[tokio::main]` is an *attribute macro*. Rust's `main` function cannot be
/// `async` on its own, because something has to drive async code. The macro
/// rewrites this function into a normal `fn main()` that creates a Tokio
/// runtime (a scheduler for async tasks) and runs the async body on it.
///
/// The return type `anyhow::Result<()>` means: "either success with no value
/// (`()`, the empty type) or an error". `anyhow::Result` can hold any kind of
/// error, which is convenient for application code. If `main` returns an
/// error, Rust prints it and exits with a non-zero status code.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // stdout carries the MCP protocol; log to stderr only.
    // `eprintln!` (note the `e`) prints to stderr. Using `println!` here would
    // write garbage into the protocol stream and break the client.
    // The `!` means `eprintln!` is a macro, not a regular function.
    eprintln!("garmin-mcp starting on stdio");

    // Create our server and start serving it over stdin/stdout.
    // - `GarminServer::new()` builds the server value (see `server.rs`).
    // - `.serve(stdio())` performs the MCP handshake with the client and
    //   returns a handle to the running service.
    // - `.await` pauses here until that async work finishes, without blocking
    //   the thread.
    // - `?` is error propagation: if the result is an error, return it from
    //   `main` immediately; if it is `Ok(value)`, unwrap it to `value`.
    let service = GarminServer::new().serve(stdio()).await?;

    // Wait until the client closes the connection (or an error occurs).
    // While we wait, the service handles incoming requests in the background.
    service.waiting().await?;

    // Everything went fine. `Ok(())` is the success value with no data.
    // The last expression in a function (without a trailing `;`) is its
    // return value, so no `return` keyword is needed.
    Ok(())
}
