//! Runtime configuration, read from environment variables.
//!
//! Values can come from two places:
//! 1. Real environment variables, e.g. set in the MCP client's server config.
//! 2. A `.env` file in the current directory (or a parent), loaded with the
//!    `dotenvy` crate. Handy for local development.
//!
//! Real environment variables win: `dotenvy` never overwrites a variable that
//! is already set.

// `fmt` is the standard library's formatting module; we use it to write a
// custom `Debug` implementation below.
use std::{env, fmt};

// `Context` is an `anyhow` trait that adds `.context("...")` to `Result` and
// `Option`, attaching a human-readable message to an error.
use anyhow::Context;

/// Base URL used when `GARMIN_BASE_URL` is not set.
// `const` is a compile-time constant. `&str` is a borrowed string slice; a
// string literal like this one lives for the whole program (`'static`).
const DEFAULT_BASE_URL: &str = "https://personal.connectapi.garmin.com";

/// Settings the server needs to talk to the Garmin API.
// No `#[derive(Debug)]` here on purpose: the derived version would print the
// API key. We write our own `Debug` below that hides it.
#[derive(Clone)]
pub struct Config {
    /// Root of all API URLs, without a trailing slash.
    pub base_url: String,
    /// Personal API key, sent as `Authorization: ApiKey <key>` to get a
    /// bearer token. Secret: never log it.
    pub api_key: String,
}

impl Config {
    /// Loads the configuration from the environment (and `.env`, if present).
    ///
    /// Returns an error if `GARMIN_API_KEY` is missing or empty, so the
    /// server stops at startup instead of failing on the first tool call.
    pub fn from_env() -> anyhow::Result<Self> {
        // Load `.env` into the process environment. It returns a `Result`
        // because the file may not exist; that is fine (variables may be set
        // another way), so we ignore the result with `.ok()`.
        dotenvy::dotenv().ok();

        // `env::var` returns `Result<String, VarError>`. `.ok()` turns it
        // into an `Option<String>` (`None` if unset), and `.filter(...)`
        // also treats an empty value as `None`. `.context(...)` converts the
        // `None` into an error with this message, and `?` returns it early.
        let api_key = env::var("GARMIN_API_KEY")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .context("GARMIN_API_KEY is not set (put it in .env or the MCP client config)")?;

        // Same pattern, but a missing value falls back to the default.
        // `|v| ...` is a *closure* (an inline anonymous function).
        // `.unwrap_or_else(...)` calls the closure only when the value is
        // `None`; `.to_string()` turns the `&str` constant into an owned
        // `String`.
        let base_url = env::var("GARMIN_BASE_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        Ok(Self {
            // Strip a trailing `/` so we can always build URLs as
            // `format!("{base_url}{path}")` with `path` starting with `/`.
            base_url: base_url.trim_end_matches('/').to_string(),
            // Field init shorthand: `api_key` means `api_key: api_key`.
            api_key,
        })
    }
}

/// Custom `Debug` output that hides the API key.
///
/// Implementing a trait by hand (instead of `#[derive]`) lets us control
/// exactly what it does. `{:?}` on a `Config` now prints the key as `***`.
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `debug_struct` is a helper that prints `Config { field: value, .. }`.
        f.debug_struct("Config")
            .field("base_url", &self.base_url)
            .field("api_key", &"***")
            .finish()
    }
}
