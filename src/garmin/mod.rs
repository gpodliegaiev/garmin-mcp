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
//! - `body_battery`: structs for the daily Body Battery report.
//! - `daily_summary`: structs for daily wellness summaries.
//! - `fitness_age`: structs for Fitness Age.
//! - `heart_rate_zones`: structs for the user's heart rate zones.
//! - `hrv`: structs for heart rate variability.
//! - `sleep`: structs for one night's sleep.
//! - `weight`: structs for weight, body composition and the weight goal.

pub mod activity;
pub mod activity_detail;
pub mod auth;
pub mod body_battery;
pub mod client;
pub mod daily_summary;
pub mod fitness_age;
pub mod heart_rate_zones;
pub mod hrv;
pub mod sleep;
pub mod types;
pub mod weight;

// Re-export so other code can write `garmin::GarminClient` instead of
// `garmin::client::GarminClient`.
pub use client::GarminClient;
