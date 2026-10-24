//! Everything needed to call the Garmin Connect personal API.
//!
//! A folder with a `mod.rs` file is a module, just like a single `.rs` file.
//! The `pub mod` lines below declare its submodules (each one a file in this
//! folder) and make them reachable from the rest of the crate.
//!
//! - `auth`: obtains and caches the short-lived bearer token.
//! - `client`: sends authenticated requests and exposes API calls.
//! - `types`: shared types and unit helpers.
//! - `activity`: structs for the activity list.
//! - `activity_detail`: structs for one activity's full details.
//! - `fitness_age`: structs for Fitness Age.
//! - `sleep`: structs for one night's sleep.

pub mod activity;
pub mod activity_detail;
pub mod auth;
pub mod client;
pub mod fitness_age;
pub mod sleep;
pub mod types;

// Re-export so other code can write `garmin::GarminClient` instead of
// `garmin::client::GarminClient`.
pub use client::GarminClient;
