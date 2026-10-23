//! Everything needed to call the Garmin Connect personal API.
//!
//! A folder with a `mod.rs` file is a module, just like a single `.rs` file.
//! The `pub mod` lines below declare its submodules (each one a file in this
//! folder) and make them reachable from the rest of the crate.
//!
//! - `auth`: obtains and caches the short-lived bearer token.
//! - `client`: sends authenticated requests and exposes API calls.

pub mod auth;
pub mod client;

// Re-export so other code can write `garmin::GarminClient` instead of
// `garmin::client::GarminClient`.
pub use client::GarminClient;
