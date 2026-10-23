# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`garmin-mcp` is an MCP (Model Context Protocol) server in Rust (edition 2024) that exposes tools for reading Garmin Connect data. Early stage: one tool so far, `list_activities`.

## Commands

```sh
cargo build                 # build
cargo run                   # run server on stdio (waits for an MCP client)
cargo clippy --all-targets  # lint
cargo fmt                   # format
cargo test                  # all tests
cargo test <name>           # single test by name substring
```

## Architecture

- Built on the `rmcp` crate (feature `transport-io`) with a `tokio` multi-thread runtime.
- `src/main.rs` — entry point; loads `Config`, builds `GarminClient`, constructs `GarminServer` and serves it over stdio transport.
- `src/config.rs` — `Config` from env vars / `.env` (`GARMIN_API_KEY` required, `GARMIN_BASE_URL` optional). Custom `Debug` hides the key.
- `src/garmin/auth.rs` — `TokenManager`: `POST /personalapitoken-service/personal-api/personal-api-key/token` with `Authorization: ApiKey <key>` returns a bearer JWT as plain text, valid 24h. Cached behind a tokio `Mutex`, refreshed 5 min before expiry. Uses `SystemTime`, not `Instant` (macOS `Instant` pauses during sleep).
- `src/garmin/client.rs` — `GarminClient`: authenticated GETs with `Authorization: Bearer <token>`; on 401 invalidates the token and retries once. API calls (e.g. `list_activities`) return typed structs via generic `get_json<T>`.
- `src/garmin/types.rs` — response structs (e.g. `Activity`). Deserialize from Garmin camelCase, serialize to tool output as snake_case; sport-specific fields are `Option` and skipped when `None`. Unknown Garmin fields are ignored. Units stay Garmin's (m, s, m/s).
- `src/server.rs` — `GarminServer`, the MCP handler:
  - Tools are async methods in the `#[tool_router] impl GarminServer` block, each annotated `#[tool(description = ...)]`.
  - Tool inputs are structs deriving `Deserialize` + `JsonSchema` (via `rmcp::schemars`), received as `Parameters<T>`. Doc comments on fields become the JSON schema descriptions shown to clients.
  - `#[tool_handler]` on the `ServerHandler` impl wires the router in; `get_info` declares capabilities, server info, and instructions.
- To add a tool: define a request struct, then add a `#[tool]` method to the `#[tool_router]` impl.

## Code comments

The maintainer is new to Rust. Comment code generously so it can be read as a learning resource:

- Every function, struct, enum, and module gets a `///` doc comment saying what it does and why it exists, in plain language.
- Explain Rust-specific concepts where they appear, briefly: ownership/borrowing (`&`, `&mut`, moves), `Result`/`Option` and the `?` operator, `async`/`.await`, traits and `impl` blocks, derive macros, lifetimes, and pattern matching/destructuring (e.g. `Parameters(req)`).
- Explain what macros like `#[tool]`, `#[tool_router]`, `#[tool_handler]`, and `#[tokio::main]` do behind the scenes.
- Inside function bodies, add `//` comments for each non-obvious step, describing what it does and what data flows through it.
- Keep comments accurate when changing code; update or remove stale ones.

## Configuration

Copy `.env.example` to `.env` and set `GARMIN_API_KEY`. `.env` is git-ignored. Real env vars (e.g. from the MCP client config) override `.env`.

## Constraints

- **stdout is the MCP protocol channel.** Never `println!` or write anything else to stdout; log to stderr (`eprintln!`).
- Never log or print the API key or bearer token.
